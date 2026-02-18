#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT_DIR"

REDIS_CONTAINER="memoryos-it-redis"
QDRANT_CONTAINER="memoryos-it-qdrant"
GATEWAY_URL="http://127.0.0.1:8080"
LOG_FILE="/tmp/memoryos-outage-it.log"
GATEWAY_PID=""
SERVICE_MODE=""
COMPOSE_FILE_PATH="${COMPOSE_FILE_PATH:-docker-compose.middleware-demo.yml}"

compose() {
  docker-compose -f "$COMPOSE_FILE_PATH" "$@"
}

cleanup() {
  if [[ -n "${GATEWAY_PID}" ]] && kill -0 "${GATEWAY_PID}" >/dev/null 2>&1; then
    kill "${GATEWAY_PID}" >/dev/null 2>&1 || true
    wait "${GATEWAY_PID}" >/dev/null 2>&1 || true
  fi
  if [[ "${SERVICE_MODE}" == "compose" ]]; then
    compose down >/dev/null 2>&1 || true
  else
    docker rm -f "${REDIS_CONTAINER}" "${QDRANT_CONTAINER}" >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT

require_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "[outage-test] missing command: $1" >&2
    exit 1
  fi
}

require_cmd docker
require_cmd curl
require_cmd jq
require_cmd lsof
require_cmd docker-compose

if lsof -nP -iTCP:8080 -sTCP:LISTEN >/dev/null 2>&1; then
  echo "[outage-test] port 8080 is already in use, stop existing gateway first" >&2
  exit 1
fi

wait_http() {
  local url="$1"
  local timeout="${2:-60}"
  local start
  start="$(date +%s)"
  while true; do
    if curl -fsS "$url" >/dev/null 2>&1; then
      return 0
    fi
    if (( "$(date +%s)" - start > timeout )); then
      echo "[outage-test] timeout waiting for $url" >&2
      return 1
    fi
    sleep 1
  done
}

wait_qdrant_ready() {
  local timeout="${1:-90}"
  local start
  start="$(date +%s)"
  while true; do
    if curl -fsS "http://127.0.0.1:6333/health" >/dev/null 2>&1 \
      || curl -fsS "http://127.0.0.1:6333/healthz" >/dev/null 2>&1; then
      return 0
    fi
    if (( "$(date +%s)" - start > timeout )); then
      echo "[outage-test] timeout waiting qdrant health endpoint (/health or /healthz)" >&2
      return 1
    fi
    sleep 1
  done
}

wait_mode() {
  local expected="$1"
  local timeout="${2:-60}"
  local start
  start="$(date +%s)"
  while true; do
    local body
    body="$(curl -fsS "$GATEWAY_URL/health/status" || true)"
    if [[ -n "$body" ]]; then
      local mode
      mode="$(echo "$body" | jq -r '.mode // empty')"
      if [[ "$mode" == "$expected" ]]; then
        return 0
      fi
    fi
    if (( "$(date +%s)" - start > timeout )); then
      echo "[outage-test] timeout waiting mode=$expected, latest=$body" >&2
      return 1
    fi
    sleep 1
  done
}

current_health_json() {
  curl -fsS "$GATEWAY_URL/health/status"
}

assert_health_matrix() {
  local expected_mode="$1"
  local expected_redis="$2"
  local expected_qdrant="$3"
  local body
  body="$(curl -fsS "$GATEWAY_URL/health/status")"
  local mode redis qdrant
  mode="$(echo "$body" | jq -r '.mode')"
  redis="$(echo "$body" | jq -r '.redis')"
  qdrant="$(echo "$body" | jq -r '.qdrant')"
  [[ "$mode" == "$expected_mode" ]] || {
    echo "[outage-test] expected mode=$expected_mode, got $mode" >&2
    exit 1
  }
  [[ "$redis" == "$expected_redis" ]] || {
    echo "[outage-test] expected redis=$expected_redis, got $redis" >&2
    exit 1
  }
  [[ "$qdrant" == "$expected_qdrant" ]] || {
    echo "[outage-test] expected qdrant=$expected_qdrant, got $qdrant" >&2
    exit 1
  }
}

wait_health_matrix() {
  local expected_mode="$1"
  local expected_redis="$2"
  local expected_qdrant="$3"
  local timeout="${4:-60}"
  local start
  start="$(date +%s)"
  while true; do
    local body mode redis qdrant
    body="$(current_health_json || true)"
    if [[ -n "$body" ]]; then
      mode="$(echo "$body" | jq -r '.mode // empty')"
      redis="$(echo "$body" | jq -r '.redis // empty')"
      qdrant="$(echo "$body" | jq -r '.qdrant // empty')"
      if [[ "$mode" == "$expected_mode" && "$redis" == "$expected_redis" && "$qdrant" == "$expected_qdrant" ]]; then
        return 0
      fi
    fi
    if (( "$(date +%s)" - start > timeout )); then
      echo "[outage-test] timeout waiting matrix mode=$expected_mode redis=$expected_redis qdrant=$expected_qdrant, latest=$body" >&2
      return 1
    fi
    sleep 1
  done
}

