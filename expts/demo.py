#!/usr/bin/env python3
"""
expts/demo.py

End-to-end demo (verbose), OS-agnostic (Linux / macOS / Windows):

  - builds & starts ml-service, chain, api-gateway, prometheus
  - waits for health endpoints
  - creates dummy model artefacts inside ml-service
  - registers multiple models with varied wm_profiles via api-gateway /models/register
  - waits until 10 blocks are produced and reports timing stats
  - shows a few log lines
  - tears the stack down (docker compose down -v) unless KEEP_CONTAINERS=1

Usage:
  python expts/demo.py

Environment:
  KEEP_CONTAINERS=1  -> leave containers running after demo
"""

from __future__ import annotations

import json
import os
import re
import subprocess
import sys
import time
from pathlib import Path
from typing import List, Sequence
from urllib.error import URLError, HTTPError
from urllib.request import Request, urlopen


# -----------------------------------------------------------------------------
# Colours (no emojis) – fall back to plain text on non-TTY
# -----------------------------------------------------------------------------
def _colour_codes():
    if sys.stdout.isatty():
        return {
            "BOLD": "\033[1m",
            "DIM": "\033[2m",
            "GREEN": "\033[32m",
            "YELLOW": "\033[33m",
            "BLUE": "\033[34m",
            "RED": "\033[31m",
            "RESET": "\033[0m",
        }
    else:
        # No colours if output is not a TTY
        return {k: "" for k in ["BOLD", "DIM", "GREEN", "YELLOW", "BLUE", "RED", "RESET"]}


C = _colour_codes()


def log_kv(label: str, *values: str) -> None:
    print(f"  {C['DIM']}{label}:{C['RESET']} {' '.join(str(v) for v in values)}")


def log_section(*msg: str) -> None:
    title = " ".join(str(m) for m in msg)
    print()
    print("###############################################################################")
    print(f"# {title}")
    print("###############################################################################")
    print()


def note(msg: str) -> None:
    print(f"  {C['BLUE']}{msg}{C['RESET']}")


def warn(msg: str) -> None:
    print(f"{C['YELLOW']}WARN{C['RESET']}: {msg}")


def err(msg: str) -> None:
    print(f"{C['RED']}ERROR{C['RESET']}: {msg}", file=sys.stderr)


# -----------------------------------------------------------------------------
# Subprocess helpers
# -----------------------------------------------------------------------------
def run(
    cmd: Sequence[str],
    *,
    cwd: Path | None = None,
    check: bool = True,
    capture_output: bool = False,
) -> subprocess.CompletedProcess:
    """Thin wrapper around subprocess.run with sane defaults."""
    cmd_str = " ".join(cmd)
    if cwd:
        note(f"Running: {cmd_str}  (cwd={cwd})")
    else:
        note(f"Running: {cmd_str}")
    return subprocess.run(
        list(cmd),
        cwd=str(cwd) if cwd else None,
        check=check,
        text=True,
        capture_output=capture_output,
    )


def find_compose_command() -> List[str]:
    """
    Decide whether to use 'docker compose' or 'docker-compose'.

    Returns:
        list[str] representing the command prefix, e.g. ['docker', 'compose'].
    """
    # Try `docker compose`
    try:
        result = subprocess.run(
            ["docker", "compose", "version"],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            check=False,
        )
        if result.returncode == 0:
            return ["docker", "compose"]
    except FileNotFoundError:
        pass

    # Try `docker-compose`
    try:
        result = subprocess.run(
            ["docker-compose", "version"],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            check=False,
        )
        if result.returncode == 0:
            return ["docker-compose"]
    except FileNotFoundError:
        pass

    raise SystemExit("Error: neither 'docker compose' nor 'docker-compose' found in PATH.")


# -----------------------------------------------------------------------------
# HTTP helpers
# -----------------------------------------------------------------------------
def wait_for_http(url: str, max_tries: int = 30, delay: float = 2.0) -> None:
    print(f"Waiting for {url} ...")
    for i in range(1, max_tries + 1):
        try:
            req = Request(url, method="GET")
            with urlopen(req, timeout=5) as resp:
                if 200 <= resp.status < 300:
                    print(f"  {C['GREEN']}OK{C['RESET']} ({url})")
                    return
        except (URLError, HTTPError):
            pass
        time.sleep(delay)

    err(f"timeout waiting for {url}")
    raise SystemExit(1)


