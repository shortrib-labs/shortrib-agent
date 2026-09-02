// Keycard is default-deny, so every allowed flow needs an explicit permit.
// This grants the confidential shortrib-agent application permission to
// impersonate an @shortrib.io user against the Google Calendar API (the App
// Home absent-user flow). No forbid is used; the permits are the allow-list.
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
