#!/usr/bin/env bash
set -euo pipefail

REDIS_URL="${REDIS_URL:-redis://127.0.0.1:6379}"
DLQ_STREAM="${DLQ_STREAM:-chat_log:dlq}"
TARGET_STREAM="${TARGET_STREAM:-chat_log}"
COUNT="${COUNT:-100}"
DRY_RUN="${DRY_RUN:-1}"

usage() {
  cat <<EOF
Replay messages from Redis Stream DLQ back to target stream.

Environment variables:
  REDIS_URL      Redis URL (default: redis://127.0.0.1:6379)
  DLQ_STREAM     DLQ stream name (default: chat_log:dlq)
  TARGET_STREAM  target stream name (default: chat_log)
  COUNT          max messages to process (default: 100)
  DRY_RUN        1=dry-run (default), 0=execute replay

Example:
  DRY_RUN=1 COUNT=10 ./scripts/replay_dlq.sh
  DRY_RUN=0 COUNT=50 REDIS_URL=redis://localhost:6379 ./scripts/replay_dlq.sh
EOF
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

echo "[replay_dlq] REDIS_URL=${REDIS_URL} DLQ_STREAM=${DLQ_STREAM} TARGET_STREAM=${TARGET_STREAM} COUNT=${COUNT} DRY_RUN=${DRY_RUN}"

processed=0
skipped=0
failed=0

for ((i = 0; i < COUNT; i++)); do
  result="$(
    redis-cli -u "${REDIS_URL}" --raw EVAL "
      local dlq = KEYS[1]
      local target = KEYS[2]
      local dry_run = ARGV[1]

      local entries = redis.call('XRANGE', dlq, '-', '+', 'COUNT', 1)
      if #entries == 0 then
        return {'EMPTY'}
      end

      local id = entries[1][1]
      local fields = entries[1][2]
      local payload = nil
      local event_id = id

      for i=1,#fields,2 do
        local k = fields[i]
        local v = fields[i+1]
        if k == 'payload' then
          payload = v
        elseif k == 'event_id' and v and v ~= '' then
          event_id = v
        end
      end

      if not payload then
        return {'NO_PAYLOAD', id, event_id}
      end

      if dry_run == '1' then
        return {'DRY', id, event_id}
      end

      local new_id = redis.call('XADD', target, '*', 'event_id', event_id, 'payload', payload)
      redis.call('XDEL', dlq, id)
      return {'OK', id, new_id, event_id}
    " 2 "${DLQ_STREAM}" "${TARGET_STREAM}" "${DRY_RUN}"
  )"

  status="$(echo "${result}" | head -n1 | tr -d '\r')"
  case "${status}" in
    EMPTY)
      echo "[replay_dlq] no more entries in ${DLQ_STREAM}"
      break
      ;;
    DRY)
      id="$(echo "${result}" | sed -n '2p' | tr -d '\r')"
      eid="$(echo "${result}" | sed -n '3p' | tr -d '\r')"
      echo "[replay_dlq][DRY] would replay dlq_id=${id} event_id=${eid}"
      processed=$((processed + 1))
      ;;
    OK)
      id="$(echo "${result}" | sed -n '2p' | tr -d '\r')"
      new_id="$(echo "${result}" | sed -n '3p' | tr -d '\r')"
      eid="$(echo "${result}" | sed -n '4p' | tr -d '\r')"
      echo "[replay_dlq] replayed dlq_id=${id} -> target_id=${new_id} event_id=${eid}"
      processed=$((processed + 1))
      ;;
    NO_PAYLOAD)
      id="$(echo "${result}" | sed -n '2p' | tr -d '\r')"
      eid="$(echo "${result}" | sed -n '3p' | tr -d '\r')"
      echo "[replay_dlq][WARN] skip dlq_id=${id} event_id=${eid}: missing payload"
      skipped=$((skipped + 1))
      ;;
    *)
      echo "[replay_dlq][ERROR] unexpected redis response:"
      echo "${result}"
      failed=$((failed + 1))
      break
      ;;
  esac
done

echo "[replay_dlq] done processed=${processed} skipped=${skipped} failed=${failed}"

if [[ "${failed}" -gt 0 ]]; then
  exit 1
fi
