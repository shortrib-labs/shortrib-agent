# shortrib-agent

A Slack assistant backed by Rig and OpenAI, with dynamically loaded Google
Calendar tools provided by Google's MCP server.

## Configuration

The agent reads a local `.env` file when present. It requires:

- `OPENAI_API_KEY`
- `SLACK_BOT_TOKEN`
- `SLACK_APP_TOKEN`

Google Calendar uses these defaults unless overridden:

| Variable | Default |
| --- | --- |
| `GOOGLE_CALENDAR_MCP_URL` | The hosted Google Calendar MCP gateway |
| `GOOGLE_CALENDAR_MCP_REDIRECT_URI` | `http://127.0.0.1:3000/oauth/callback` |
| `GOOGLE_CALENDAR_MCP_CALLBACK_BIND` | `127.0.0.1:3000` |

The redirect URI registered with the MCP OAuth client must route to the
callback bind address. Calendar tools are discovered and added to the running
agent after the first user authorizes Google Calendar. Each subsequent tool
call still resolves the requesting user's own authenticated MCP connection.

## Calendar messages in Slack

Successful Calendar MCP event results render as Slack Block Kit alongside the
assistant's summary. Each event includes its title, viewer-local date and time
(or an all-day date range), location, a sanitized and truncated description,
and available meeting and Google Calendar links.

The renderer also provides explicit empty and error states. To stay within
Slack's limits, it renders at most 20 distinct events, caps section text at
3,000 characters, and retains the assistant response as the message's plain
text accessibility and notification fallback.

## Development

```sh
just fmt
just check
just build
```
