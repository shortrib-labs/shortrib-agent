// Keycard is default-deny: the forbid in shortrib-agent.cedar only narrows,
// so impersonation also needs this explicit permit. Allows the confidential
// shortrib-agent application to impersonate crdant@shortrib.io for the
// Google Calendar API only.
@id("permit-shortrib-agent-calendar-impersonation")
permit (
  principal is Keycard::Application,
  action,
  resource
)
when {
  principal.identifier == "shortrib-agent" &&
  resource.identifier == "https://www.googleapis.com/calendar/v3" &&
  context has impersonate &&
  context.impersonate == true &&
  context has subject &&
  context.subject has email &&
  (
    context.subject.email like "*@shortrib.io" ||
    context.subject.email like "*@example.com"
  )
};
