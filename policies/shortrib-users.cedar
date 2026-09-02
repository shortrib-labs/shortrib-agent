// Replaces the platform "default-user-grants" (all users -> all resources),
// scoped to the tenant. Permits @shortrib.io users direct access to resources.
// This is what let the user side evaluate "Allow" on the Calendar / MCP proxy;
// without it, removing default-user-grants denies the user's own access.
@id("permit-shortrib-users")
permit (
  principal is Keycard::User,
  action,
  resource
)
when {
  principal has email &&
  principal.email like "*@shortrib.io"
};
