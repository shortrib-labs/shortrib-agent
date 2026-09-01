use chrono::{DateTime, Duration, NaiveDate};
use serde::Deserialize;
use slack_morphism::prelude::*;

use crate::google_calendar::CalendarToolOutput;

const MAX_MESSAGE_TEXT: usize = 4_000;
const MAX_SECTION_TEXT: usize = 3_000;
const MAX_EVENTS: usize = 20;
const MAX_ERRORS: usize = 3;
const MAX_TITLE: usize = 250;
const MAX_LOCATION: usize = 500;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CalendarEvent {
    #[serde(default)]
    id: String,
    #[serde(default)]
    summary: String,
    description: Option<String>,
    location: Option<String>,
    html_link: Option<String>,
    conference_url: Option<String>,
    start: Option<EventTime>,
    end: Option<EventTime>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EventTime {
    date: Option<String>,
    date_time: Option<String>,
}

pub(crate) fn calendar_message(
    response: String,
    outputs: &[CalendarToolOutput],
) -> SlackMessageContent {
    let fallback = truncate(&response, MAX_MESSAGE_TEXT);
    if outputs.is_empty() {
        return SlackMessageContent::new().with_text(fallback);
    }

    let mut events = Vec::new();
    let mut saw_empty_list = false;
    let mut errors = Vec::new();
    for output in outputs {
        if let Some(error) = &output.error {
            errors.push(error.as_str());
            continue;
        }
        let Some(payload) = &output.payload else {
            continue;
        };
        if let Some(items) = payload.get("events").and_then(|value| value.as_array()) {
            saw_empty_list |= items.is_empty() && is_list_events(&output.tool_name);
            events.extend(items.iter().filter_map(parse_event));
        } else if let Some(event) = parse_event(payload) {
            events.push(event);
        }
    }

    deduplicate_events(&mut events);
    if events.is_empty() && !saw_empty_list && errors.is_empty() {
        return SlackMessageContent::new().with_text(fallback);
    }

    let mut blocks = vec![
        SlackHeaderBlock::new(SlackBlockPlainText::new("Google Calendar".to_owned()).into()).into(),
    ];

    if events.is_empty() && saw_empty_list {
        blocks.push(section(":calendar: No calendar events found.".to_owned()));
    }

    let event_count = events.len();
    for event in events.iter().take(MAX_EVENTS) {
        blocks.push(section(render_event(event)));
    }
    if event_count > MAX_EVENTS {
        blocks.push(context(format!(
            "Showing the first {MAX_EVENTS} of {event_count} events."
        )));
    }

    for error in errors.iter().take(MAX_ERRORS) {
        blocks.push(section(format!(
            ":warning: *Google Calendar error*\n{}",
            truncate(&escape(error), MAX_SECTION_TEXT - 40)
        )));
    }
    if errors.len() > MAX_ERRORS {
        blocks.push(context(format!(
            "{} additional calendar errors were omitted.",
            errors.len() - MAX_ERRORS
        )));
    }

    SlackMessageContent::new()
        .with_text(if fallback.is_empty() {
            "Google Calendar results".to_owned()
        } else {
            fallback
        })
        .with_blocks(blocks)
}

fn parse_event(value: &serde_json::Value) -> Option<CalendarEvent> {
    let event: CalendarEvent = serde_json::from_value(value.clone()).ok()?;
    if event.start.is_none() && event.end.is_none() {
        return None;
    }
    Some(event)
}

fn is_list_events(tool_name: &str) -> bool {
    tool_name
        .rsplit(['.', '/', ':'])
        .next()
        .is_some_and(|name| name == "list_events" || name == "list-events")
}

fn deduplicate_events(events: &mut Vec<CalendarEvent>) {
    let mut seen = std::collections::HashSet::new();
    events.retain(|event| event.id.is_empty() || seen.insert(event.id.clone()));
}

fn render_event(event: &CalendarEvent) -> String {
    let title = truncate(
        &escape(if event.summary.trim().is_empty() {
            "Untitled event"
        } else {
            event.summary.trim()
        }),
        MAX_TITLE,
    );
    let mut text = format!("*{title}*\n:calendar: {}", render_time(event));

    if let Some(location) = event
        .location
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        text.push_str("\n:round_pushpin: ");
        text.push_str(&truncate(&escape(location.trim()), MAX_LOCATION));
    }

    let links = render_links(event);
    let description = event
        .description
        .as_deref()
        .map(clean_description)
        .filter(|value| !value.is_empty());
    if let Some(description) = description {
        let reserved = links.chars().count() + 2;
        let available = MAX_SECTION_TEXT
            .saturating_sub(text.chars().count())
            .saturating_sub(reserved);
        if available > 1 {
            text.push('\n');
            text.push_str(&truncate(&description, available));
        }
    }
    if !links.is_empty() {
        text.push('\n');
        text.push_str(&links);
    }

    truncate(&text, MAX_SECTION_TEXT)
}