def http_post_json(url: str, payload: dict) -> None:
    data = json.dumps(payload).encode("utf-8")
    req = Request(url, data=data, method="POST")
    req.add_header("Content-Type", "application/json")

    # Show the payload like the original script
    print(f"{C['DIM']}payload:{C['RESET']}")
    for line in json.dumps(payload, indent=2).splitlines():
        print(f"    {line}")

    try:
        with urlopen(req, timeout=10) as resp:
            body = resp.read()
            print(f"  {C['GREEN']}HTTP {resp.status}{C['RESET']}")
            if body:
                try:
                    parsed = json.loads(body.decode("utf-8"))
                    print("  response:", json.dumps(parsed, indent=2))
                except Exception:
                    print("  raw response:", body.decode("utf-8", errors="replace"))
    except HTTPError as e:
        # Mirror curl || true behaviour: don't crash the whole demo
        warn(f"POST {url} returned HTTP {e.code}")
        try:
            body = e.read()
            if body:
                print("  error body:", body.decode("utf-8", errors="replace"))
        except Exception:
            pass
    except URLError as e:
        warn(f"POST {url} failed: {e}")


# -----------------------------------------------------------------------------
# Chain / logs helpers
# -----------------------------------------------------------------------------
HEIGHT_RE = re.compile(r"proposed block height=(\d+)")


def wait_for_blocks(target_blocks: int = 10, poll_delay: float = 2.0, timeout_secs: int = 240) -> None:
    """
    Watch 'docker logs chain' for block production and compute timing stats.

    Height is zero-based, so height 9 == 10 blocks.
    """
    target_height = target_blocks - 1
    start = time.time()
    last_height = -1

    log_section(f"Waiting for {target_blocks} blocks and benchmarking")
    print(f"Watching chain logs for block production (target: {target_blocks} blocks)...")

    while (time.time() - start) < timeout_secs:
        try:
            proc = subprocess.run(
                ["docker", "logs", "chain", "--tail=400"],
                text=True,
                capture_output=True,
                check=False,
            )
        except FileNotFoundError:
            err("docker not found while reading chain logs.")
            raise SystemExit(1)

        max_height = -1
        for line in proc.stdout.splitlines():
            m = HEIGHT_RE.search(line)
            if m:
                h = int(m.group(1))
                if h > max_height:
                    max_height = h

        if max_height > last_height:
            elapsed = int(time.time() - start)
            note(f"latest height={max_height} (elapsed {elapsed}s)")
            last_height = max_height

        if max_height >= target_height:
            duration = time.time() - start
            avg = duration / target_blocks if target_blocks > 0 else float("nan")
            print(
                f"Produced {target_blocks} blocks in {duration:.1f}s "
                f"(~{avg:.2f}s per block)"
            )
            return

        time.sleep(poll_delay)

    err(f"Did not see {target_blocks} blocks within {timeout_secs}s")
    raise SystemExit(1)


# -----------------------------------------------------------------------------
# Model helpers (run code inside ml-service container)
# -----------------------------------------------------------------------------
def create_model(aid_hex: str) -> None:
    script = f"""
import os
from pathlib import Path
import torch

model_root = Path(os.environ.get("ML_SERVICE_MODEL_ROOT", "/app/ml_service/models"))
model_root.mkdir(parents=True, exist_ok=True)
aid_hex = "{aid_hex}"
model_path = model_root / (aid_hex + ".pt")
torch.save({{"demo": aid_hex}}, model_path)
print("model created:", model_path)
"""
    run(["docker", "exec", "ml-service", "python", "-c", script], check=True)


def register_model(
    owner_hex: str,
    aid_hex: str,
    evidence_hex: str,
    tau_input: float,
    tau_feat: float,
    logit_low: float,
    logit_high: float,
) -> None:
    payload = {
        "owner_account_hex": owner_hex,
        "aid_hex": aid_hex,
        "scheme_id": "multi_factor_v1",
        "evidence_hash_hex": evidence_hex,
        "wm_profile": {
            "tau_input": tau_input,
            "tau_feat": tau_feat,
            "logit_band_low": logit_low,
            "logit_band_high": logit_high,
        },
    }
    http_post_json("http://localhost:8081/models/register", payload)
    print()


