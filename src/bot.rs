use slack_morphism::{errors::SlackClientError, prelude::*};
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::Mutex;

use crate::agent::{Agent, ChatResponse};
use crate::app_home::{ActionValue, CalendarError, CalendarHome, InvitationResponse};
use crate::calendar_blocks::calendar_message;
use crate::state::{ConversationHistory, ConversationKey, UserKey, UserStateStore};

type BotResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub struct SlackBot {
    agent: Agent,
    users: UserStateStore,
    bot_token: SlackApiToken,
    app_token: SlackApiToken,
    authorized_team: Option<SlackTeamId>,
    calendar_home: CalendarHome,
    home_revisions: Mutex<HashMap<UserKey, u64>>,
    handled_actions: Mutex<HashMap<String, Instant>>,
}

impl SlackBot {
    pub fn new(agent: Agent) -> Result<Self, String> {
        Ok(Self {
            agent,
            users: UserStateStore::default(),
            bot_token: SlackApiToken::new(config_env_var("SLACK_BOT_TOKEN")?.into()),
            app_token: SlackApiToken::new(config_env_var("SLACK_APP_TOKEN")?.into()),
            authorized_team: None,
            calendar_home: CalendarHome::from_env().map_err(|error| error.to_string())?,
            home_revisions: Mutex::new(HashMap::new()),
            handled_actions: Mutex::new(HashMap::new()),
        })
    }

    pub async fn start(mut self) -> BotResult<()> {
        let client = Arc::new(SlackClient::new(SlackClientHyperConnector::new()?));
        let auth = client.open_session(&self.bot_token).auth_test().await?;
        self.authorized_team = Some(auth.team_id);
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
        client: Arc<SlackHyperClient>,
        states: SlackClientEventsUserState,
    ) -> BotResult<()> {
        Self::from_state(states)
            .await?
            .handle_interaction(event, client)
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
        if !self.is_authorized_team(&event.team_id) {
            tracing::warn!("ignored Slack event for an unauthorized team");
            return Ok(());
        }

        match event.event {
            SlackEventCallbackBody::Message(message) => {
                if message.sender.bot_id.is_some() {
                    return Ok(());
                }

                let Some(user_id) = message.sender.user.clone() else {
                    return Ok(());
                };
                let Some(channel_id) = message.origin.channel.clone() else {
                    return Ok(());
                };
                let user_key = UserKey::new(event.team_id, user_id.clone());
                let conversation = self
                    .users
                    .conversation(
                        user_key.clone(),
                        ConversationKey::new(channel_id, message.origin.thread_ts.clone()),
                    )
                    .await;

                tokio::spawn(async move {
                    if let Err(error) = self
                        .chat(user_key, user_id, message, client, conversation)
                        .await
                    {
                        tracing::warn!(error = %error, "Slack chat request failed");
                    }
                });
            }
            SlackEventCallbackBody::AppHomeOpened(home)
                if home.tab.as_deref().is_none_or(|tab| tab == "home") =>
            {
                let user = UserKey::new(event.team_id, home.user);
                let revision = self.next_home_revision(&user).await;
                tokio::spawn(async move {
                    if let Err(error) = self.refresh_home(user, revision, client).await {
                        tracing::warn!(error = %error, "Slack App Home refresh failed");
                    }
                });
            }
            _ => {}
        }

        Ok(())
    }

    async fn chat(
        &self,
        user_key: UserKey,
        user_id: SlackUserId,
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

        let ChatResponse {
            message: response,
            authorization_url,
            calendar_outputs,
        } = self.agent.chat(user_key, &message, &mut history).await?;
        let session = client.open_session(&self.bot_token);
        let post_result = async {
            if let Some(authorization_url) = authorization_url {
                session
                    .chat_post_ephemeral(&SlackApiChatPostEphemeralRequest::new(
                        channel.clone(),
                        user_id,
                        SlackMessageContent::new().with_text(format!(
                            "Authorize Google Calendar to continue: {authorization_url}"
                        )),
                    ))
                    .await?;
            }
            session
                .chat_post_message(&SlackApiChatPostMessageRequest::new(
                    channel,
                    calendar_message(response, &calendar_outputs),
                ))
                .await?;
            Ok::<_, SlackClientError>(())
        }
        .await;

        if let Err(error) = post_result {
            history.truncate(previous_history_len);
            return Err(error.into());
        }

        Ok(())
    }

    async fn handle_interaction(
        self: Arc<Self>,
        event: SlackInteractionEvent,
        client: Arc<SlackHyperClient>,
    ) -> BotResult<()> {
        let SlackInteractionEvent::BlockActions(event) = event else {
            return Ok(());
        };
        if !self.is_authorized_team(&event.team.id) {
            tracing::warn!("ignored Slack interaction for an unauthorized team");
            return Ok(());
        }
        if !matches!(&event.container, SlackInteractionActionContainer::View(_))
            || !matches!(&event.view, Some(SlackView::Home(_)))
        {
            return Ok(());
        }
        let Some(slack_user) = event.user else {
            return Ok(());
        };
        let user = UserKey::new(event.team.id, slack_user.id);
        let Some(action) = event.actions.and_then(|actions| actions.into_iter().next()) else {
            return Ok(());
        };
        let Some(response) = InvitationResponse::from_action_id(&action.action_id.0) else {
            return Ok(());
        };
        let Some(value) = action.value.as_deref().and_then(ActionValue::decode) else {
            tracing::warn!("ignored malformed Calendar App Home action");
            return Ok(());
        };
        if !value.belongs_to(&user) {
            tracing::warn!("blocked cross-user Calendar App Home action replay");
            return Ok(());
        }
        let Some(action_ts) = action.action_ts else {
            return Ok(());
        };
        let replay_key = format!(
            "{}\0{}\0{}\0{}",
            user.team_id().0,
            user.user_id().0,
            action_ts.0,
            action.action_id.0
        );
        if !self.mark_action(replay_key).await {
            return Ok(());
        }

        let event_id = value.event_id().to_owned();
        let revision = self.next_home_revision(&user).await;
        tokio::spawn(async move {
            if let Err(error) = self
                .handle_calendar_response(user, event_id, response, revision, client)
                .await
            {
                tracing::warn!(error = %error, "Calendar invitation response failed");
            }
        });
        Ok(())
    }