fn render_time(event: &CalendarEvent) -> String {
    let Some(start) = &event.start else {
        return "Time not provided".to_owned();
    };

    if let Some(start_date) = start.date.as_deref().and_then(parse_date) {
        let last_date = event
            .end
            .as_ref()
            .and_then(|end| end.date.as_deref())
            .and_then(parse_date)
            .and_then(|end| (end > start_date).then_some(end - Duration::days(1)))
            .unwrap_or(start_date);
        return if last_date > start_date {
            format!(
                "{}–{} (all day)",
                start_date.format("%b %-d, %Y"),
                last_date.format("%b %-d, %Y")
            )
        } else {
            format!("{} (all day)", start_date.format("%b %-d, %Y"))
        };
    }

    let Some(start_raw) = start.date_time.as_deref() else {
        return "Time not provided".to_owned();
    };
    let Ok(start_time) = DateTime::parse_from_rfc3339(start_raw) else {
        return escape(start_raw);
    };
    let start_token = slack_date(&start_time, "{date_short_pretty} at {time}");
    let Some(end_raw) = event.end.as_ref().and_then(|end| end.date_time.as_deref()) else {
        return start_token;
    };
    let Ok(end_time) = DateTime::parse_from_rfc3339(end_raw) else {
        return format!("{start_token}–{}", escape(end_raw));
    };
    let format = if start_time.date_naive() == end_time.date_naive() {
        "{time}"
    } else {
        "{date_short_pretty} at {time}"
    };
    format!("{start_token}–{}", slack_date(&end_time, format))
}

fn slack_date(time: &DateTime<chrono::FixedOffset>, format: &str) -> String {
    format!(
        "<!date^{}^{}|{}>",
        time.timestamp(),
        format,
        time.format("%b %-d, %Y at %-I:%M %P %:z")
    )
}

fn parse_date(value: &str) -> Option<NaiveDate> {
    value
        .get(..10)
        .and_then(|date| NaiveDate::parse_from_str(date, "%Y-%m-%d").ok())
}

fn render_links(event: &CalendarEvent) -> String {
    let mut links = Vec::new();
    if let Some(url) = event.conference_url.as_deref().and_then(safe_url) {
        links.push(format!("<{url}|Join meeting>"));
    }
    if let Some(url) = event.html_link.as_deref().and_then(safe_url) {
        links.push(format!("<{url}|Open in Google Calendar>"));
    }
    links.join(" · ")
}

fn safe_url(value: &str) -> Option<String> {
    let url = reqwest::Url::parse(value).ok()?;
    (url.scheme() == "https" || url.scheme() == "http").then(|| url.to_string())
}

fn clean_description(value: &str) -> String {
    let with_breaks = value
        .replace("<br>", "\n")
        .replace("<br/>", "\n")
        .replace("<br />", "\n")
        .replace("</p>", "\n");
    let mut plain = String::with_capacity(with_breaks.len());
    let mut inside_tag = false;
    for character in with_breaks.chars() {
        match character {
            '<' => inside_tag = true,
            '>' => inside_tag = false,
            _ if !inside_tag => plain.push(character),
            _ => {}
        }
    }
    let decoded = plain
        .replace("&nbsp;", " ")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&amp;", "&");
    escape(decoded.trim())
}

fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn truncate(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_owned();
    }
    if max_chars == 0 {
        return String::new();
    }
    let mut truncated = value
        .chars()
        .take(max_chars.saturating_sub(1))
        .collect::<String>();
    truncated.push('…');
    truncated
}

fn section(text: String) -> SlackBlock {
    SlackSectionBlock::new()
        .with_text(SlackBlockMarkDownText::new(text).into())
        .into()
}

