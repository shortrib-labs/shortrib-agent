// The broad "allow access to the MCP server" grant. Resource-scoped with NO
// principal constraint, so it matches every principal that touches the proxy:
// the shortrib-agent app clients, the user, AND the Zone MCP Gateway service.
// The gateway is a Keycard "Service", which is not an entity type in the Cedar
// schema, so it cannot be matched by `principal is Keycard::Application` or by
// `principal.identifier` -- only a principal-unconstrained permit reaches it.
// This is what authorizes the gateway's brokering token-exchange for mcp/v1.
@id("permit-mcp-proxy-access")
permit (
  principal,
  action,
  resource
)
when {
  resource.identifier == "https://google-calendar-mcp-hrlw48pl49z9x55u3a2kremvhe.mcp.gateway.context.cloud/mcp/v1"
};
