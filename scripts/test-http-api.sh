#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 3 ]]; then
  echo "Usage: $0 <base_url> <api_key_id> <prompt> [model]" >&2
  echo "Example: $0 http://localhost:8080 vsk_live_123 'Hello' gpt-5" >&2
  exit 1
fi

base_url="$1"
api_key_id="$2"
prompt="$3"
model="${4:-gpt-5}"

json_escape() {
  python - <<'PY' "$1"
import json
import sys
print(json.dumps(sys.argv[1]))
PY
}

prompt_json=$(json_escape "$prompt")

start_payload=$(cat <<JSON
{"apiKeyId":"$api_key_id","model":"$model","initialContext":[]}
JSON
)

session_id=$(curl -sS -X POST "$base_url/session/start" \
  -H "Content-Type: application/json" \
  -d "$start_payload" \
  | python - <<'PY'
import json
import sys
obj = json.load(sys.stdin)
print(obj.get("sessionId", ""))
PY
)

if [[ -z "$session_id" ]]; then
  echo "Failed to start session" >&2
  exit 1
fi

echo "Session: $session_id"

send_payload=$(cat <<JSON
{"apiKeyId":"$api_key_id","sessionId":"$session_id","messages":[{"role":"user","content":$prompt_json}]}
JSON
)

turn_id=$(curl -sS -X POST "$base_url/session/send" \
  -H "Content-Type: application/json" \
  -d "$send_payload" \
  | python - <<'PY'
import json
import sys
obj = json.load(sys.stdin)
print(obj.get("turnId", ""))
PY
)

if [[ -z "$turn_id" ]]; then
  echo "Failed to send prompt" >&2
  exit 1
fi

echo "Turn: $turn_id"

echo "Streaming events (Ctrl+C to stop):"
# Expect server-sent events
curl -N -sS "$base_url/session/events?apiKeyId=$api_key_id&sessionId=$session_id"
