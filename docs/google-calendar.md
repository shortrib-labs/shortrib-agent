# Google Calendar MCP operations

## Configuration

The agent uses the production Calendar MCP endpoint by default:

```text
https://google-calendar-mcp-hrlw48pl49z9x55u3a2kremvhe.mcp.gateway.context.cloud/mcp/v1
```

Configure `GOOGLE_CALENDAR_MCP_REDIRECT_URI` as a public HTTPS URL and route its
`/oauth/callback` path to the socket in `GOOGLE_CALENDAR_MCP_CALLBACK_BIND`.
The bind address is private listener configuration; it must not appear in the
OAuth redirect URI. Local development defaults both values to loopback port
`3000`.

Keycard discovers RFC 9728 protected-resource metadata, issuer
`https://hrlw48pl49z9x55u3a2kremvhe.keycard.cloud/`, PKCE, and dynamic client
registration automatically. The agent lets RMCP select the scopes advertised by
the authorization server, currently including `openid` and `offline_access`.

Run `just verify-calendar-metadata` to make a credential-free request and check
the live resource, issuer, registration endpoint, and PKCE metadata. This check
never starts authorization and never receives or prints tokens.

## Encrypted credential persistence

Set both persistence variables or neither:

```sh
export GOOGLE_CALENDAR_OAUTH_STORAGE_DIR=/var/lib/shortrib-agent/oauth
export GOOGLE_CALENDAR_OAUTH_ENCRYPTION_KEY="$(openssl rand -base64 32)"
```

Generate the key once, put it in the deployment secret manager, and mount the
storage directory from a durable private volume. Keep the same key across
restarts. Losing it makes existing credentials undecryptable; exposing it
exposes every stored Calendar grant.

The agent serializes RMCP `StoredCredentials`, encrypts each record with
AES-256-GCM, binds the ciphertext to the Slack team/user identity as associated
data, hashes that identity for the filename, and writes through an atomic
rename. Unix directory and file modes are set to `0700` and `0600`. Access and
refresh tokens are never logged.

Connections restore lazily: after a restart, the first Calendar request by a
previously authorized Slack user loads their record, validates the discovered
issuer through RMCP, reconnects to MCP, and lets Keycard refresh tokens as
needed. If Keycard rejects restored credentials after its application or
credential configuration changes, the agent starts a new authorization flow
that replaces that user's stored grant on completion. Without the two
persistence variables, credentials remain in memory and every restart requires
authorization again.

Key rotation is not implemented. Rotate by stopping the agent, deleting the
credential directory, installing the new key, and asking users to authorize
again. The same reauthorization procedure applies when Google or Keycard
revokes a refresh token and automatic refresh can no longer recover.

## Replica and callback constraints

Run one Calendar-enabled agent replica. Do not point multiple replicas at the
same credential directory:

- Pending `OAuthState` and the map from OAuth state to Slack identity remain in
  memory. A restart during browser consent invalidates that attempt.
- Any callback must reach the same process that generated its authorization
  URL. Ordinary load-balancer affinity is insufficient because the browser
  first visits the callback host only after leaving the authorization server.
- RMCP `3.1.4`, pinned through Keycard revision
  `0d7aae33e67597b1a2c4c0a51022150cfacf6cc3`, has no distributed refresh guard.
  Concurrent replicas can race when refresh tokens rotate.
- A shared filesystem provides durability, not safe multi-instance
  coordination.

A viable multi-instance design needs a dedicated authorization coordinator (or
equivalent state-aware callback router) that owns authorization sessions,
persists encrypted PKCE state and DCR client configuration in a shared database,
atomically consumes callback state, and serializes token refresh per Slack
team/user. The pinned SDK exposes `CredentialStore` and `StateStore`, but its
high-level `OAuthState` session is not serializable or restorable. Implementing
the lower-level callback exchange also requires persisting and reconstructing
the redirect URI, registration/client configuration, and discovered metadata.
Until Keycard/RMCP exposes a complete restorable session and distributed refresh
contract, a single replica is the safe deployment mode.

## Human end-to-end verification

The browser consent step and live Calendar data require a real Slack user and
Google account, so they cannot run in CI. Verify a deployment as follows:

1. Run `just check`, `just build`, and `just verify-calendar-metadata`.
2. Configure the environment from `.env.example`, including durable storage,
   and start one replica.
3. Confirm the public redirect URI returns an HTTP response from the agent on
   `/oauth/callback`. A request without OAuth parameters should return
   `400 Bad Request`; do not place a real `code` or `state` in shell history.
4. In Slack, ask: “What is the next event on my Google Calendar?” The agent
   should invoke `connect_google_calendar` and send an ephemeral authorization
   link visible only to that Slack user.
5. Open the link in a browser, choose the intended Google account, review the
   consent screen, and approve it. The callback page should say that Calendar
   is connected. Do not copy the callback URL into logs or tickets because it
   contains a short-lived authorization code and OAuth state.
6. Repeat the Slack question. The agent should load the live Calendar tools and
   answer from that account. Cross-check the returned event in Google Calendar.
   Use a read-only question for the first test; test create/update/delete only
   in a disposable calendar.
7. Restart the replica without changing the encryption key or storage volume,
   then repeat the question. No authorization link should appear, proving
   credential restoration. Inspect only file presence, ownership, and modes;
   never decrypt or print the record during routine verification.
8. Repeat with a second Slack user and confirm each sees only their own Calendar
   data and receives their own private authorization link.

Record the deployment version, Slack test identities, consent result, observed
Calendar answer, restart result, and timestamp. Do not record authorization
URLs, callback URLs, access tokens, refresh tokens, or encryption keys.
