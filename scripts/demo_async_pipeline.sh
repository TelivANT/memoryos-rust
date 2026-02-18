#!/usr/bin/env bash
set -euo pipefail

# One-click async pipeline demo:
# 1) Start middleware (Redis + Qdrant)
# 2) Start gateway + worker
# 3) Run smoke_async_pipeline.sh
# 4) Print quick verification hints

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LOG_DIR="${ROOT_DIR}/.demo-logs"
mkdir -p "${LOG_DIR}"

GATEWAY_LOG="${LOG_DIR}/gateway.log"
WORKER_LOG="${LOG_DIR}/worker.log"

GATEWAY_URL="${GATEWAY_URL:-http://127.0.0.1:8080}"
REDIS_URL="${REDIS_URL:-redis://127.0.0.1:6379}"
DEMO_TIMEOUT_SEC="${DEMO_TIMEOUT_SEC:-60}"

need_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "[demo_async_pipeline][ERROR] missing command: $1"
    exit 1
  fi
}

cleanup() {
  set +e
  if [[ -n "${WORKER_PID:-}" ]]; then
    kill "${WORKER_PID}" >/dev/null 2>&1 || true
  fi
  if [[ -n "${GATEWAY_PID:-}" ]]; then
    kill "${GATEWAY_PID}" >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT

need_cmd cargo
need_cmd curl
need_cmd redis-cli
if command -v docker-compose >/dev/null 2>&1; then
  COMPOSE_BIN="docker-compose"
elif command -v docker >/dev/null 2>&1; then
  COMPOSE_BIN="docker compose"
else
  echo "[demo_async_pipeline][ERROR] missing docker-compose/docker compose"
  exit 1
fi

echo "[demo_async_pipeline] root=${ROOT_DIR}"
echo "[demo_async_pipeline] logs=${LOG_DIR}"
echo "[demo_async_pipeline] starting middleware..."

cd "${ROOT_DIR}"
${COMPOSE_BIN} -f docker-compose.middleware-demo.yml up -d

echo "[demo_async_pipeline] waiting redis..."
for _ in {1..30}; do
  if redis-cli -u "${REDIS_URL}" ping >/dev/null 2>&1; then
    break
  fi
  sleep 1
done
if ! redis-cli -u "${REDIS_URL}" ping >/dev/null 2>&1; then
  echo "[demo_async_pipeline][ERROR] redis not ready: ${REDIS_URL}"
  exit 1
fi

echo "[demo_async_pipeline] starting gateway..."
MEMORYOS_ASYNC_MEMORY_PIPELINE=true \
RUST_LOG=info \
cargo run -p memoryos-gateway >"${GATEWAY_LOG}" 2>&1 &
GATEWAY_PID=$!

echo "[demo_async_pipeline] starting worker..."
RUST_LOG=info \
cargo run -p memoryos-worker >"${WORKER_LOG}" 2>&1 &
WORKER_PID=$!

echo "[demo_async_pipeline] waiting gateway health..."
deadline=$(( $(date +%s) + DEMO_TIMEOUT_SEC ))
ready=0
while [[ "$(date +%s)" -lt "${deadline}" ]]; do
  if curl -s "${GATEWAY_URL}/health/live" >/dev/null 2>&1; then
    ready=1
    break
  fi
  sleep 1
done
if [[ "${ready}" -ne 1 ]]; then
  echo "[demo_async_pipeline][ERROR] gateway not ready in ${DEMO_TIMEOUT_SEC}s"
  echo "[demo_async_pipeline] tail gateway log:"
  tail -n 40 "${GATEWAY_LOG}" || true
  exit 1
fi

echo "[demo_async_pipeline] running smoke check..."
GATEWAY_URL="${GATEWAY_URL}" REDIS_URL="${REDIS_URL}" "${ROOT_DIR}/scripts/smoke_async_pipeline.sh"

echo "[demo_async_pipeline] quick status:"
echo "  gateway log: ${GATEWAY_LOG}"
echo "  worker  log: ${WORKER_LOG}"
echo "  stream len: $(redis-cli -u "${REDIS_URL}" XLEN chat_log | tr -d '\r')"
echo "  dlq len:    $(redis-cli -u "${REDIS_URL}" XLEN chat_log:dlq | tr -d '\r')"
echo "[demo_async_pipeline] PASS"
