use std::time::Duration;

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, NaiveDate, Utc};
use keycard::{Client as KeycardClient, Error as KeycardError, SecretString};
use reqwest::{StatusCode, Url};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use slack_morphism::prelude::SlackView;

use crate::state::UserKey;

const CALENDAR_API_RESOURCE: &str = "https://www.googleapis.com/calendar/v3";
const CALENDAR_EVENTS_SCOPE: &str = "https://www.googleapis.com/auth/calendar.events";
const PAGE_SIZE: u8 = 20;
const MAX_PAGES: usize = 5;
const EVENT_LIMIT: usize = 5;

pub(crate) const ACCEPT_ACTION: &str = "calendar_rsvp_accept";
pub(crate) const TENTATIVE_ACTION: &str = "calendar_rsvp_tentative";
pub(crate) const DECLINE_ACTION: &str = "calendar_rsvp_decline";

#[derive(Clone)]
pub(crate) struct CalendarHome {
    keycard: KeycardClient,
    http: reqwest::Client,
    reauthorization_url: Url,
}

impl CalendarHome {
    pub(crate) fn from_env() -> anyhow::Result<Self> {
        let issuer = required_env("KEYCARD_ISSUER")?;
        let client_id = required_env("KEYCARD_CLIENT_ID")?;
        let client_secret = required_env("KEYCARD_CLIENT_SECRET")?;
        let reauthorization_url = required_env("KEYCARD_CALENDAR_AUTHORIZATION_URL")?
            .parse::<Url>()
            .map_err(|_| anyhow::anyhow!("KEYCARD_CALENDAR_AUTHORIZATION_URL must be a URL"))?;
        if reauthorization_url.scheme() != "https" {
            anyhow::bail!("KEYCARD_CALENDAR_AUTHORIZATION_URL must use HTTPS");
        }

        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .user_agent("shortrib-agent/0.1")
            .build()?;
        let keycard = KeycardClient::builder(issuer)
            .basic_auth(client_id, client_secret)
            .build()?;
        Ok(Self {
            keycard,
            http,
            reauthorization_url,
        })
    }

    pub(crate) fn loading_view(&self) -> SlackView {
        state_view(
            ":calendar: Your calendar",
            "Loading your next events…",
            None,
        )
    }

    pub(crate) fn updating_view(&self) -> SlackView {
        state_view(
            ":calendar: Your calendar",
            "Updating your invitation response…",
            None,
        )
    }

    pub(crate) fn error_view(&self, error: &CalendarError) -> SlackView {
        match error {
            CalendarError::Authorization(message) => state_view(
                ":calendar: Connect Google Calendar",
                message,
                Some((&self.reauthorization_url, "Authorize Calendar")),
            ),
            CalendarError::RateLimited => state_view(
                ":calendar: Calendar is busy",
                "Google Calendar is temporarily rate-limiting requests. Reopen this tab in a moment.",
                None,
            ),
            CalendarError::Stale => state_view(
                ":calendar: Event changed",
                "That invitation changed or was deleted. Your calendar has been refreshed.",
                None,
            ),
            CalendarError::NotActionable => state_view(
                ":calendar: Response not changed",
                "This event no longer has an invitation you can respond to.",
                None,
            ),
            CalendarError::Unavailable => state_view(
                ":calendar: Calendar unavailable",
                "I couldn’t load Google Calendar right now. Reopen this tab to try again.",
                None,
            ),
        }
    }