assert_retrieve_degraded_header() {
  local expect_degraded="$1" # true|false
  local response
  response="$(curl -si -X POST "$GATEWAY_URL/v1/memory/retrieve" \
    -H "Content-Type: application/json" \
    -d '{"user_id":"it-user","query":"hello"}')"
  local status
  status="$(echo "$response" | head -n1 | awk '{print $2}')"
  [[ "$status" == "200" ]] || {
    echo "[outage-test] expected HTTP 200, got $status" >&2
    echo "$response" >&2
    exit 1
  }

  if [[ "$expect_degraded" == "true" ]]; then
    echo "$response" | grep -iq '^X-MemoryOS-Status: degraded' || {
      echo "[outage-test] expected degraded header but missing" >&2
      echo "$response" >&2
      exit 1
    }
  else
    if echo "$response" | grep -iq '^X-MemoryOS-Status: degraded'; then
      echo "[outage-test] degraded header should not exist in ready mode" >&2
      echo "$response" >&2
      exit 1
    fi
  fi
}

setup_backends() {
  local run_help
  run_help="$(docker run --help 2>&1 || true)"
  if echo "$run_help" | grep -qE '(^|[[:space:]])docker run([[:space:]]|$)'; then
    SERVICE_MODE="docker"
    echo "[outage-test] backend mode: docker"
    docker rm -f "${REDIS_CONTAINER}" "${QDRANT_CONTAINER}" >/dev/null 2>&1 || true
    docker run -d --name "${REDIS_CONTAINER}" -p 6379:6379 redis:7-alpine >/dev/null
    docker run -d --name "${QDRANT_CONTAINER}" -p 6333:6333 qdrant/qdrant:latest >/dev/null
  else
    SERVICE_MODE="compose"
    echo "[outage-test] backend mode: docker-compose"
    compose down >/dev/null 2>&1 || true
    compose up -d redis qdrant >/dev/null
  fi
}

pause_service() {
  local svc="$1"
  if [[ "${SERVICE_MODE}" == "compose" ]]; then
    compose pause "$svc" >/dev/null
  else
    if [[ "$svc" == "redis" ]]; then
      docker pause "${REDIS_CONTAINER}" >/dev/null
    else
      docker pause "${QDRANT_CONTAINER}" >/dev/null
    fi
  fi
}

unpause_service() {
  local svc="$1"
  if [[ "${SERVICE_MODE}" == "compose" ]]; then
    compose unpause "$svc" >/dev/null
  else
    if [[ "$svc" == "redis" ]]; then
      docker unpause "${REDIS_CONTAINER}" >/dev/null
    else
      docker unpause "${QDRANT_CONTAINER}" >/dev/null
    fi
  fi
}

echo "[outage-test] Starting Redis/Qdrant containers..."
setup_backends
wait_qdrant_ready 90

echo "[outage-test] Starting gateway..."
if [[ -x "./target/release/memoryos-gateway" ]]; then
  RUST_LOG=warn ./target/release/memoryos-gateway >"$LOG_FILE" 2>&1 &
else
  RUST_LOG=warn cargo run --release -p memoryos-gateway >"$LOG_FILE" 2>&1 &
fi
GATEWAY_PID="$!"
wait_http "$GATEWAY_URL/health" 180
local_health="$(current_health_json)"
baseline_mode="$(echo "$local_health" | jq -r '.mode')"
baseline_redis="$(echo "$local_health" | jq -r '.redis')"
baseline_qdrant="$(echo "$local_health" | jq -r '.qdrant')"

if [[ "$baseline_redis" != "up" ]]; then
  echo "[outage-test] baseline invalid: redis is not up ($baseline_redis)" >&2
  echo "$local_health" >&2
  exit 1
fi

if [[ "$baseline_qdrant" == "up" ]]; then
  echo "[outage-test] baseline: full ready matrix"
  wait_mode "ready" 90
  assert_health_matrix "ready" "up" "up"
  assert_retrieve_degraded_header "false"

  echo "[outage-test] Inject outage: pause qdrant"
  pause_service qdrant
  wait_health_matrix "degraded_ready" "up" "down" 30
  assert_health_matrix "degraded_ready" "up" "down"
  assert_retrieve_degraded_header "true"

  echo "[outage-test] Recover: unpause qdrant"
  unpause_service qdrant
  wait_health_matrix "ready" "up" "up" 60
  assert_health_matrix "ready" "up" "up"
  assert_retrieve_degraded_header "false"
else
  echo "[outage-test] baseline: qdrant unavailable ($baseline_qdrant), running redis-only degraded recovery checks"
  wait_mode "degraded_ready" 90
  assert_health_matrix "degraded_ready" "up" "down"
  assert_retrieve_degraded_header "true"
fi

echo "[outage-test] Inject outage: pause redis"
pause_service redis
if [[ "$baseline_qdrant" == "up" ]]; then
  wait_health_matrix "degraded_ready" "down" "up" 30
else
  wait_health_matrix "degraded_ready" "down" "down" 30
fi
if [[ "$baseline_qdrant" == "up" ]]; then
  assert_health_matrix "degraded_ready" "down" "up"
else
  assert_health_matrix "degraded_ready" "down" "down"
fi
assert_retrieve_degraded_header "true"

echo "[outage-test] Recover: unpause redis"
unpause_service redis
if [[ "$baseline_qdrant" == "up" ]]; then
  wait_health_matrix "ready" "up" "up" 60
  assert_health_matrix "ready" "up" "up"
  assert_retrieve_degraded_header "false"
else
  wait_health_matrix "degraded_ready" "up" "down" 60
  assert_health_matrix "degraded_ready" "up" "down"
  assert_retrieve_degraded_header "true"
fi

echo "[outage-test] PASS"
