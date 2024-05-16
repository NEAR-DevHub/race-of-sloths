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
            "score" | "rate" | "value" => BotScored::construct(pr_metadata, comment, args),
            "pause" | "block" => BotPaused::construct(pr_metadata, comment),
            "unpause" | "unblock" => BotUnpaused::construct(pr_metadata, comment),
            "exclude" | "leave" => BotExcluded::construct(pr_metadata, comment),
            "include" | "in" | "start" | "join" => BotIncluded::construct(pr_metadata, comment),

            _ => {
                info!(
                    "Unknown command: {} for PR: {}",
                    command, pr_metadata.full_id
                );
                UnknownCommand::construct(pr_metadata, comment, command, args)
            }
        })
    }

    pub fn pr(&self) -> &PrMetadata {
        match self {
            Command::Include(event) => &event.pr_metadata,
            Command::Score(event) => &event.pr_metadata,
            Command::Pause(event) => &event.pr_metadata,
            Command::Unpause(event) => &event.pr_metadata,
            Command::Excluded(event) => &event.pr_metadata,
            Command::Unknown(event) => &event.pr_metadata,
        }
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
}

impl Command {
    #[instrument(skip(self, context, check_info), fields(pr = self.pr().full_id))]
    pub async fn execute(&self, context: Context, check_info: PRInfo) -> anyhow::Result<bool> {
        let pr = self.pr();
        if !check_info.allowed_org {
            info!(
                "Sloth called for a PR from not allowed org: {}. Skipping",
                pr.full_id
            );
            context
                .github
                .reply(
                    &pr.owner,
                    &pr.repo,
                    pr.number,
                    "The organization is not a part of the allowed organizations.",
                )
                .await?;

            return Ok(false);
        }

        if check_info.executed {
            info!(
                "Sloth called for a PR that is already executed: {}. Skipping",
                pr.full_id
            );

            return Ok(false);
        }

        if !check_info.allowed_repo && !matches!(self, Command::Unpause(_)) {
            info!(
                "Sloth called for a PR from paused repo: {}. Skipping",
                pr.full_id
            );

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
            Command::Include(event) => event.execute(context, check_info).await,
            Command::Score(event) => event.execute(context, check_info).await,
            Command::Pause(event) => event.execute(context, check_info).await,
            Command::Unpause(event) => event.execute(context, check_info).await,
            Command::Excluded(event) => event.execute(context, check_info).await,
            Command::Unknown(event) => event.execute(context, check_info).await,
        }
    }
}
