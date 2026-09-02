// Keycard is default-deny, so every allowed flow needs an explicit permit.
// Two principals need access to the Google Calendar MCP proxy:
//   - the shortrib-agent application, on behalf of an @shortrib.io user
//     (the pre-registered public MCP login client), and
//   - the Zone MCP Gateway service, which performs the delegation
//     token-exchange when a client calls the proxy. The gateway is a Service
//     principal, so it is matched by identifier without an `is Application`
//     head (which would exclude it). Without the gateway branch that exchange
//     is denied ("Client is not allowed to exchange token for this resource")
//     and MCP initialize loops through login.
// No forbid is used; the permits are the allow-list.
@id("permit-shortrib-agent-mcp-proxy")
permit (
  principal,
  action,
  resource
)
when {
  resource.identifier == "https://google-calendar-mcp-hrlw48pl49z9x55u3a2kremvhe.mcp.gateway.context.cloud/mcp/v1" &&
  (
    (
      principal is Keycard::Application &&
      principal.identifier == "shortrib-agent" &&
      context has subject &&
      context.subject has email &&
      context.subject.email like "*@shortrib.io"
    )
    ||
    principal.identifier == "https://mcp.gateway.context.cloud"
  )
};
