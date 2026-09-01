use std::sync::Arc;

use anyhow::Result;
use rig::{
    client::{AgentClientExt, ProviderClient},
    completion::{Message, Prompt},
    providers::openai,
    tool::{DynamicTool, ToolContext},
};
use tokio::sync::OnceCell;

use crate::{
    google_calendar::{CalendarSession, GoogleCalendarMcp},
    state::UserKey,
};

const MODEL: &str = "gpt-5.5";
const PREAMBLE: &str = "You are a helpful assistant with access to the user's Google Calendar.";
const MAX_TURNS: usize = 10;
const AUTHORIZE_FIRST: &str = "I need access to your Google Calendar before I can help. \
    I've sent you a private link to authorize it; once you have, send your message again.";

pub struct Agent {
    openai_client: openai::Client,
    google_calendar: Arc<GoogleCalendarMcp>,
    /// Built once, from the first authorized user's tool list. The Google
    /// Calendar MCP server only lists tools for a caller with a Google
    /// grant, so the agent cannot be assembled at startup.
    agent: OnceCell<rig::agent::Agent>,
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
        Ok(Self {
            openai_client: openai::Client::from_env()?,
            google_calendar: GoogleCalendarMcp::from_env().await?,
            agent: OnceCell::new(),
        })
    }

    pub async fn chat(
        &self,
        user: UserKey,
        message: &str,
        history: &mut Vec<Message>,
    ) -> Result<ChatResponse> {
        let agent = match self.agent.get() {
            Some(agent) => agent,
            None => match self.google_calendar.tools(&user).await? {
                Ok(tools) => self.agent.get_or_init(|| async { self.build(tools) }).await,
                Err(authorization_url) => {
                    return Ok(ChatResponse {
                        message: AUTHORIZE_FIRST.to_owned(),
                        authorization_url: Some(authorization_url),
                    });
                }
            },
        };

        let session = CalendarSession::new(user);
        let mut context = ToolContext::new();
        context.insert(session.clone());

        let response = agent
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

    fn build(&self, tools: Vec<DynamicTool>) -> rig::agent::Agent {
        self.openai_client
            .agent(MODEL)
            .preamble(PREAMBLE)
            .dynamic_tools(tools)
            .default_max_turns(MAX_TURNS)
            .build()
    }
}
