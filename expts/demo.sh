#!/usr/bin/env bash
#
# expts/demo.sh
#
# End-to-end demo:
#   - builds & starts ml-service, chain, api-gateway, prometheus
#   - waits for health endpoints
#   - creates a dummy model artefact inside ml-service
#   - registers that model via api-gateway /models/register
#   - shows a few log lines
#   - tears the stack down (docker compose down -v)
#
# Usage:
#   bash expts/demo.sh
#
# Set KEEP_CONTAINERS=1 to leave the stack running after the demo:
#   KEEP_CONTAINERS=1 bash expts/demo.sh

set -euo pipefail

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
      echo "  ➜ OK (${url})"
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

###############################################################################
# 2. Wait for health / metrics endpoints
###############################################################################

# ml-service: /health
wait_for_http "http://localhost:8080/health"

# api-gateway: /health
wait_for_http "http://localhost:8081/health"

# chain metrics
wait_for_http "http://localhost:9898/metrics"

# api-gateway metrics (mapped to host 9899)
wait_for_http "http://localhost:9899/metrics"

###############################################################################
# 3. Create a dummy model artefact inside ml-service
###############################################################################

log_section "Creating dummy model artefact inside ml-service"

# This AID must match what we send to /models/register below.
# 64 hex chars = 32 bytes.
AID_HEX="bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"

# path inside the ml-service container: /app/ml_service/models/<aid>.pt
docker exec ml-service python - <<PY
import os
from pathlib import Path
import torch

model_root = Path(os.environ.get("ML_SERVICE_MODEL_ROOT", "/app/ml_service/models"))
model_root.mkdir(parents=True, exist_ok=True)
aid_hex = "$AID_HEX"
model_path = model_root / f"{aid_hex}.pt"

# Save a tiny torch object so torch.load() works
torch.save({"demo": "model"}, model_path)
print(f"Created dummy model at {model_path}")
PY

###############################################################################
# 4. Call api-gateway /models/register
###############################################################################

log_section "Registering dummy model via api-gdoateway /models/register"

OWNER_HEX="aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
EVIDENCE_HEX="cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"

REGISTER_PAYLOAD=$(cat <<JSON
{
  "owner_account_hex": "${OWNER_HEX}",
  "aid_hex": "${AID_HEX}",
  "scheme_id": "multi_factor_v1",
  "evidence_hash_hex": "${EVIDENCE_HEX}",
  "wm_profile": {
    "tau_input": 0.0,
    "tau_feat": 1.0,
    "logit_band_low": -1.0,
    "logit_band_high": 1.0
  }
}
JSON
)

echo "Request payload:"
echo "$REGISTER_PAYLOAD" | sed 's/^/  /'

echo
echo "Sending request to http://localhost:8081/models/register ..."
echo

curl -i -X POST "http://localhost:8081/models/register" \
  -H "Content-Type: application/json" \
  -d "$REGISTER_PAYLOAD"

echo

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
echo
echo "To keep containers running after the script, use:"
echo "  KEEP_CONTAINERS=1 bash expts/demo.sh"