    pub(crate) async fn upcoming_events(
        &self,
        user_identifier: &str,
    ) -> Result<Vec<CalendarEvent>, CalendarError> {
        let token = self.token_for(user_identifier).await?;
        let mut page_token: Option<String> = None;
        let mut events = Vec::new();

        for _ in 0..MAX_PAGES {
            let mut url = calendar_url(&["calendars", "primary", "events"]);
            {
                let mut query = url.query_pairs_mut();
                query
                    .append_pair("timeMin", &Utc::now().to_rfc3339())
                    .append_pair("singleEvents", "true")
                    .append_pair("orderBy", "startTime")
                    .append_pair("showDeleted", "false")
                    .append_pair("maxAttendees", "1")
                    .append_pair("maxResults", &PAGE_SIZE.to_string());
                if let Some(page_token) = &page_token {
                    query.append_pair("pageToken", page_token);
                }
            }

            let response = self.get_with_retry(url, &token).await?;
            let page: EventPage = response
                .json()
                .await
                .map_err(|_| CalendarError::Unavailable)?;
            events.extend(page.items.into_iter().filter(CalendarEvent::is_relevant));
            if events.len() >= EVENT_LIMIT {
                break;
            }
            let Some(next_page_token) = page.next_page_token else {
                break;
            };
            page_token = Some(next_page_token);
        }

        events.sort_by(|left, right| {
            left.sort_key()
                .cmp(&right.sort_key())
                .then_with(|| left.id.cmp(&right.id))
        });
        events.truncate(EVENT_LIMIT);
        Ok(events)
    }

    pub(crate) async fn respond(
        &self,
        user_identifier: &str,
        event_id: &str,
        response: InvitationResponse,
    ) -> Result<(), CalendarError> {
        let token = self.token_for(user_identifier).await?;
        let mut url = calendar_url(&["calendars", "primary", "events", event_id]);
        url.query_pairs_mut().append_pair("maxAttendees", "1");
        let current_response = self
            .http
            .get(url.clone())
            .bearer_auth(token.expose_secret())
            .send()
            .await
            .map_err(|_| CalendarError::Unavailable)?;
        let current_response = checked_response(current_response)?;
        let event: ActionEvent = current_response
            .json()
            .await
            .map_err(|_| CalendarError::Unavailable)?;
        if event.status.as_deref() == Some("cancelled") {
            return Err(CalendarError::Stale);
        }
        let attendee = event
            .self_attendee()
            .filter(|_| {
                !event
                    .organizer
                    .as_ref()
                    .is_some_and(|organizer| organizer.self_user)
            })
            .ok_or(CalendarError::NotActionable)?;
        if attendee.response_status.as_deref() == Some(response.google_value()) {
            return Ok(());
        }
        let email = attendee
            .email
            .as_deref()
            .ok_or(CalendarError::NotActionable)?;
        let etag = event.etag.as_deref().ok_or(CalendarError::Stale)?;

        let mut patch_url = url;
        patch_url
            .query_pairs_mut()
            .append_pair("sendUpdates", "none");
        let patch_response = self
            .http
            .patch(patch_url)
            .bearer_auth(token.expose_secret())
            .header(reqwest::header::IF_MATCH, etag)
            .json(&json!({
                "attendeesOmitted": true,
                "attendees": [{
                    "email": email,
                    "responseStatus": response.google_value()
                }]
            }))
            .send()
            .await
            .map_err(|_| CalendarError::Unavailable)?;
        checked_response(patch_response)?;
        Ok(())
    }

    pub(crate) fn events_view(&self, user: &UserKey, events: &[CalendarEvent]) -> SlackView {
        let mut blocks = vec![
            json!({
                "type": "header",
                "text": { "type": "plain_text", "text": "Your upcoming calendar", "emoji": true }
            }),
            json!({
                "type": "context",
                "elements": [{
                    "type": "mrkdwn",
                    "text": "Your next five non-declined events, in your Slack time zone."
                }]
            }),
        ];

        if events.is_empty() {
            blocks.push(json!({
                "type": "section",
                "text": { "type": "mrkdwn", "text": ":sparkles: *You’re all clear.*\nNo upcoming events were found." }
            }));
        }

        for (index, event) in events.iter().enumerate() {
            if index > 0 {
                blocks.push(json!({ "type": "divider" }));
            }
            blocks.push(json!({
                "type": "section",
                "block_id": event_block_id(event),
                "text": { "type": "mrkdwn", "text": event_text(event) }
            }));
            if event.is_actionable() {
                blocks.push(action_block(user, event));
            }
        }

        home_view(blocks)
    }

