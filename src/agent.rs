use anyhow::Result;
use rig::{
    client::{AgentClientExt, ProviderClient},
    completion::{Chat, Message},
    providers::openai,
};
use std::sync::Arc;

pub struct Agent {
    agent: Arc<rig::agent::Agent>,
}

impl Agent {
    pub async fn new() -> Result<Self> {
        // Initialize the OpenAI client and models
        let openai_client = openai::Client::from_env()?;

        let agent = Arc::new(
            openai_client
                .agent("gpt-5.5")
                .preamble("You are a helpful assistant.")
                .build(),
        );

        Ok(Self { agent })
    }

    pub async fn chat(&self, message: &str, history: &mut Vec<Message>) -> Result<String> {
        self.agent
            .chat(message, history)
            .await
            .map_err(anyhow::Error::from)
    }
}
