#!/usr/bin/env bash
set -euo pipefail

GATEWAY_URL="${GATEWAY_URL:-http://127.0.0.1:8080}"
REDIS_URL="${REDIS_URL:-redis://127.0.0.1:6379}"
STREAM_KEY="${STREAM_KEY:-chat_log}"
USER_ID="${USER_ID:-smoke-user}"
TIMEOUT_SEC="${TIMEOUT_SEC:-20}"

need_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "[smoke_async_pipeline][ERROR] missing command: $1"
    exit 1
  fi
}

need_cmd curl
need_cmd redis-cli
need_cmd date

event_id="evt-smoke-$(date +%s)"
payload="$(cat <<EOF
{"user_id":"${USER_ID}","role":"user","content":"smoke async pipeline ${event_id}","event_id":"${event_id}"}
EOF
)"

echo "[smoke_async_pipeline] gateway=${GATEWAY_URL} redis=${REDIS_URL} stream=${STREAM_KEY} event_id=${event_id}"

before_len="$(redis-cli -u "${REDIS_URL}" XLEN "${STREAM_KEY}" | tr -d '\r')"
echo "[smoke_async_pipeline] stream length before=${before_len}"

http_code="$(
  curl -s -o /tmp/memoryos_smoke_resp.json -w "%{http_code}" \
    -X POST "${GATEWAY_URL}/v1/memory/add" \
    -H "Content-Type: application/json" \
    -d "${payload}"
)"

if [[ "${http_code}" != "200" ]]; then
  echo "[smoke_async_pipeline][ERROR] /v1/memory/add status=${http_code}"
  cat /tmp/memoryos_smoke_resp.json || true
  exit 1
fi

echo "[smoke_async_pipeline] memory/add accepted"

deadline=$(( $(date +%s) + TIMEOUT_SEC ))
found=0
while [[ "$(date +%s)" -lt "${deadline}" ]]; do
  if redis-cli -u "${REDIS_URL}" --raw XRANGE "${STREAM_KEY}" - + COUNT 200 \
    | grep -q "${event_id}"; then
    found=1
    break
  fi
  sleep 1
done

if [[ "${found}" -ne 1 ]]; then
  echo "[smoke_async_pipeline][ERROR] event_id not found in stream within ${TIMEOUT_SEC}s: ${event_id}"
  exit 1
fi

after_len="$(redis-cli -u "${REDIS_URL}" XLEN "${STREAM_KEY}" | tr -d '\r')"
echo "[smoke_async_pipeline] stream length after=${after_len}"
echo "[smoke_async_pipeline] PASS event queued: ${event_id}"
