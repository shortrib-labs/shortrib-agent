// Guardrail scoped to the Calendar resources only: when shortrib-agent
// touches them, it may only act as itself against the MCP proxy, or
// impersonate an @shortrib.io user against the Calendar API with its
// client secret. Platform flows (OIDC sign-in, UserInfo) are untouched.
forbid (
  principal is Keycard::Application,
  action,
  resource
)
when {
  principal.identifier == "shortrib-agent" &&
  [
    "https://www.googleapis.com/calendar/v3",
    "https://google-calendar-mcp-hrlw48pl49z9x55u3a2kremvhe.mcp.gateway.context.cloud/mcp/v1"
  ].contains(resource.identifier)
}
unless {
  principal has credential_type &&
  principal.credential_type == Keycard::CredentialType::"password" &&
  (
    (
      context.on_behalf == false &&
      resource.identifier == "https://google-calendar-mcp-hrlw48pl49z9x55u3a2kremvhe.mcp.gateway.context.cloud/mcp/v1"
    ) ||
    (
      resource.identifier == "https://www.googleapis.com/calendar/v3" &&
      context has subject &&
      context.subject has email &&
      context.subject.email like "*@shortrib.io"
    )
  )
};