fn context(text: String) -> SlackBlock {
    SlackContextBlock::new(vec![SlackBlockMarkDownText::new(text).into()]).into()
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::*;

    fn output(tool_name: &str, payload: Value) -> CalendarToolOutput {
        CalendarToolOutput {
            tool_name: tool_name.to_owned(),
            payload: Some(payload),
            error: None,
        }
    }

    fn block_texts(message: &SlackMessageContent) -> Vec<String> {
        serde_json::to_value(message.blocks.as_ref().unwrap())
            .unwrap()
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|block| block.pointer("/text/text").and_then(Value::as_str))
            .map(str::to_owned)
            .collect()
    }

    #[test]
    fn renders_timed_event_details_and_safe_links() {
        let message = calendar_message(
            "You have one event.".to_owned(),
            &[output(
                "list_events",
                json!({ "events": [{
                    "id": "one",
                    "summary": "Planning <review>",
                    "description": "Bring <b>metrics</b><br>and notes &amp; questions.",
                    "location": "Room A & B",
                    "htmlLink": "https://calendar.google.com/event?eid=one",
                    "conferenceUrl": "https://meet.google.com/abc-defg-hij",
                    "start": { "dateTime": "2026-09-03T10:00:00-07:00" },
                    "end": { "dateTime": "2026-09-03T11:00:00-07:00" }
                }] }),
            )],
        );

        let texts = block_texts(&message);
        assert_eq!(message.text.as_deref(), Some("You have one event."));
        assert_eq!(texts[0], "Google Calendar");
        assert!(!texts.iter().any(|text| text == "You have one event."));
        let event = &texts[1];
        assert!(event.contains("*Planning &lt;review&gt;*"));
        assert!(event.contains("<!date^1788454800^{date_short_pretty} at {time}|"));
        assert!(event.contains("<!date^1788458400^{time}|"));
        assert!(event.contains(":round_pushpin: Room A &amp; B"));
        assert!(event.contains("Bring metrics\nand notes &amp; questions."));
        assert!(event.contains("<https://meet.google.com/abc-defg-hij|Join meeting>"));
        assert!(event.contains("Open in Google Calendar"));
    }

    #[test]
    fn renders_single_and_multi_day_all_day_events() {
        let message = calendar_message(
            String::new(),
            &[output(
                "list_events",
                json!({ "events": [
                    {
                        "id": "one",
                        "summary": "Holiday",
                        "start": { "date": "2026-09-07T00:00:00Z" },
                        "end": { "date": "2026-09-08T00:00:00Z" }
                    },
                    {
                        "id": "two",
                        "summary": "Conference",
                        "start": { "date": "2026-09-10" },
                        "end": { "date": "2026-09-13" }
                    }
                ] }),
            )],
        );

        let texts = block_texts(&message);
        assert!(texts[1].contains("Sep 7, 2026 (all day)"));
        assert!(texts[2].contains("Sep 10, 2026–Sep 12, 2026 (all day)"));
    }

    #[test]
    fn renders_empty_results_and_tool_errors() {
        let empty = calendar_message(
            "Nothing scheduled.".to_owned(),
            &[output("calendar.list_events", json!({ "events": [] }))],
        );
        assert!(
            block_texts(&empty)
                .iter()
                .any(|text| text.contains("No calendar events found"))
        );

        let error = calendar_message(
            "I couldn't load the calendar.".to_owned(),
            &[CalendarToolOutput {
                tool_name: "get_event".to_owned(),
                payload: None,
                error: Some("Event <not found>".to_owned()),
            }],
        );
        assert!(
            block_texts(&error)
                .iter()
                .any(|text| text.contains("Event &lt;not found&gt;"))
        );
    }

    #[test]
    fn respects_block_and_text_limits() {
        let events = (0..75)
            .map(|index| {
                json!({
                    "id": index.to_string(),
                    "summary": "x".repeat(400),
                    "description": "d".repeat(5_000),
                    "start": { "dateTime": "2026-09-03T10:00:00Z" },
                    "end": { "dateTime": "2026-09-03T11:00:00Z" }
                })
            })
            .collect::<Vec<_>>();
        let message = calendar_message(
            "r".repeat(5_000),
            &[output("list_events", json!({ "events": events }))],
        );
        let json = serde_json::to_value(&message).unwrap();

        assert!(json["text"].as_str().unwrap().chars().count() <= MAX_MESSAGE_TEXT);
        assert!(json["blocks"].as_array().unwrap().len() <= 50);
        for text in block_texts(&message) {
            assert!(text.chars().count() <= MAX_SECTION_TEXT);
        }
        assert!(
            json.to_string()
                .contains("Showing the first 20 of 75 events")
        );
    }

    #[test]
    fn ignores_invalid_event_and_link_payloads() {
        let message = calendar_message(
            "Calendar updated.".to_owned(),
            &[output(
                "update_event",
                json!({
                    "id": "one",
                    "summary": "Updated",
                    "htmlLink": "javascript:alert(1)",
                    "start": { "dateTime": "not-a-date" },
                    "end": { "dateTime": "also-not-a-date" }
                }),
            )],
        );
        let serialized = serde_json::to_string(&message).unwrap();

        assert!(serialized.contains("not-a-date"));
        assert!(!serialized.contains("javascript"));
    }
}
