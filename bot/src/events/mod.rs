use std::sync::Arc;

use chrono::Utc;
use octocrab::models::{issues::Comment, NotificationId};
use tracing::{info, instrument, Level};

use crate::{api, messages::MessageLoader};

use shared::{
    github::{PrMetadata, User},
    near::NearClient,
    telegram::TelegramSubscriber,
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
    pub telegram: Arc<TelegramSubscriber>,
}

pub struct Event {
    pub event: EventType,
    pub pr: PrMetadata,
    pub comment: Option<Comment>,
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
                        self.comment.is_none(),
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
        send_event_to_telegram(&context.telegram, self, result.is_ok());
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

fn send_event_to_telegram(
    telegram: &Arc<TelegramSubscriber>,
    event: &crate::events::Event,
    success: bool,
) {
    let message = format!(
        "{} in the [{}](https://github.com/{}/{}/pull/{}) was {}",
        event.event,
        event.pr.full_id,
        event.pr.owner,
        event.pr.repo,
        event.pr.number,
        if success { "successful" } else { "failed" },
    );
    telegram.send_to_telegram(&message, &Level::INFO);
}
