# Deterministic Slack Calendar App Home

The App Home path is intentionally independent of the LLM and Calendar MCP:

```text
authenticated Slack event
  -> verify bot-token team and Slack user
  -> users.info (workspace email)
  -> Keycard substitute-user token exchange
  -> Google Calendar API v3
  -> deterministic Slack Block Kit view
```

The existing encrypted RMCP `StoredCredentials` remain the source of truth for
Calendar MCP chat sessions only. They are resource-bound to the MCP gateway;
they cannot mint a Google Calendar audience token and must not be sent to
Google. The App Home instead uses the pinned `keycard` Rust SDK's
`Client::impersonate(email, "https://www.googleapis.com/calendar/v3")` flow.
It requests the narrow read/write event scope
`https://www.googleapis.com/auth/calendar.events` and discards every returned
provider token after that App Home operation.

## Slack configuration

1. Enable **App Home > Home Tab**.
2. Subscribe the bot to `app_home_opened`.
3. Enable interactivity. Socket Mode delivers both events and block actions, so
   no public Slack request URL is needed.
4. Add `users:read` and `users:read.email` bot scopes, in addition to the
   existing messaging scopes, then reinstall the app.

At startup the bot calls `auth.test` and accepts App Home events and actions
only for that bot token's team. Each button value contains the publishing team,
user, and event ID; the action is accepted only when Slack's authenticated
interaction has the same team and user. App Home views are private to the user,
and only Slack can deliver a Socket Mode interaction. Duplicate action
timestamps are ignored for ten minutes to absorb Slack retries.

The Keycard user identifier is the email returned by Slack `users.info`. Treat
the workspace's email ownership controls as the verified Slack-to-Keycard
identity link, and ensure each Keycard user has that exact identifier. Do not
replace this lookup with an action value, request parameter, or unverified
profile field.

## Keycard and Google configuration

Create or confirm the Google Calendar catalog resource with the exact identifier
`https://www.googleapis.com/calendar/v3`, Google Calendar API enabled, and the
`https://www.googleapis.com/auth/calendar.events` scope.

Create two Keycard applications as described by Keycard's **Act on Behalf of
Absent Users** guide:

- A confidential shortrib-agent application with the Calendar resource as a
  dependency and implicit consent. Generate its client ID and secret (or adapt
  the code to a supported workload credential before production).
- A public application with the Calendar resource as a dependency and the
  App Home callback URI registered as an allowed redirect. The same
  shortrib-agent process runs its authorization-code-with-PKCE flow; no separate
  landing-page service is required.

Configure:

```sh
KEYCARD_ISSUER=https://YOUR_ZONE.keycard.cloud
KEYCARD_CLIENT_ID=...
KEYCARD_CLIENT_SECRET=...
KEYCARD_CALENDAR_PUBLIC_CLIENT_ID=...
KEYCARD_CALENDAR_REDIRECT_URI=https://agent.example.com/calendar/callback
KEYCARD_CALENDAR_CALLBACK_BIND=0.0.0.0:3001
```

Keep the secret in the deployment secret manager. The app never logs it or
provider tokens. The redirect URI must be HTTPS in production (loopback HTTP is
accepted for local development), must exactly match the public Keycard
application registration, and must route to the private callback bind. Existing
MCP consent may not establish delegation for this distinct confidential
application. `interaction_required`, an unknown/revoked user grant, Google
401/403, and missing scope all start the in-process authorization flow rather
than exposing provider details.

## In-process authorization

When Keycard reports that the Slack-linked user has no usable Calendar grant,
the agent generates a 128-character PKCE verifier, S256 challenge, and random
32-byte state. It stores the verifier, expected Slack team/user/email, and
authorization URL in memory for ten minutes, then puts the Keycard authorization
URL directly on the private App Home.

Keycard redirects the browser to `KEYCARD_CALENDAR_REDIRECT_URI`. The callback
listener accepts only that configured path, atomically consumes state, exchanges
the code with `KEYCARD_CALENDAR_PUBLIC_CLIENT_ID`, and discards the returned
tokens. It then proves the authorized account established delegation for the
expected Slack email by performing a confidential impersonation exchange and a
Calendar list request. A different Keycard login therefore cannot authorize or
expose data for the expected identity. On success, the process automatically
republishes that user's App Home.