# -----------------------------------------------------------------------------
# Main flow
# -----------------------------------------------------------------------------
def main() -> None:
    start_time = time.time()

    # Resolve paths relative to this file: ROOT/expts/demo.py -> ROOT
    root_dir = Path(__file__).resolve().parent.parent
    compose_file = root_dir / "deploy" / "docker-compose.yml"
    deploy_dir = root_dir / "deploy"

    if not compose_file.exists():
        raise SystemExit(f"docker-compose.yml not found at {compose_file}")

    compose_cmd = find_compose_command()
    keep_containers = os.environ.get("KEEP_CONTAINERS", "0") == "1"

    # -------------------------------------------------------------------------
    # 1. Build and start the devnet stack
    # -------------------------------------------------------------------------
    log_section("Building and starting devnet stack (ml-service, chain, api-gateway, prometheus)")
    run(
        compose_cmd + ["-f", str(compose_file), "up", "-d", "--build"],
        cwd=deploy_dir,
        check=True,
    )
    log_kv("compose", str(compose_file))
    note("Containers are building; this can take a minute on first run.")

    try:
        # ---------------------------------------------------------------------
        # 2. Wait for health / metrics endpoints
        # ---------------------------------------------------------------------
        wait_for_http("http://localhost:8080/health")
        log_kv("ml-service", "healthy")

        wait_for_http("http://localhost:8081/health")
        log_kv("api-gateway", "healthy")

        wait_for_http("http://localhost:9898/metrics")
        log_kv("chain metrics", "reachable")

        wait_for_http("http://localhost:9899/metrics")
        log_kv("api-gateway metrics", "reachable")

        # ---------------------------------------------------------------------
        # 3. Create dummy model artefacts inside ml-service
        # ---------------------------------------------------------------------
        log_section("Creating dummy model artefacts inside ml-service")

        owner_hex = "a" * 64

        wm_tau_inputs = [0.0, 0.2, -0.1, 0.4, -0.3, 0.6, 0.1, -0.2, 0.5, 0.8]
        wm_tau_feats = [1.0, 0.9, 1.1, 1.2, 0.85, 1.05, 0.95, 1.15, 1.25, 0.75]
        wm_logit_lows = [-1.0, -0.8, -1.2, -0.6, -1.4, -0.5, -1.1, -0.7, -1.3, -0.9]
        wm_logit_highs = [1.0, 1.2, 0.9, 1.3, 0.8, 1.4, 0.95, 1.25, 1.5, 1.1]

        for i in range(10):
            log_section(f"Preparing model {i+1}/10")
            aid_hex = f"{i+1:064x}"
            evidence_hex = f"{i+1000:064x}"

            log_kv("aid_hex", aid_hex)
            log_kv("evidence_hex", evidence_hex)
            log_kv(
                "wm_profile",
                f"tau_input={wm_tau_inputs[i]}",
                f"tau_feat={wm_tau_feats[i]}",
                f"band=[{wm_logit_lows[i]}, {wm_logit_highs[i]}]",
            )

            create_model(aid_hex)
            register_model(
                owner_hex,
                aid_hex,
                evidence_hex,
                wm_tau_inputs[i],
                wm_tau_feats[i],
                wm_logit_lows[i],
                wm_logit_highs[i],
            )

        # ---------------------------------------------------------------------
        # 4. Wait for a few blocks and report timings
        # ---------------------------------------------------------------------
        wait_for_blocks(target_blocks=10, poll_delay=2.0, timeout_secs=240)

        # ---------------------------------------------------------------------
        # 5. Show some logs from api-gateway and chain
        # ---------------------------------------------------------------------
        log_section("Recent logs from api-gateway")
        run(["docker", "logs", "--tail=20", "api-gateway"], check=False)

        log_section("Recent logs from chain")
        run(["docker", "logs", "--tail=20", "chain"], check=False)

        log_section("Demo complete")

        total_secs = int(time.time() - start_time)
        print("You can now:")
        print("  - visit Prometheus at http://localhost:9090")
        print("  - manually hit http://localhost:8080/health (ml-service)")
        print("  - manually hit http://localhost:8081/health (api-gateway)")
        print("  - inspect metrics: http://localhost:9898/metrics (chain), http://localhost:9899/metrics (api-gateway)")
        print(f"  - total demo time: {total_secs}s")
        print()
        print("To keep containers running after the script, use:")
        print("  KEEP_CONTAINERS=1 python expts/demo.py")

    finally:
        if keep_containers:
            log_section("KEEP_CONTAINERS=1 so devnet stack is left running")
        else:
            log_section("Tearing down devnet stack (docker compose down -v)")
            try:
                run(
                    compose_cmd + ["-f", str(compose_file), "down", "-v"],
                    cwd=deploy_dir,
                    check=False,
                )
            except Exception as e:
                warn(f"Failed to tear down stack cleanly: {e}")


if __name__ == "__main__":
    main()
