# shortrib-agent

A Slack agent with dynamically loaded Google Calendar tools from the Google
Cloud MCP gateway. Calendar authorization uses the Keycard RMCP OAuth
composition, is isolated by Slack team and user, and can persist credentials
in encrypted storage across restarts.

## Configuration

The agent reads a local `.env` file when present. Copy `.env.example` to `.env`
and configure the Slack, OpenAI, Calendar callback, and OAuth storage values.
For deployment constraints, encrypted credential management, and live
verification, follow [the Google Calendar operations guide](docs/google-calendar.md).

Calendar tools are discovered and added to the running agent after a user
authorizes Google Calendar. Each tool call resolves the requesting user's own
authenticated MCP connection; restored users reconnect lazily after a restart.

## Calendar messages in Slack

Calendar MCP event results render directly as Slack Block Kit without a
duplicate visible assistant summary. Each event includes its title,
viewer-local date and time (or an all-day date range), location, a sanitized
and truncated description, and available meeting and Google Calendar links.

The renderer provides explicit empty and error states. To stay within Slack's
limits, it renders at most 20 distinct events, caps section text at 3,000
characters, and retains the assistant response as the message's plain-text
accessibility and notification fallback.

## Development

```sh
just fmt
just check
just build
just verify-calendar-metadata
just dev
```
