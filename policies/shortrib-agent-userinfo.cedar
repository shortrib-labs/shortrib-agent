// Keycard is default-deny and the platform default policies have been removed
// from this set, so UserInfo access needs an explicit permit. The shortrib-agent
// application reads OpenID Connect UserInfo to confirm the linking user's email
// during the App Home flow (and the MCP tokens carry userinfo as a resource).
// Deliberately NOT scoped to an @shortrib.io subject: UserInfo is how the email
// is verified in the first place, and the token already binds the call to the
// authenticated user.
@id("permit-shortrib-agent-userinfo")
permit (
  principal is Keycard::Application,
  action,
  resource
)
when {
  principal.identifier == "shortrib-agent" &&
  resource.identifier == "https://hrlw48pl49z9x55u3a2kremvhe.keycard.cloud/openid/connect/userinfo"
};
