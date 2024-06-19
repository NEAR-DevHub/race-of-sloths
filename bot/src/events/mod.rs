use std::sync::Arc;

use chrono::Utc;
use octocrab::models::{issues::Comment, CommentId, NotificationId};
use tracing::{info, instrument};

use crate::{api, messages::MessageLoader};

use shared::{
    github::{PrMetadata, User},
    near::NearClient,
    PRInfo,
};

use self::{actions::Action, commands::Command};

pub mod actions;
pub mod commands;
pub(crate) mod common;

#[derive(Clone)]
pub struct Context {
    pub github: Arc<api::GithubClient>,
    pub near: Arc<NearClient>,
    pub messages: Arc<MessageLoader>,
    pub prometheus: Arc<api::prometheus::PrometheusClient>,
    pub telegram: Arc<api::telegram::TelegramSubscriber>,
}

pub struct Event {
    pub event: EventType,
    pub pr: PrMetadata,
    pub comment_id: Option<CommentId>,
    pub event_time: chrono::DateTime<Utc>,
}

impl Event {
    pub async fn execute(&self, context: Context) -> anyhow::Result<bool> {
        let check_info = context.check_info(&self.pr).await?;

        let result = match &self.event {
            EventType::Command {
                command,
                sender,
                notification_id,
            } => {
                let should_update = command
                    .execute(
                        &self.pr,
                        context.clone(),
                        check_info,
                        sender,
                        self.comment_id.is_none(),
                    )
                    .await;
                if should_update.is_ok() {
                    context
                        .github
                        .mark_notification_as_read(notification_id.0)
                        .await?;
                }
                should_update
            }
            EventType::Action(action) => {
                action.execute(&self.pr, context.clone(), check_info).await
            }
        };
        context.telegram.process_event(self, result.is_ok());
        context
            .prometheus
            .record(&self.event, &self.pr, result.is_ok(), self.event_time);

        result
    }
}

#[derive(Debug, Clone)]
pub enum EventType {
    Command {
        command: Command,
        sender: User,
        notification_id: NotificationId,
    },
    Action(Action),
}

impl std::fmt::Display for EventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EventType::Command {
                command, sender, ..
            } => write!(
                f,
                "Command `{}` send by [{name}](https://github.com/{name})",
                command,
                name = sender.login
            ),
            EventType::Action(action) => write!(f, "Action `{action}`",),
        }
    }
}
