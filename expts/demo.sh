#!/usr/bin/env bash
#
# expts/demo.sh
#
# End-to-end demo (verbose):
#   - builds & starts ml-service, chain, api-gateway, prometheus
#   - waits for health endpoints
#   - creates dummy model artefacts inside ml-service
#   - registers multiple models with varied wm_profiles via api-gateway /models/register
#   - waits until 10 blocks are produced and reports timing stats
#   - shows a few log lines
#   - tears the stack down (docker compose down -v)
#
# Usage:
#   bash expts/demo.sh
#
# Set KEEP_CONTAINERS=1 to leave the stack running after the demo:
#   KEEP_CONTAINERS=1 bash expts/demo.sh

set -euo pipefail
SECONDS=0

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
COMPOSE_FILE="$ROOT_DIR/deploy/docker-compose.yml"

# Which compose command to use (docker compose vs docker-compose)
if docker compose version &>/dev/null; then
  COMPOSE="docker compose"
elif docker-compose version &>/dev/null; then
  COMPOSE="docker-compose"
else
  echo "Error: neither 'docker compose' nor 'docker-compose' found in PATH" >&2
  exit 1
fi

KEEP_CONTAINERS="${KEEP_CONTAINERS:-0}"

# Colours (no emojis)
BOLD="$(printf '\033[1m')"
DIM="$(printf '\033[2m')"
GREEN="$(printf '\033[32m')"
YELLOW="$(printf '\033[33m')"
BLUE="$(printf '\033[34m')"
RED="$(printf '\033[31m')"
RESET="$(printf '\033[0m')"

log_kv() {
  local label="$1"; shift
  echo "  ${DIM}${label}:${RESET} $*"
}

###############################################################################
# Helpers
###############################################################################

wait_for_http() {
  local url="$1"
  local max_tries="${2:-30}"
  local delay="${3:-2}"

  echo "Waiting for ${url} ..."
  for ((i=1; i<=max_tries; i++)); do
    if curl -fsS "$url" >/dev/null 2>&1; then
      echo "  ${GREEN}OK${RESET} (${url})"
      return 0
    fi
    sleep "$delay"
  done

  echo "ERROR: timeout waiting for ${url}" >&2
  return 1
}

log_section() {
  echo
  echo "###############################################################################"
  echo "# $*"
  echo "###############################################################################"
  echo
}

note() {
  local msg="$1"
  echo "  ${BLUE}${msg}${RESET}"
}

wait_for_blocks() {
  local target_blocks="${1:-10}"
  local poll_delay="${2:-2}"
  local timeout_secs="${3:-180}"

  # Height is zero-based, so height 9 == 10 blocks.
  local target_height=$((target_blocks - 1))
  local start_secs=$SECONDS
  local last_height=-1

  echo "Watching chain logs for block production (target: ${target_blocks} blocks)..."

  while (( (SECONDS - start_secs) < timeout_secs )); do
    local max_height
    max_height=$(
      docker logs chain --tail=400 2>/dev/null | \
        awk 'BEGIN{max=-1} /proposed block height=/ {sub(/.*height=/,""); sub(/ .*/,""); if($1+0>max) max=$1+0} END{print max}'
    )

    if (( max_height > last_height )); then
      local elapsed=$((SECONDS - start_secs))
      note "latest height=${max_height} (elapsed ${elapsed}s)"
      last_height=$max_height
    fi

    if (( max_height >= target_height )); then
      local duration=$((SECONDS - start_secs))
      local avg
      avg=$(awk -v d="$duration" -v n="$target_blocks" 'BEGIN { if (n==0) {print "n/a"} else {printf "%.2f", d/n} }')

      echo "Produced ${target_blocks} blocks in ${duration}s (~${avg}s per block)"
      return 0
    fi

    sleep "$poll_delay"
  done

  echo "ERROR: Did not see ${target_blocks} blocks within ${timeout_secs}s" >&2
  return 1
}

cleanup() {
  if [[ "$KEEP_CONTAINERS" != "1" ]]; then
    log_section "Tearing down devnet stack (docker compose down -v)"
    (cd "$ROOT_DIR/deploy" && $COMPOSE down -v) || true
  else
    log_section "KEEP_CONTAINERS=1 so devnet stack is left running"
  fi
}
trap cleanup EXIT

###############################################################################
# 1. Build and start the devnet stack
###############################################################################

log_section "Building and starting devnet stack (ml-service, chain, api-gateway, prometheus)"

cd "$ROOT_DIR/deploy"
$COMPOSE -f "$COMPOSE_FILE" up -d --build
log_kv "compose" "$COMPOSE_FILE"
note "Containers are building; this can take a minute on first run."

###############################################################################
# 2. Wait for health / metrics endpoints
###############################################################################