    async fn refresh_home(
        &self,
        user: UserKey,
        revision: u64,
        client: Arc<SlackHyperClient>,
    ) -> BotResult<()> {
        self.publish_home(
            user.user_id().clone(),
            self.calendar_home.loading_view(),
            &client,
        )
        .await?;
        let result = match self.slack_user_identifier(&user, &client).await {
            Ok(identifier) => self.calendar_home.upcoming_events(&identifier).await,
            Err(error) => Err(error),
        };
        if !self.is_current_home_revision(&user, revision).await {
            return Ok(());
        }
        let view = match result {
            Ok(events) => self.calendar_home.events_view(&user, &events),
            Err(error) => self.calendar_home.error_view(&error),
        };
        self.publish_home(user.user_id().clone(), view, &client)
            .await
    }

    async fn handle_calendar_response(
        &self,
        user: UserKey,
        event_id: String,
        response: InvitationResponse,
        revision: u64,
        client: Arc<SlackHyperClient>,
    ) -> BotResult<()> {
        self.publish_home(
            user.user_id().clone(),
            self.calendar_home.updating_view(),
            &client,
        )
        .await?;
        let identifier = match self.slack_user_identifier(&user, &client).await {
            Ok(identifier) => identifier,
            Err(error) => {
                if self.is_current_home_revision(&user, revision).await {
                    self.publish_home(
                        user.user_id().clone(),
                        self.calendar_home.error_view(&error),
                        &client,
                    )
                    .await?;
                }
                return Ok(());
            }
        };

        let response_result = self
            .calendar_home
            .respond(&identifier, &event_id, response)
            .await;
        if let Err(error) = response_result
            && !matches!(error, CalendarError::Stale | CalendarError::NotActionable)
        {
            if self.is_current_home_revision(&user, revision).await {
                self.publish_home(
                    user.user_id().clone(),
                    self.calendar_home.error_view(&error),
                    &client,
                )
                .await?;
            }
            return Ok(());
        }

        let events = self.calendar_home.upcoming_events(&identifier).await;
        if !self.is_current_home_revision(&user, revision).await {
            return Ok(());
        }
        let view = match events {
            Ok(events) => self.calendar_home.events_view(&user, &events),
            Err(error) => self.calendar_home.error_view(&error),
        };
        self.publish_home(user.user_id().clone(), view, &client)
            .await
    }

    async fn slack_user_identifier(
        &self,
        user: &UserKey,
        client: &SlackHyperClient,
    ) -> Result<String, CalendarError> {
        let response = client
            .open_session(&self.bot_token)
            .users_info(&SlackApiUsersInfoRequest::new(user.user_id().clone()))
            .await
            .map_err(|_| CalendarError::Authorization(
                "I couldn’t verify your Slack email. Ask an administrator to grant the bot users:read and users:read.email, then reinstall it.",
            ))?;
        if response.user.id != *user.user_id()
            || response
                .user
                .team_id
                .as_ref()
                .is_some_and(|team| team != user.team_id())
        {
            return Err(CalendarError::Authorization(
                "Your Slack identity could not be verified for this workspace.",
            ));
        }
        response
            .user
            .profile
            .and_then(|profile| profile.email)
            .map(|email| email.0)
            .ok_or(CalendarError::Authorization(
                "Your Slack profile needs a verified email linked to the same Keycard user before Calendar can be shown.",
            ))
    }

    async fn publish_home(
        &self,
        user_id: SlackUserId,
        view: SlackView,
        client: &SlackHyperClient,
    ) -> BotResult<()> {
        client
            .open_session(&self.bot_token)
            .views_publish(&SlackApiViewsPublishRequest::new(user_id, view))
            .await?;
        Ok(())
    }

    fn is_authorized_team(&self, team_id: &SlackTeamId) -> bool {
        self.authorized_team.as_ref() == Some(team_id)
    }

    async fn next_home_revision(&self, user: &UserKey) -> u64 {
        let mut revisions = self.home_revisions.lock().await;
        let revision = revisions.entry(user.clone()).or_default();
        *revision = revision.wrapping_add(1);
        *revision
    }

    async fn is_current_home_revision(&self, user: &UserKey, revision: u64) -> bool {
        self.home_revisions.lock().await.get(user) == Some(&revision)
    }

    async fn mark_action(&self, key: String) -> bool {
        const RETENTION: Duration = Duration::from_secs(600);
        let mut handled = self.handled_actions.lock().await;
        handled.retain(|_, timestamp| timestamp.elapsed() < RETENTION);
        handled.insert(key, Instant::now()).is_none()
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
