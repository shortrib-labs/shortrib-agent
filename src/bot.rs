use slack_morphism::prelude::*;
use std::sync::Arc;

use crate::agent::Agent;
use crate::state::{ConversationHistory, ConversationKey, UserKey, UserStateStore};

type BotResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub struct SlackBot {
    agent: Agent,
    users: UserStateStore,
    bot_token: SlackApiToken,
    app_token: SlackApiToken,
}

impl SlackBot {
    pub fn new(agent: Agent) -> Result<Self, String> {
        Ok(Self {
            agent,
            users: UserStateStore::default(),
            bot_token: SlackApiToken::new(config_env_var("SLACK_BOT_TOKEN")?.into()),
            app_token: SlackApiToken::new(config_env_var("SLACK_APP_TOKEN")?.into()),
        })
    }

    pub async fn start(self) -> BotResult<()> {
        let client = Arc::new(SlackClient::new(SlackClientHyperConnector::new()?));
        let bot = Arc::new(self);

        let socket_mode_callbacks = SlackSocketModeListenerCallbacks::new()
            .with_command_events(Self::command_callback)
            .with_interaction_events(Self::interaction_callback)
            .with_push_events(Self::push_event_callback);

        let listener_environment = Arc::new(
            SlackClientEventsListenerEnvironment::new(client.clone())
                .with_user_state(Arc::clone(&bot))
                .with_error_handler(Self::handle_error),
        );

        let socket_mode_listener = SlackClientSocketModeListener::new(
            &SlackClientSocketModeConfig::new(),
            listener_environment,
            socket_mode_callbacks,
        );

        socket_mode_listener.listen_for(&bot.app_token).await?;
        socket_mode_listener.serve().await;

        Ok(())
    }

    async fn command_callback(
        event: SlackCommandEvent,
        _client: Arc<SlackHyperClient>,
        states: SlackClientEventsUserState,
    ) -> BotResult<SlackCommandEventResponse> {
        Self::from_state(states).await?.handle_command(event).await
    }

    async fn interaction_callback(
        event: SlackInteractionEvent,
        _client: Arc<SlackHyperClient>,
        states: SlackClientEventsUserState,
    ) -> BotResult<()> {
        Self::from_state(states)
            .await?
            .handle_interaction(event)
            .await
    }

    async fn push_event_callback(
        event: SlackPushEventCallback,
        client: Arc<SlackHyperClient>,
        states: SlackClientEventsUserState,
    ) -> BotResult<()> {
        Self::from_state(states)
            .await?
            .handle_push_event(event, client)
            .await
    }

    async fn from_state(states: SlackClientEventsUserState) -> Result<Arc<Self>, std::io::Error> {
        states
            .read()
            .await
            .get_user_state::<Arc<Self>>()
            .cloned()
            .ok_or_else(|| std::io::Error::other("SlackBot state is not configured"))
    }

    async fn handle_command(
        &self,
        event: SlackCommandEvent,
    ) -> BotResult<SlackCommandEventResponse> {
        let command = event.command.0;
        println!("Unhandled command: {command}");

        Ok(SlackCommandEventResponse::new(
            SlackMessageContent::new()
                .with_text(format!("The `{command}` command is not implemented yet.")),
        ))
    }

    async fn handle_push_event(
        self: Arc<Self>,
        event: SlackPushEventCallback,
        client: Arc<SlackHyperClient>,
    ) -> BotResult<()> {
        if let SlackEventCallbackBody::Message(message) = event.event {
            if message.sender.bot_id.is_some() {
                return Ok(());
            }

            let Some(user_id) = message.sender.user.clone() else {
                return Ok(());
            };
            let Some(channel_id) = message.origin.channel.clone() else {
                return Ok(());
            };
            let conversation = self
                .users
                .conversation(
                    UserKey::new(event.team_id, user_id),
                    ConversationKey::new(channel_id, message.origin.thread_ts.clone()),
                )
                .await;

            tokio::spawn(async move {
                if let Err(error) = self.chat(message, client, conversation).await {
                    eprintln!("Chat error: {error:#}");
                }
            });
        }

        Ok(())
    }

    async fn chat(
        &self,
        event: SlackMessageEvent,
        client: Arc<SlackHyperClient>,
        history: ConversationHistory,
    ) -> BotResult<()> {
        let Some(channel) = event.origin.channel else {
            return Ok(());
        };
        let Some(message) = event.content.and_then(|content| content.text) else {
            return Ok(());
        };

        let mut history = history.lock().await;
        let previous_history_len = history.len();
        let response = self.agent.chat(&message, &mut history).await?;
        let request = SlackApiChatPostMessageRequest::new(
            channel,
            SlackMessageContent::new().with_text(response),
        );

        if let Err(error) = client
            .open_session(&self.bot_token)
            .chat_post_message(&request)
            .await
        {
            history.truncate(previous_history_len);
            return Err(error.into());
        }

        Ok(())
    }

    async fn handle_interaction(&self, event: SlackInteractionEvent) -> BotResult<()> {
        println!("Unhandled interaction event: {event:#?}");
        Ok(())
    }

    fn handle_error(
        error: Box<dyn std::error::Error + Send + Sync>,
        _client: Arc<SlackHyperClient>,
        _states: SlackClientEventsUserState,
    ) -> HttpStatusCode {
        eprintln!("Slack listener error: {error:#}");

        // Acknowledge the event so Slack does not retry it.
        HttpStatusCode::OK
    }
}

fn config_env_var(name: &str) -> Result<String, String> {
    std::env::var(name).map_err(|error| format!("{name}: {error}"))
}
