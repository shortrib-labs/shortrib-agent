use anyhow::Result;
use rig::{
    client::{AgentClientExt, ProviderClient},
    completion::{Message, Prompt},
    providers::openai,
    tool::ToolContext,
    tool::server::ToolServer,
};

use crate::{
    google_calendar::{CONNECT_TOOL, CalendarSession, GoogleCalendarMcp},
    state::UserKey,
};

const MODEL: &str = "gpt-5.5";
const MAX_TURNS: usize = 10;

pub struct Agent {
    agent: rig::agent::Agent,
}

pub struct ChatResponse {
    /// The assistant's reply, to be posted where the user asked.
    pub message: String,
    /// A Google Calendar authorization link the user must open before their
    /// calendar tools work. Sent to the user privately.
    pub authorization_url: Option<String>,
}

impl Agent {
    pub async fn new() -> Result<Self> {
        let openai_client = openai::Client::from_env()?;
        // The tool server starts with only the connect tool; the calendar
        // tools are added to it by `GoogleCalendarMcp` once the first user
        // authorizes, and the agent picks them up on its next request.
        let tools = ToolServer::new().run();
        GoogleCalendarMcp::from_env(tools.clone()).await?;

        let agent = openai_client
            .agent(MODEL)
            .preamble(&preamble())
            .default_max_turns(MAX_TURNS)
            .tool_server_handle(tools)
            .build();

        Ok(Self { agent })
    }

    pub async fn chat(
        &self,
        user: UserKey,
        message: &str,
        history: &mut Vec<Message>,
    ) -> Result<ChatResponse> {
        let session = CalendarSession::new(user);
        let mut context = ToolContext::new();
        context.insert(session.clone());

        let response = self
            .agent
            .prompt(message)
            .history(history.clone())
            .tool_context(context)
            .extended_details()
            .await?;
        if let Some(messages) = response.messages {
            history.extend(messages);
        }

        Ok(ChatResponse {
            message: response.output,
            authorization_url: session.authorization_url(),
        })
    }
}

fn preamble() -> String {
    format!(
        "You are a helpful assistant with access to the user's Google Calendar once they have \
         connected it. If the user asks about their calendar and no calendar tools are \
         available, or a calendar tool reports that the user is not connected, call \
         `{CONNECT_TOOL}` and tell the user to open the private authorization link they were \
         sent, then ask again."
    )
}
