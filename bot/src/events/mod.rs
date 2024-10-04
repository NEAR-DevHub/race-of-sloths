use std::sync::Arc;

use chrono::Utc;
use tracing::{info, instrument, Level};

use crate::{
    api::{self, CommentRepr},
    messages::{FinalMessageData, MessageLoader},
};

use shared::{
    github::{PrMetadata, RepoInfo, User},
    near::NearClient,
    telegram::TelegramSubscriber,
    PRInfo, TimePeriod,
};

use self::{actions::Action, pr_commands::Command};

pub mod actions;
pub(crate) mod common;
pub mod issue_commands;
pub mod pr_commands;

#[derive(Clone)]
pub struct Context {
    pub github: Arc<api::GithubClient>,
    pub bot_name: String,
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
                .get_bot_comment(&pr.repo_info.owner, &pr.repo_info.repo, pr.repo_info.number)
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
                                pr.repo_info.full_id
                            );
                            return;
                        }
                    }
                };

                self.github
                    .edit_comment(&pr.repo_info.owner, &pr.repo_info.repo, comment.id, &msg)
                    .await
            }
            None => {
                // No comment found, create a new one
                match self.new_status_message(pr, &info, final_data).await {
                    Ok(msg) => self
                        .github
                        .reply(
                            &pr.repo_info.owner,
                            &pr.repo_info.repo,
                            pr.repo_info.number,
                            &msg,
                        )
                        .await
                        .map(|_| ()),
                    Err(e) => {
                        tracing::error!(
                            "Failed to get new status message for {}: {e}",
                            pr.repo_info.full_id
                        );
                        return;
                    }
                }
            }
        };

        if let Err(e) = result {
            tracing::error!(
                "Failed to update status comment for {}: {e}",
                pr.repo_info.full_id
            );
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
            .include_message_text(&self.bot_name, info, pr, user, final_data))
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
            .status_message(&self.bot_name, info, pr, final_data)
            .unwrap_or_else(|err| {
                tracing::error!(
                    "Failed to get status message for {}: {err}",
                    pr.repo_info.full_id
                );
                String::default()
            });

        self.messages.update_pr_status_message(text, status)
    }
}

pub struct Event {
    pub event: EventType,
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
            EventType::PRCommand {
                command,
                sender,
                notification,
                pr,
            } => {
                let should_update = command
                    .execute(
                        pr,
                        context.clone(),
                        check_info,
                        sender,
                        self.comment.is_none(),
                    )
                    .await;
                if should_update.is_ok() {
                    context
                        .github
                        .mark_notification_as_read(*notification)
                        .await?;
                }
                should_update
            }
            EventType::Action { action, pr } => {
                action.execute(pr, context.clone(), check_info).await
            }
            EventType::IssueCommand {
                command,
                repo_info,
                sender,
                notification,
            } => {
                let result = command
                    .execute(
                        repo_info,
                        context.clone(),
                        check_info,
                        self.comment.is_none(),
                        sender,
                    )
                    .await;
                if result.is_ok() {
                    context
                        .github
                        .mark_notification_as_read(*notification)
                        .await?;
                }
                result
            }
        };

        match &self.event {
            EventType::PRCommand { pr, .. } | EventType::Action { pr, .. } => {
                context.prometheus.record_pr(
                    &self.event,
                    &pr.repo_info,
                    Some(&pr.author),
                    &result,
                    self.event_time,
                );
            }
            EventType::IssueCommand { repo_info, .. } => {
                context.prometheus.record_pr(
                    &self.event,
                    repo_info,
                    None,
                    &result,
                    self.event_time,
                );
            }
        }

        send_event_to_telegram(&context.telegram, self, &result);

        result
    }
}

#[derive(Debug, Clone)]
pub enum EventType {
    PRCommand {
        command: Command,
        sender: User,
        notification: crate::api::Notification,
        pr: PrMetadata,
    },
    Action {
        action: Action,
        pr: PrMetadata,
    },
    IssueCommand {
        command: issue_commands::Command,
        sender: User,
        notification: crate::api::Notification,
        repo_info: RepoInfo,
    },
}

impl EventType {
    pub fn repo_info(&self) -> &RepoInfo {
        match self {
            EventType::PRCommand { pr, .. } => &pr.repo_info,
            EventType::Action { pr, .. } => &pr.repo_info,
            EventType::IssueCommand {
                repo_info: issue, ..
            } => issue,
        }
    }

    pub fn same_event(&self, other: &Self) -> bool {
        match (self, other) {
            (
                EventType::PRCommand {
                    command: command1,
                    sender: sender1,
                    ..
                },
                EventType::PRCommand {
                    command: command2,
                    sender: sender2,
                    ..
                },
            ) => {
                std::mem::discriminant(command1) == std::mem::discriminant(command2)
                    && sender1.login == sender2.login
            }
            (
                EventType::Action { action, .. },
                EventType::Action {
                    action: action2, ..
                },
            ) => std::mem::discriminant(action) == std::mem::discriminant(action2),
            _ => false,
        }
    }
}

impl std::fmt::Display for EventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EventType::PRCommand {
                command, sender, ..
            } => write!(
                f,
                "PR Command `{}` send by [{name}](https://github.com/{name})",
                command,
                name = sender.login
            ),
            EventType::Action { action, .. } => write!(f, "Action `{action}`",),
            EventType::IssueCommand { command, .. } => write!(f, "Issue Command `{command}`",),
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

    let repo_info = event.event.repo_info();
    let message = format!(
        "{} in the [{}](https://github.com/{}/{}/pull/{}) was {}",
        event.event, repo_info.full_id, repo_info.owner, repo_info.repo, repo_info.number, text
    );
    telegram.send_to_telegram(&message, &Level::INFO);
}
