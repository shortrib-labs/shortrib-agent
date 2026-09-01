# shortrib-agent

A Slack agent with dynamically loaded Google Calendar tools from the Google
Cloud MCP gateway. Calendar authorization uses the Keycard RMCP OAuth
composition and is isolated by Slack team and user.

Copy `.env.example` to `.env`, fill in the Slack and OpenAI credentials, and
follow [the Google Calendar operations guide](docs/google-calendar.md) for
callback routing, encrypted OAuth storage, deployment constraints, and live
verification.

```sh
just check
just verify-calendar-metadata
just dev
```
