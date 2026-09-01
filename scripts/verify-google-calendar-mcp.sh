#!/usr/bin/env bash
set -euo pipefail

endpoint="${GOOGLE_CALENDAR_MCP_URL:-https://google-calendar-mcp-hrlw48pl49z9x55u3a2kremvhe.mcp.gateway.context.cloud/mcp/v1}"
expected_issuer="https://hrlw48pl49z9x55u3a2kremvhe.keycard.cloud/"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

status="$(curl --silent --show-error \
    --dump-header "$work/headers" \
    --output /dev/null \
    --write-out '%{http_code}' \
    --request POST \
    --header 'content-type: application/json' \
    --header 'accept: application/json, text/event-stream' \
    --data '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"shortrib-agent-metadata-check","version":"0.1.0"}}}' \
    "$endpoint")"

if [[ "$status" != "401" ]]; then
    printf 'Expected unauthenticated MCP request to return 401, got %s\n' "$status" >&2
    exit 1
fi

resource_metadata="$(tr -d '\r' < "$work/headers" \
    | sed -n 's/^[Ww][Ww][Ww]-[Aa]uthenticate:.*resource_metadata="\([^"]*\)".*/\1/p' \
    | head -n 1)"
if [[ -z "$resource_metadata" ]]; then
    printf 'MCP response did not advertise RFC 9728 resource metadata\n' >&2
    exit 1
fi

curl --fail --silent --show-error "$resource_metadata" > "$work/resource.json"
jq --exit-status \
    --arg endpoint "$endpoint" \
    --arg issuer "$expected_issuer" \
    '.resource == $endpoint and (.authorization_servers | index($issuer) != null)' \
    "$work/resource.json" >/dev/null

issuer="$(jq --raw-output '.authorization_servers[0]' "$work/resource.json")"
authorization_metadata="${issuer%/}/.well-known/oauth-authorization-server"
curl --fail --silent --show-error "$authorization_metadata" > "$work/authorization.json"
jq --exit-status \
    --arg issuer "${expected_issuer%/}" \
    '.issuer == $issuer
     and (.registration_endpoint | type == "string")
     and (.code_challenge_methods_supported | index("S256") != null)
     and ((has("scopes_supported") | not) or .scopes_supported == null or .scopes_supported == [])' \
    "$work/authorization.json" >/dev/null

printf 'Google Calendar MCP OAuth metadata verified (401, RFC 9728, issuer, DCR, PKCE S256, no scopes).\n'
