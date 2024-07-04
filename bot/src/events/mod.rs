use std::sync::Arc;

use chrono::Utc;
use octocrab::models::{issues::Comment, NotificationId};
use tracing::{info, instrument, Level};

use crate::{api, messages::MessageLoader};

use shared::{
    github::{PrMetadata, User},
    near::NearClient,
    telegram::TelegramSubscriber,
    PRInfo, TimePeriod,
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

impl Context {
    pub async fn status_message(
        &self,
        pr: &PrMetadata,
        mut comment: Option<Comment>,
        info: PRInfo,
    ) {
        if comment.is_none() {
            comment = self
                .github
                .get_bot_comment(&pr.owner, &pr.repo, pr.number)
                .await
                .ok()
                .flatten();
        }

        let result = match comment {
            Some(comment) => {
                let text = comment
                    .body
                    .or(comment.body_html)
                    .or(comment.body_text)
                    .unwrap_or_default();
                let status = self
                    .messages
                    .status_message(&self.github.user_handle, &info, pr);

                let message = self.messages.update_pr_status_message(text, status);

                self.github
                    .edit_comment(&pr.owner, &pr.repo, comment.id.0, &message)
                    .await
            }
            None => {
                let timestamp = chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default() as u64;
                let user = match self
                    .near
                    .user_info(
                        &pr.author.login,
                        vec![
                            TimePeriod::AllTime.time_string(timestamp),
                            TimePeriod::Month.time_string(timestamp),
                            TimePeriod::Week.time_string(timestamp),
                        ],
                    )
                    .await
                {
                    Ok(info) => info,
                    Err(e) => {
                        tracing::error!("Failed to get user info for {}: {e}", pr.author.login);
                        return;
                    }
                };

                let message =
                    self.messages
                        .include_message_text(&self.github.user_handle, &info, pr, user);

                self.github
                    .reply(&pr.owner, &pr.repo, pr.number, &message)
                    .await
                    .map(|_| ())
            }
        };

        if let Err(e) = result {
            tracing::error!("Failed to update status comment for {}: {e}", pr.full_id);
        }
    }
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

impl EventType {
    pub fn same_event(&self, other: &Self) -> bool {
        match (self, other) {
            (
                EventType::Command {
                    command: command1,
                    sender: sender1,
                    ..
                },
                EventType::Command {
                    command: command2,
                    sender: sender2,
                    ..
                },
            ) => {
                std::mem::discriminant(command1) == std::mem::discriminant(command2)
                    && sender1.login == sender2.login
            }
            (EventType::Action(a), EventType::Action(b)) => {
                std::mem::discriminant(a) == std::mem::discriminant(b)
            }
            _ => false,
        }
    }
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
