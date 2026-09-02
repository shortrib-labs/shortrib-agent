use rig::completion::Message;
use slack_morphism::prelude::{SlackChannelId, SlackTeamId, SlackTs, SlackUserId};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{Mutex, RwLock};

pub(crate) type ConversationHistory = Arc<Mutex<Vec<Message>>>;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct UserKey {
    team_id: SlackTeamId,
    user_id: SlackUserId,
}

impl UserKey {
    pub(crate) fn new(team_id: SlackTeamId, user_id: SlackUserId) -> Self {
        Self { team_id, user_id }
    }

    pub(crate) fn oauth_storage_identity(&self) -> String {
        format!("{}\0{}", self.team_id.0, self.user_id.0)
    }

    pub(crate) fn team_id(&self) -> &SlackTeamId {
        &self.team_id
    }

    pub(crate) fn user_id(&self) -> &SlackUserId {
        &self.user_id
    }
}

#[derive(Clone, Eq, Hash, PartialEq)]
pub(crate) struct ConversationKey {
    channel_id: SlackChannelId,
    thread_ts: Option<SlackTs>,
}

impl ConversationKey {
    pub(crate) fn new(channel_id: SlackChannelId, thread_ts: Option<SlackTs>) -> Self {
        Self {
            channel_id,
            thread_ts,
        }
    }
}

#[derive(Default)]
struct UserState {
    conversations: HashMap<ConversationKey, ConversationHistory>,
}

#[derive(Clone, Default)]
pub(crate) struct UserStateStore {
    users: Arc<RwLock<HashMap<UserKey, Arc<RwLock<UserState>>>>>,
}

impl UserStateStore {
    pub(crate) async fn conversation(
        &self,
        user_key: UserKey,
        conversation_key: ConversationKey,
    ) -> ConversationHistory {
        let user = {
            let mut users = self.users.write().await;
            Arc::clone(
                users
                    .entry(user_key)
                    .or_insert_with(|| Arc::new(RwLock::new(UserState::default()))),
            )
        };

        let mut user = user.write().await;
        Arc::clone(
            user.conversations
                .entry(conversation_key)
                .or_insert_with(|| Arc::new(Mutex::new(Vec::new()))),
        )
    }
}