# ml-service: /health
wait_for_http "http://localhost:8080/health"
log_kv "ml-service" "healthy"

# api-gateway: /health
wait_for_http "http://localhost:8081/health"
log_kv "api-gateway" "healthy"

# chain metrics
wait_for_http "http://localhost:9898/metrics"
log_kv "chain metrics" "reachable"

# api-gateway metrics (mapped to host 9899)
wait_for_http "http://localhost:9899/metrics"
log_kv "api-gateway metrics" "reachable"

###############################################################################
# 3. Create dummy model artefacts inside ml-service
###############################################################################

log_section "Creating dummy model artefacts inside ml-service"

OWNER_HEX="aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"

# Ten wm profiles (one per block target)
declare -a WM_TAU_INPUTS=("0.0" "0.2" "-0.1" "0.4" "-0.3" "0.6" "0.1" "-0.2" "0.5" "0.8")
declare -a WM_TAU_FEATS=("1.0" "0.9" "1.1" "1.2" "0.85" "1.05" "0.95" "1.15" "1.25" "0.75")
declare -a WM_LOGIT_LOWS=("-1.0" "-0.8" "-1.2" "-0.6" "-1.4" "-0.5" "-1.1" "-0.7" "-1.3" "-0.9")
declare -a WM_LOGIT_HIGHS=("1.0" "1.2" "0.9" "1.3" "0.8" "1.4" "0.95" "1.25" "1.5" "1.1")

create_model() {
  local aid_hex="$1"
  docker exec ml-service python - <<PY
import os
from pathlib import Path
import torch

model_root = Path(os.environ.get("ML_SERVICE_MODEL_ROOT", "/app/ml_service/models"))
model_root.mkdir(parents=True, exist_ok=True)
aid_hex = "$aid_hex"
model_path = model_root / f"{aid_hex}.pt"
torch.save({"demo": aid_hex}, model_path)
print(f"model created: {model_path}")
PY
}

register_model() {
  local aid_hex="$1"
  local evidence_hex="$2"
  local tau_input="$3"
  local tau_feat="$4"
  local logit_low="$5"
  local logit_high="$6"

  local payload
  payload=$(cat <<JSON
{
  "owner_account_hex": "${OWNER_HEX}",
  "aid_hex": "${aid_hex}",
  "scheme_id": "multi_factor_v1",
  "evidence_hash_hex": "${evidence_hex}",
  "wm_profile": {
    "tau_input": ${tau_input},
    "tau_feat": ${tau_feat},
    "logit_band_low": ${logit_low},
    "logit_band_high": ${logit_high}
  }
}
JSON
)

  echo "${DIM}payload:${RESET}"
  echo "$payload" | sed 's/^/    /'

  curl -s -i -X POST "http://localhost:8081/models/register" \
    -H "Content-Type: application/json" \
    -d "$payload" \
    || true

  echo
}

for i in $(seq 0 9); do
  log_section "Preparing model $((i+1))/10"
  AID_HEX=$(printf "%064x" "$((i+1))")
  EVIDENCE_HEX=$(printf "%064x" "$((i+1000))")

  log_kv "aid_hex" "$AID_HEX"
  log_kv "evidence_hex" "$EVIDENCE_HEX"
  log_kv "wm_profile" "tau_input=${WM_TAU_INPUTS[$i]} tau_feat=${WM_TAU_FEATS[$i]} band=[${WM_LOGIT_LOWS[$i]}, ${WM_LOGIT_HIGHS[$i]}]"

  create_model "$AID_HEX"
  register_model "$AID_HEX" "$EVIDENCE_HEX" "${WM_TAU_INPUTS[$i]}" "${WM_TAU_FEATS[$i]}" "${WM_LOGIT_LOWS[$i]}" "${WM_LOGIT_HIGHS[$i]}"
done

###############################################################################
# 4. Wait for a few blocks and report timings
###############################################################################

log_section "Waiting for 10 blocks and benchmarking"
wait_for_blocks 10 2 240

###############################################################################
# 5. Show some logs from api-gateway and chain
###############################################################################

log_section "Recent logs from api-gateway"

docker logs --tail=20 api-gateway || true

log_section "Recent logs from chain"

docker logs --tail=20 chain || true

log_section "Demo complete"

echo "You can now:"
echo "  - visit Prometheus at http://localhost:9090"
echo "  - manually hit http://localhost:8080/health (ml-service)"
echo "  - manually hit http://localhost:8081/health (api-gateway)"
echo "  - inspect metrics: http://localhost:9898/metrics (chain), http://localhost:9899/metrics (api-gateway)"
echo "  - total demo time: ${SECONDS}s"
echo
echo "To keep containers running after the script, use:"
echo "  KEEP_CONTAINERS=1 bash expts/demo.sh"