Pending PKCE state is intentionally in memory, matching the existing
single-replica OAuth constraint. A restart invalidates an unfinished attempt;
reopening Home creates a new one. Never put callback query strings in logs or
tickets because they contain short-lived codes and state.

## Event and action semantics

The list request uses `primary`, `timeMin=now`, expanded recurring instances,
`orderBy=startTime`, `showDeleted=false`, `maxAttendees=1` (the authenticated
participant only), and pages through at most 100 raw events (five pages of 20)
until it has five relevant events. Cancelled and
self-declined events are excluded. Remaining events are sorted by actual
instant, then event ID. Timed events use Slack date tokens so Slack renders the
viewer's time zone; all-day Google end dates are treated as exclusive.

Each event can include location, organizer, the user's attendee response, a
validated HTTPS meeting link, and a Google Calendar link. Organizers and events
without a `self` attendee do not get RSVP buttons. The current response is
shown and its corresponding button is omitted.

Before an RSVP, the bot fetches the event again. It sends an ETag-guarded
`events.patch` containing `attendeesOmitted: true` and only the self attendee's
email and new `responseStatus`; this is Google Calendar's partial-attendee
semantics and does not replace unrelated attendees or event fields. Notification
emails are suppressed with `sendUpdates=none`. A stale, deleted, or already
changed event triggers a fresh Home load. Safe list GETs retry once for 429/5xx;
token exchanges and PATCH writes are not blindly retried. The Home refreshes
after every action.

## Representative Block Kit payload

Slack Home views do not have a top-level notification fallback `text` field.
The sections carry complete readable text, links have labels, and every button
has an `accessibility_label`.

```json
{
  "type": "home",
  "blocks": [
    {
      "type": "header",
      "text": { "type": "plain_text", "text": "Your upcoming calendar", "emoji": true }
    },
    {
      "type": "context",
      "elements": [
        { "type": "mrkdwn", "text": "Your next five non-declined events, in your Slack time zone." }
      ]
    },
    {
      "type": "section",
      "block_id": "event_7b995032ec0d9f0d",
      "text": {
        "type": "mrkdwn",
        "text": "*Design review*\n:clock3: <!date^1788278400^{date_short_pretty} at {time}|2026-09-01 09:00>–<!date^1788280200^{time}|09:30>\n:round_pushpin: Room 3\n:bust_in_silhouette: Organized by Ada\n:incoming_envelope: *Awaiting your response*\n<https://meet.google.com/abc-defg-hij|Join meeting>  •  <https://calendar.google.com/event?eid=example|Open in Google Calendar>"
      }
    },
    {
      "type": "actions",
      "block_id": "rsvp_7b995032ec0d9f0d",
      "elements": [
        {
          "type": "button",
          "text": { "type": "plain_text", "text": "Accept", "emoji": true },
          "action_id": "calendar_rsvp_accept",
          "value": "BASE64URL_IDENTITY_BOUND_ACTION",
          "style": "primary",
          "accessibility_label": "Accept this calendar invitation"
        },
        {
          "type": "button",
          "text": { "type": "plain_text", "text": "Maybe", "emoji": true },
          "action_id": "calendar_rsvp_tentative",
          "value": "BASE64URL_IDENTITY_BOUND_ACTION",
          "accessibility_label": "Respond maybe to this calendar invitation"
        },
        {
          "type": "button",
          "text": { "type": "plain_text", "text": "Decline", "emoji": true },
          "action_id": "calendar_rsvp_decline",
          "value": "BASE64URL_IDENTITY_BOUND_ACTION",
          "style": "danger",
          "accessibility_label": "Decline this calendar invitation"
        }
      ]
    }
  ]
}
```

## Human end-to-end checks

CI cannot prove Keycard policy, one-time Google consent, Slack app settings, or
live Calendar writes. In a test workspace/account:

1. Open Home before consent and verify the authorization state/link.
2. Authorize through the in-process PKCE flow and verify Home refreshes
   automatically. Compare ordering, timed and all-day display, links, organizer,
   location, and statuses with Google.
3. Test accept, maybe, and decline on disposable invitations. Confirm only the
   signed-in attendee changes and the Home refreshes.
4. Repeat with a second Slack/Keycard user and verify no event or action crosses
   identities.
5. Revoke Google access and verify Home returns to reauthorization. Exercise a
   stale/deleted event and, if practical, a rate-limited test environment.

Never record tokens, authorization callback URLs, client secrets, or decrypted
credential records in test evidence.
