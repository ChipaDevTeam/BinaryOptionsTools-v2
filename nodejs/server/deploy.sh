#!/usr/bin/env bash
#
# Deploys the console image to ChipaCloud Run.
#
# Creates the service the first time and replaces the image on every run after
# that, so the same command works for a first deploy and for shipping a new
# build.
#
#   CHIPA_KEY=cc_xxx IMAGE=ghcr.io/chipadevteam/binary-options-console:sha ./deploy.sh
#
# Environment:
#   CHIPA_KEY      required  API key with the cloudrun:write scope
#   IMAGE          required  publicly pullable image reference
#   CHIPA_API                base URL (default https://cloud.chipatrade.com/cloudrun)
#   SERVICE_NAME             service name (default binary-options-console)
#   AUTH_TOKEN               token the console requires on every /api call
#   ALLOW_TRADING            set to 1 to let the deployed console place orders
#   MEMORY / CPU             container limits (default 512Mi / 1000m)

set -euo pipefail

CHIPA_API="${CHIPA_API:-https://cloud.chipatrade.com/cloudrun}"
SERVICE_NAME="${SERVICE_NAME:-binary-options-console}"
MEMORY="${MEMORY:-512Mi}"
CPU="${CPU:-1000m}"

die() { echo "error: $*" >&2; exit 1; }

[ -n "${CHIPA_KEY:-}" ] || die "CHIPA_KEY is not set. Console → IAM → Create API key, with Cloud Run → Write."
[ -n "${IMAGE:-}" ] || die "IMAGE is not set. Point it at a publicly pullable image reference."

# The deployed console gets a public HTTPS URL. Without a token, anyone who
# finds it can drive any session opened on it.
if [ -z "${AUTH_TOKEN:-}" ] && [ "${ALLOW_NO_AUTH:-}" != "1" ]; then
  die "AUTH_TOKEN is not set. Generate one (openssl rand -hex 24), or set ALLOW_NO_AUTH=1 to deploy it open on purpose."
fi

if [ "${ALLOW_TRADING:-}" = "1" ]; then
  echo "warning: ALLOW_TRADING=1 — the deployed console will be able to place real orders." >&2
fi

api() {
  local method="$1" path="$2" body="${3:-}"
  local args=(-sS -X "$method" "$CHIPA_API$path"
              -H "Authorization: Bearer $CHIPA_KEY"
              -H "Content-Type: application/json"
              -w '\n%{http_code}')
  [ -n "$body" ] && args+=(-d "$body")
  curl "${args[@]}"
}

# Split the trailing status code curl appends from the body.
split() {
  local response="$1"
  BODY="$(printf '%s' "$response" | sed '$d')"
  STATUS="$(printf '%s' "$response" | tail -n1)"
}

json_field() { node -e '
  let raw = "";
  process.stdin.on("data", (d) => (raw += d)).on("end", () => {
    try { const v = JSON.parse(raw); process.stdout.write(String(v?.[process.argv[1]] ?? "")); }
    catch { process.stdout.write(""); }
  });' "$1"; }

env_json() { node -e '
  const env = { AUTH_TOKEN: process.env.AUTH_TOKEN || "", ALLOW_TRADING: process.env.ALLOW_TRADING || "" };
  for (const k of Object.keys(env)) if (!env[k]) delete env[k];
  process.stdout.write(JSON.stringify(env));'; }

echo "Looking for an existing service named $SERVICE_NAME…"
split "$(api GET /services)"
[ "$STATUS" = "200" ] || die "listing services returned HTTP $STATUS: $BODY"

SERVICE_ID="$(printf '%s' "$BODY" | node -e '
  let raw = "";
  process.stdin.on("data", (d) => (raw += d)).on("end", () => {
    let list = [];
    try { const v = JSON.parse(raw); list = Array.isArray(v) ? v : (v.services || v.items || []); } catch {}
    const hit = list.find((s) => s && s.name === process.argv[1]);
    process.stdout.write(hit ? String(hit.id) : "");
  });' "$SERVICE_NAME")"

PAYLOAD="$(IMAGE="$IMAGE" node -e '
  const body = {
    name: process.argv[1],
    image: process.env.IMAGE,
    port: 8080,
    cpu: process.argv[2],
    memory: process.argv[3],
    env_vars: JSON.parse(process.argv[4]),
  };
  process.stdout.write(JSON.stringify(body));' "$SERVICE_NAME" "$CPU" "$MEMORY" "$(env_json)")"

if [ -n "$SERVICE_ID" ]; then
  echo "Updating $SERVICE_NAME ($SERVICE_ID) to $IMAGE…"
  split "$(api PUT "/services/$SERVICE_ID" "$PAYLOAD")"
  [ "$STATUS" = "200" ] || die "update returned HTTP $STATUS: $BODY"
else
  echo "Creating $SERVICE_NAME from $IMAGE…"
  split "$(api POST /services "$PAYLOAD")"
  case "$STATUS" in
    200|201) ;;
    409) die "a service named $SERVICE_NAME already exists but was not in the list — check the console" ;;
    403) die "the API key is missing the cloudrun:write scope" ;;
    *) die "create returned HTTP $STATUS: $BODY" ;;
  esac
  SERVICE_ID="$(printf '%s' "$BODY" | json_field id)"
fi

STATE="$(printf '%s' "$BODY" | json_field status)"
PROXY_URL="$(printf '%s' "$BODY" | json_field proxy_url)"

echo
echo "service id : $SERVICE_ID"
echo "status     : $STATE"
# proxy_url works immediately; `url` is a vanity hostname with no certificate
# until someone creates one on the server, so it is not printed as the address.
echo "url        : $PROXY_URL"

if [ "$STATE" = "ERROR" ]; then
  echo
  echo "The deploy reported ERROR. Recent logs:" >&2
  split "$(api GET "/services/$SERVICE_ID/logs?tail=50")"
  printf '%s\n' "$BODY" >&2
  exit 1
fi
