use std::sync::Arc;

use chrono::Utc;
use octocrab::models::NotificationId;
use tracing::{info, instrument, Level};

use crate::{
    api::{self, CommentRepr},
    messages::{FinalMessageData, MessageLoader},
};

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

pub enum EventResult {
    Success { should_update: bool },
    Skipped,
    RepliedWithError,
}

impl EventResult {
    pub fn success(should_update: bool) -> Self {
        Self::Success { should_update }
    }
}

impl std::fmt::Display for EventResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EventResult::Success { .. } => write!(f, "Success"),
            EventResult::Skipped => write!(f, "Skipped"),
            EventResult::RepliedWithError => write!(f, "Replied with error"),
        }
    }
}

impl Context {
    pub async fn status_message(
        &self,
        pr: &PrMetadata,
        mut comment: Option<CommentRepr>,
        info: PRInfo,
        final_data: Option<FinalMessageData>,
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
                let msg = if let Some(msg) =
                    self.try_update_message(comment.text, &info, pr, final_data.clone())
                {
                    msg
                } else {
                    // Couldn't update the message, it probabl means we try to overwrite some other message (as example, that repo is paused)
                    match self.new_status_message(pr, &info, final_data).await {
                        Ok(msg) => msg,
                        Err(e) => {
                            tracing::error!(
                                "Failed to get new status message for {}: {e}",
                                pr.full_id
                            );
                            return;
                        }
                    }
                };

                self.github
                    .edit_comment(&pr.owner, &pr.repo, comment.id, &msg)
                    .await
            }
            None => {
                // No comment found, create a new one
                match self.new_status_message(pr, &info, final_data).await {
                    Ok(msg) => self
                        .github
                        .reply(&pr.owner, &pr.repo, pr.number, &msg)
                        .await
                        .map(|_| ()),
                    Err(e) => {
                        tracing::error!("Failed to get new status message for {}: {e}", pr.full_id);
                        return;
                    }
                }
            }
        };

        if let Err(e) = result {
            tracing::error!("Failed to update status comment for {}: {e}", pr.full_id);
        }
    }

    async fn new_status_message(
        &self,
        pr: &PrMetadata,
        info: &PRInfo,
        final_data: Option<FinalMessageData>,
    ) -> anyhow::Result<String> {
        let timestamp = chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default() as u64;
        let user = self
            .near
            .user_info(
                &pr.author.login,
                vec![
                    TimePeriod::AllTime.time_string(timestamp),
                    TimePeriod::Month.time_string(timestamp),
                    TimePeriod::Week.time_string(timestamp),
                ],
            )
            .await?;

        Ok(self
            .messages
            .include_message_text(&self.github.user_handle, info, pr, user, final_data))
    }

    fn try_update_message(
        &self,
        text: String,
        info: &PRInfo,
        pr: &PrMetadata,
        final_data: Option<FinalMessageData>,
    ) -> Option<String> {
        let status = self
            .messages
            .status_message(&self.github.user_handle, info, pr, final_data)
            .unwrap_or_else(|err| {
                tracing::error!("Failed to get status message for {}: {err}", pr.full_id);
                String::default()
            });

        self.messages.update_pr_status_message(text, status)
    }
}

pub struct Event {
    pub event: EventType,
    pub pr: PrMetadata,
    pub comment: Option<CommentRepr>,
    pub event_time: chrono::DateTime<Utc>,
}

impl Event {
    pub async fn execute(
        &self,
        context: Context,
        check_info: &mut PRInfo,
    ) -> anyhow::Result<EventResult> {
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
        send_event_to_telegram(&context.telegram, self, &result);
        context
            .prometheus
            .record(&self.event, &self.pr, &result, self.event_time);

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
    result: &anyhow::Result<EventResult>,
) {
    let text = if let Ok(result) = result {
        result.to_string()
    } else {
        "Failed".to_string()
    };

    let message = format!(
        "{} in the [{}](https://github.com/{}/{}/pull/{}) was {}",
        event.event, event.pr.full_id, event.pr.owner, event.pr.repo, event.pr.number, text
    );
    telegram.send_to_telegram(&message, &Level::INFO);
}
