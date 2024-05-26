use std::sync::Arc;

use octocrab::models::{issues::Comment, CommentId, NotificationId};
use tracing::{info, instrument};

use crate::{api, messages::MessageLoader};

use shared::{github::PrMetadata, near::NearClient, PRInfo};

use self::{actions::Action, commands::Command};

pub mod actions;
pub mod commands;
pub(crate) mod common;

#[derive(Clone, Debug)]
pub struct Context {
    pub github: Arc<api::GithubClient>,
    pub near: Arc<NearClient>,
    pub messages: Arc<MessageLoader>,
}

pub struct Event {
    pub event: EventType,
    pub notification_id: Option<NotificationId>,
    pub comment_id: Option<CommentId>,
}

impl Event {
    pub async fn execute(&self, context: Context) -> anyhow::Result<bool> {
        let pr = self.event.pr();
        let check_info = context.check_info(pr).await?;

        let result = match &self.event {
            EventType::Command(command) => command.execute(context.clone(), check_info).await,
            EventType::Action(action) => action.execute(context.clone(), check_info).await,
        }?;

        // TODO: this should be done somehow more properly
        if let Some(notification_id) = self.notification_id {
            context
                .github
                .mark_notification_as_read(notification_id)
                .await?;
        }

        Ok(result)
    }

    pub fn pr(&self) -> &PrMetadata {
        self.event.pr()
    }
}

#[derive(Debug, Clone)]
pub enum EventType {
    Command(Command),
    Action(Action),
}

impl EventType {
    pub fn pr(&self) -> &PrMetadata {
        match self {
            EventType::Command(command) => command.pr(),
            EventType::Action(action) => action.pr(),
        }
    }
}
