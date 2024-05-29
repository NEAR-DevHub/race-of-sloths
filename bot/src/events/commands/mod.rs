use shared::github::User;

use crate::messages::MsgCategory;

use super::*;

pub mod exclude;
pub mod pause;
pub mod score;
pub mod start;
pub mod unknown;

pub use self::{exclude::*, pause::*, score::*, start::*, unknown::*};

#[derive(Debug, Clone)]
pub enum Command {
    Include(BotIncluded),
    Score(BotScored),
    Pause(BotPaused),
    Unpause(BotUnpaused),
    Excluded(BotExcluded),
    Unknown(UnknownCommand),
}

impl Command {
    pub fn parse_command(
        bot_name: &str,
        pr_metadata: &PrMetadata,
        comment: &Comment,
    ) -> Option<Command> {
        let (command, args) = common::extract_command_with_args(bot_name, comment)?;

        Some(match command.as_str() {
            "score" | "rate" | "value" => BotScored::construct(comment, args),
            "pause" | "block" => BotPaused::construct(comment),
            "unpause" | "unblock" => BotUnpaused::construct(comment),
            "exclude" | "leave" => BotExcluded::construct(comment),
            "include" | "in" | "start" | "join" => BotIncluded::construct(comment),

            _ => {
                info!(
                    "Unknown command: {} for PR: {}",
                    command, pr_metadata.full_id
                );
                UnknownCommand::construct(comment, command, args)
            }
        })
    }

    pub fn parse_body(bot_name: &str, pr_metadata: &PrMetadata) -> Option<Command> {
        BotIncluded::parse_body(bot_name, pr_metadata)
    }

    pub fn timestamp(&self) -> &chrono::DateTime<chrono::Utc> {
        match self {
            Command::Include(event) => &event.timestamp,
            Command::Score(event) => &event.timestamp,
            Command::Pause(event) => &event.timestamp,
            Command::Unpause(event) => &event.timestamp,
            Command::Excluded(event) => &event.timestamp,
            Command::Unknown(event) => &event.timestamp,
        }
    }

    #[instrument(skip(self, context, check_info, pr), fields(pr = pr.full_id))]
    pub async fn execute(
        &self,
        pr: &PrMetadata,
        context: Context,
        check_info: PRInfo,
        sender: &User,
        first_reply: bool,
    ) -> anyhow::Result<bool> {
        if !check_info.allowed_org {
            info!(
                "Sloth called for a PR from not allowed org: {}. Skipping",
                pr.full_id
            );
            context
                .reply_with_error(
                    pr,
                    None,
                    MsgCategory::ErrorOrgNotInAllowedListMessage,
                    vec![("pr_author_username".to_string(), pr.author.login.clone())],
                )
                .await?;

            return Ok(false);
        }

        if check_info.executed {
            info!(
                "Sloth called for a PR that is already executed: {}. Skipping",
                pr.full_id
            );
            if let Command::Score(event) = self {
                context
                    .reply_with_error(
                        pr,
                        Some(event.comment_id),
                        MsgCategory::ErrorLateScoringMessage,
                        vec![],
                    )
                    .await?;
            }

            return Ok(false);
        }

        if !check_info.allowed_repo && !matches!(self, Command::Unpause(_) | Command::Pause(_)) {
            info!(
                "Sloth called for a PR from paused repo: {}. Skipping",
                pr.full_id
            );

            if first_reply {
                context
                    .reply_with_error(
                        pr,
                        None,
                        MsgCategory::ErrorPausedMessage,
                        vec![("user".to_string(), sender.login.clone())],
                    )
                    .await?;
            }

            return Ok(false);
        }

        if check_info.excluded && !matches!(self, Command::Include(_)) {
            info!(
                "Sloth called for a PR from excluded PR: {}. Skipping",
                pr.full_id
            );

            return Ok(false);
        }

        match self {
            Command::Include(event) => event.execute(pr, context, check_info, sender).await,
            Command::Score(event) => event.execute(pr, context, check_info, sender).await,
            Command::Pause(event) => event.execute(pr, context, check_info, sender).await,
            Command::Unpause(event) => event.execute(pr, context, check_info, sender).await,
            Command::Excluded(event) => event.execute(pr, context, check_info).await,
            Command::Unknown(event) => event.execute(pr, context, check_info, sender).await,
        }
    }
}
