// Scoped replacement for the platform "default-app-delegation" (which permits
// ANY application to act on behalf of a user). This limits on-behalf delegation
// to the two applications that actually need it:
//   - shortrib-agent (the agent's own clients), and
//   - the Zone MCP Gateway (uid n0jkp9admylcgg9vbuu7z0xvxp), which performs the
//     brokering token-exchange for the Calendar MCP proxy.
// The gateway is a Keycard::Application (the "Service" label in audit logs is an
// actor classification, not the Cedar entity type). If the gateway branch does
// not take effect, VERIFY its exact `identifier` in the console -- a trailing
// slash or different form breaks the string match; fall back to the unrestricted
// platform default-app-delegation if needed.
@id("permit-app-delegation")
permit (
  principal is Keycard::Application,
  action,
  resource
)
when {
  context has on_behalf &&
  context.on_behalf == true &&
  (
    principal.identifier == "shortrib-agent" ||
    principal.identifier == "https://mcp.gateway.context.cloud"
  )
};