    async fn token_for(&self, user_identifier: &str) -> Result<SecretString, CalendarError> {
        self.keycard
            .impersonate(user_identifier, CALENDAR_API_RESOURCE)
            .scope(CALENDAR_EVENTS_SCOPE)
            .send()
            .await
            .map(|response| response.access_token)
            .map_err(CalendarError::from_keycard)
    }

    async fn get_with_retry(
        &self,
        url: Url,
        token: &SecretString,
    ) -> Result<reqwest::Response, CalendarError> {
        for attempt in 0..2 {
            let response = self
                .http
                .get(url.clone())
                .bearer_auth(token.expose_secret())
                .send()
                .await
                .map_err(|_| CalendarError::Unavailable)?;
            if attempt == 0
                && (response.status() == StatusCode::TOO_MANY_REQUESTS
                    || response.status().is_server_error())
            {
                let delay = response
                    .headers()
                    .get(reqwest::header::RETRY_AFTER)
                    .and_then(|value| value.to_str().ok())
                    .and_then(|value| value.parse::<u64>().ok())
                    .unwrap_or(1)
                    .min(2);
                tokio::time::sleep(Duration::from_secs(delay)).await;
                continue;
            }
            return checked_response(response);
        }
        Err(CalendarError::Unavailable)
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum CalendarError {
    #[error("Google Calendar authorization is required")]
    Authorization(&'static str),
    #[error("Google Calendar rate limit exceeded")]
    RateLimited,
    #[error("calendar event is stale or missing")]
    Stale,
    #[error("calendar event cannot be answered by this user")]
    NotActionable,
    #[error("Google Calendar is unavailable")]
    Unavailable,
}

impl CalendarError {
    fn from_keycard(error: KeycardError) -> Self {
        match error.as_oauth().map(|error| error.code.as_str()) {
            Some("interaction_required" | "invalid_grant") => Self::Authorization(
                "Authorize Google Calendar for this Slack account, then reopen this tab.",
            ),
            Some("access_denied") => Self::Authorization(
                "Calendar access is not allowed for this account. Reauthorize or ask your Keycard administrator to enable the Calendar dependency and impersonation policy.",
            ),
            _ => Self::Unavailable,
        }
    }
}

fn checked_response(response: reqwest::Response) -> Result<reqwest::Response, CalendarError> {
    match response.status() {
        status if status.is_success() => Ok(response),
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => Err(CalendarError::Authorization(
            "Google Calendar authorization is missing, revoked, or does not include the required calendar.events scope. Reauthorize, then reopen this tab.",
        )),
        StatusCode::NOT_FOUND | StatusCode::GONE | StatusCode::PRECONDITION_FAILED => {
            Err(CalendarError::Stale)
        }
        StatusCode::TOO_MANY_REQUESTS => Err(CalendarError::RateLimited),
        _ => Err(CalendarError::Unavailable),
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CalendarEvent {
    id: String,
    status: Option<String>,
    summary: Option<String>,
    location: Option<String>,
    html_link: Option<String>,
    hangout_link: Option<String>,
    start: EventTime,
    end: EventTime,
    organizer: Option<Person>,
    #[serde(default)]
    attendees: Vec<Attendee>,
    conference_data: Option<ConferenceData>,
}

impl CalendarEvent {
    fn is_relevant(&self) -> bool {
        self.status.as_deref() != Some("cancelled")
            && self
                .self_attendee()
                .and_then(|attendee| attendee.response_status.as_deref())
                != Some("declined")
    }

    fn self_attendee(&self) -> Option<&Attendee> {
        self.attendees.iter().find(|attendee| attendee.self_user)
    }

    fn is_actionable(&self) -> bool {
        self.self_attendee().is_some()
            && !self
                .organizer
                .as_ref()
                .is_some_and(|organizer| organizer.self_user)
    }

    fn sort_key(&self) -> i64 {
        self.start.sort_key()
    }

    fn meeting_url(&self) -> Option<&str> {
        self.hangout_link.as_deref().or_else(|| {
            self.conference_data
                .as_ref()?
                .entry_points
                .iter()
                .find_map(|entry| {
                    (entry.entry_point_type.as_deref() == Some("video"))
                        .then_some(entry.uri.as_deref())
                        .flatten()
                })
        })
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EventTime {
    date: Option<String>,
    date_time: Option<String>,
}

impl EventTime {
    fn sort_key(&self) -> i64 {
        self.date_time
            .as_deref()
            .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
            .map(|value| value.timestamp())
            .or_else(|| {
                self.date
                    .as_deref()
                    .and_then(|value| NaiveDate::parse_from_str(value, "%Y-%m-%d").ok())
                    .and_then(|value| value.and_hms_opt(0, 0, 0))
                    .map(|value| value.and_utc().timestamp())
            })
            .unwrap_or(i64::MAX)
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Person {
    email: Option<String>,
    display_name: Option<String>,
    #[serde(default, rename = "self")]
    self_user: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Attendee {
    email: Option<String>,
    response_status: Option<String>,
    #[serde(default, rename = "self")]
    self_user: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConferenceData {
    #[serde(default)]
    entry_points: Vec<EntryPoint>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EntryPoint {
    entry_point_type: Option<String>,
    uri: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct EventPage {
    #[serde(default)]
    items: Vec<CalendarEvent>,
    next_page_token: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ActionEvent {
    etag: Option<String>,
    status: Option<String>,
    organizer: Option<Person>,
    #[serde(default)]
    attendees: Vec<Attendee>,
}

impl ActionEvent {
    fn self_attendee(&self) -> Option<&Attendee> {
        self.attendees.iter().find(|attendee| attendee.self_user)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum InvitationResponse {
    Accepted,
    Tentative,
    Declined,
}

impl InvitationResponse {
    pub(crate) fn from_action_id(action_id: &str) -> Option<Self> {
        match action_id {
            ACCEPT_ACTION => Some(Self::Accepted),
            TENTATIVE_ACTION => Some(Self::Tentative),
            DECLINE_ACTION => Some(Self::Declined),
            _ => None,
        }
    }

    fn google_value(self) -> &'static str {
        match self {
            Self::Accepted => "accepted",
            Self::Tentative => "tentative",
            Self::Declined => "declined",
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct ActionValue {
    team_id: String,
    user_id: String,
    event_id: String,
}

impl ActionValue {
    fn new(user: &UserKey, event_id: &str) -> Self {
        Self {
            team_id: user.team_id().0.clone(),
            user_id: user.user_id().0.clone(),
            event_id: event_id.to_owned(),
        }
    }

    pub(crate) fn decode(value: &str) -> Option<Self> {
        let bytes = URL_SAFE_NO_PAD.decode(value).ok()?;
        serde_json::from_slice(&bytes).ok()
    }

    pub(crate) fn belongs_to(&self, user: &UserKey) -> bool {
        self.team_id == user.team_id().0 && self.user_id == user.user_id().0
    }

    pub(crate) fn event_id(&self) -> &str {
        &self.event_id
    }

    fn encode(&self) -> String {
        URL_SAFE_NO_PAD.encode(serde_json::to_vec(self).expect("action value is serializable"))
    }
}

fn action_block(user: &UserKey, event: &CalendarEvent) -> Value {
    let current = event
        .self_attendee()
        .and_then(|attendee| attendee.response_status.as_deref());
    let value = ActionValue::new(user, &event.id).encode();
    let mut elements = Vec::new();
    if current != Some("accepted") {
        elements.push(button(
            "Accept",
            ACCEPT_ACTION,
            &value,
            Some("primary"),
            "Accept this calendar invitation",
        ));
    }
    if current != Some("tentative") {
        elements.push(button(
            "Maybe",
            TENTATIVE_ACTION,
            &value,
            None,
            "Respond maybe to this calendar invitation",
        ));
    }
    if current != Some("declined") {
        elements.push(button(
            "Decline",
            DECLINE_ACTION,
            &value,
            Some("danger"),
            "Decline this calendar invitation",
        ));
    }
    json!({
        "type": "actions",
        "block_id": format!("rsvp_{}", short_hash(&event.id)),
        "elements": elements
    })
}

fn button(
    label: &str,
    action_id: &str,
    value: &str,
    style: Option<&str>,
    accessibility_label: &str,
) -> Value {
    let mut button = json!({
        "type": "button",
        "text": { "type": "plain_text", "text": label, "emoji": true },
        "action_id": action_id,
        "value": value,
        "accessibility_label": accessibility_label
    });
    if let Some(style) = style {
        button["style"] = json!(style);
    }
    button
}

fn event_text(event: &CalendarEvent) -> String {
    let title = escape_mrkdwn(event.summary.as_deref().unwrap_or("(No title)"));
    let mut lines = vec![format!("*{}*", truncate(&title, 240)), render_time(event)];
    if let Some(location) = event
        .location
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        lines.push(format!(
            ":round_pushpin: {}",
            truncate(&escape_mrkdwn(location), 300)
        ));
    }
    if let Some(organizer) = &event.organizer {
        let organizer = organizer
            .display_name
            .as_deref()
            .or(organizer.email.as_deref())
            .unwrap_or("Unknown organizer");
        lines.push(format!(
            ":bust_in_silhouette: Organized by {}",
            truncate(&escape_mrkdwn(organizer), 180)
        ));
    }
    if let Some(status) = event
        .self_attendee()
        .and_then(|attendee| attendee.response_status.as_deref())
    {
        lines.push(format!(
            "{} *{}*",
            status_icon(status),
            status_label(status)
        ));
    }
    let mut links = Vec::new();
    if let Some(url) = safe_https_url(event.meeting_url()) {
        links.push(format!("<{url}|Join meeting>"));
    }
    if let Some(url) = safe_https_url(event.html_link.as_deref()) {
        links.push(format!("<{url}|Open in Google Calendar>"));
    }
    if !links.is_empty() {
        lines.push(links.join("  •  "));
    }
    truncate(&lines.join("\n"), 2_950)
}

fn render_time(event: &CalendarEvent) -> String {
    if let Some(start) = event.start.date_time.as_deref()
        && let Ok(start) = DateTime::parse_from_rfc3339(start)
    {
        let start_epoch = start.timestamp();
        if let Some(end) = event.end.date_time.as_deref()
            && let Ok(end) = DateTime::parse_from_rfc3339(end)
        {
            return format!(
                ":clock3: <!date^{start_epoch}^{{date_short_pretty}} at {{time}}|{}>–<!date^{}^{{time}}|{}>",
                start.format("%Y-%m-%d %H:%M"),
                end.timestamp(),
                end.format("%H:%M")
            );
        }
    }

    let Some(start) = event
        .start
        .date
        .as_deref()
        .and_then(|date| NaiveDate::parse_from_str(date, "%Y-%m-%d").ok())
    else {
        return ":clock3: Time unavailable".to_owned();
    };
    let end = event
        .end
        .date
        .as_deref()
        .and_then(|date| NaiveDate::parse_from_str(date, "%Y-%m-%d").ok())
        .and_then(|date| date.pred_opt())
        .unwrap_or(start);
    if start == end {
        format!(":calendar: {} · All day", start.format("%a, %b %-d"))
    } else {
        format!(
            ":calendar: {}–{} · All day",
            start.format("%b %-d"),
            end.format("%b %-d")
        )
    }
}

fn state_view(title: &str, message: &str, link: Option<(&Url, &str)>) -> SlackView {
    let mut blocks = vec![
        json!({
            "type": "header",
            "text": { "type": "plain_text", "text": title, "emoji": true }
        }),
        json!({
            "type": "section",
            "text": { "type": "mrkdwn", "text": message }
        }),
    ];
    if let Some((url, label)) = link {
        blocks.push(json!({
            "type": "actions",
            "elements": [{
                "type": "button",
                "text": { "type": "plain_text", "text": label, "emoji": true },
                "url": url,
                "action_id": "calendar_authorize",
                "accessibility_label": "Authorize access to Google Calendar"
            }]
        }));
    }
    home_view(blocks)
}

fn home_view(blocks: Vec<Value>) -> SlackView {
    serde_json::from_value(json!({ "type": "home", "blocks": blocks }))
        .expect("App Home blocks must be valid Slack Block Kit")
}

fn calendar_url(segments: &[&str]) -> Url {
    let mut url = Url::parse(CALENDAR_API_RESOURCE).expect("Calendar API resource URL is valid");
    url.path_segments_mut()
        .expect("Calendar API URL can have path segments")
        .extend(segments);
    url
}

fn event_block_id(event: &CalendarEvent) -> String {
    format!("event_{}", short_hash(&event.id))
}

fn short_hash(value: &str) -> String {
    use sha2::{Digest as _, Sha256};
    let digest = Sha256::digest(value.as_bytes());
    digest[..8]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn safe_https_url(value: Option<&str>) -> Option<Url> {
    let url = value?.parse::<Url>().ok()?;
    (url.scheme() == "https").then_some(url)
}

fn status_icon(status: &str) -> &'static str {
    match status {
        "accepted" => ":white_check_mark:",
        "tentative" => ":large_yellow_circle:",
        "declined" => ":no_entry_sign:",
        _ => ":incoming_envelope:",
    }
}

fn status_label(status: &str) -> &'static str {
    match status {
        "accepted" => "Accepted",
        "tentative" => "Maybe",
        "declined" => "Declined",
        "needsAction" => "Awaiting your response",
        _ => "Invitation",
    }
}

fn escape_mrkdwn(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn truncate(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_owned();
    }
    let mut output: String = value.chars().take(max_chars.saturating_sub(1)).collect();
    output.push('…');
    output
}

fn required_env(name: &str) -> anyhow::Result<String> {
    std::env::var(name).map_err(|error| anyhow::anyhow!("{name}: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use slack_morphism::prelude::{SlackTeamId, SlackUserId};

    fn event(value: Value) -> CalendarEvent {
        serde_json::from_value(value).unwrap()
    }

    fn user() -> UserKey {
        UserKey::new(SlackTeamId("T123".into()), SlackUserId("U123".into()))
    }

    #[test]
    fn filters_cancelled_and_declined_events_and_sorts_instants() {
        let accepted = event(json!({
            "id": "later", "status": "confirmed", "summary": "Later",
            "start": { "dateTime": "2026-09-01T12:00:00-07:00" },
            "end": { "dateTime": "2026-09-01T13:00:00-07:00" },
            "attendees": [{ "self": true, "email": "me@example.com", "responseStatus": "accepted" }]
        }));
        let earlier = event(json!({
            "id": "earlier", "status": "confirmed", "summary": "Earlier",
            "start": { "dateTime": "2026-09-01T18:00:00Z" },
            "end": { "dateTime": "2026-09-01T18:30:00Z" }
        }));
        let declined = event(json!({
            "id": "declined", "status": "confirmed",
            "start": { "date": "2026-09-02" }, "end": { "date": "2026-09-03" },
            "attendees": [{ "self": true, "responseStatus": "declined" }]
        }));
        let cancelled = event(json!({
            "id": "cancelled", "status": "cancelled",
            "start": { "date": "2026-09-02" }, "end": { "date": "2026-09-03" }
        }));

        let mut relevant: Vec<_> = [accepted, earlier, declined, cancelled]
            .into_iter()
            .filter(CalendarEvent::is_relevant)
            .collect();
        relevant.sort_by_key(CalendarEvent::sort_key);

        assert_eq!(
            relevant
                .iter()
                .map(|event| event.id.as_str())
                .collect::<Vec<_>>(),
            ["earlier", "later"]
        );
    }

    #[test]
    fn renders_representative_timed_invitation_with_safe_actions() {
        let event = event(json!({
            "id": "evt/one", "etag": "etag-one", "status": "confirmed",
            "summary": "Design <review>", "location": "Room & Zoom",
            "htmlLink": "https://calendar.google.com/event?eid=one",
            "hangoutLink": "https://meet.google.com/abc-defg-hij",
            "start": { "dateTime": "2026-09-01T09:00:00-07:00", "timeZone": "America/Los_Angeles" },
            "end": { "dateTime": "2026-09-01T09:30:00-07:00", "timeZone": "America/Los_Angeles" },
            "organizer": { "displayName": "Ada", "email": "ada@example.com" },
            "attendees": [{ "self": true, "email": "me@example.com", "responseStatus": "needsAction" }]
        }));
        let view = home_view_for_test(&[event]);
        let payload = serde_json::to_value(view).unwrap();
        let serialized = payload.to_string();

        assert_eq!(payload["type"], "home");
        assert!(serialized.contains("Design &lt;review&gt;"));
        assert!(serialized.contains(ACCEPT_ACTION));
        assert!(serialized.contains(TENTATIVE_ACTION));
        assert!(serialized.contains(DECLINE_ACTION));
        assert!(serialized.contains("accessibility_label"));

        let encoded = payload["blocks"][3]["elements"][0]["value"]
            .as_str()
            .unwrap();
        let action = ActionValue::decode(encoded).unwrap();
        assert!(action.belongs_to(&user()));
        assert_eq!(action.event_id(), "evt/one");
        assert!(!action.belongs_to(&UserKey::new(
            SlackTeamId("T123".into()),
            SlackUserId("OTHER".into())
        )));
    }

    #[test]
    fn all_day_ranges_use_exclusive_google_end_date() {
        let event = event(json!({
            "id": "all-day", "status": "confirmed", "summary": "Offsite",
            "start": { "date": "2026-09-01" }, "end": { "date": "2026-09-04" }
        }));
        assert_eq!(render_time(&event), ":calendar: Sep 1–Sep 3 · All day");
    }

    #[test]
    fn current_response_is_shown_and_its_button_is_omitted() {
        let event = event(json!({
            "id": "accepted", "status": "confirmed", "summary": "Planning",
            "start": { "dateTime": "2026-09-01T09:00:00Z" },
            "end": { "dateTime": "2026-09-01T10:00:00Z" },
            "organizer": { "email": "organizer@example.com" },
            "attendees": [{ "self": true, "email": "me@example.com", "responseStatus": "accepted" }]
        }));
        let payload = serde_json::to_string(&home_view_for_test(&[event])).unwrap();
        assert!(payload.contains("Accepted"));
        assert!(!payload.contains(ACCEPT_ACTION));
        assert!(payload.contains(TENTATIVE_ACTION));
        assert!(payload.contains(DECLINE_ACTION));
    }

    fn home_view_for_test(events: &[CalendarEvent]) -> SlackView {
        let home = CalendarHome {
            keycard: KeycardClient::new("https://example.keycard.cloud").unwrap(),
            http: reqwest::Client::new(),
            reauthorization_url: Url::parse("https://example.com/authorize").unwrap(),
        };
        home.events_view(&user(), events)
    }
}
