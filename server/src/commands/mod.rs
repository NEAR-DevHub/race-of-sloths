use std::sync::Arc;

use octocrab::models::issues::Comment;
use tracing::{info, instrument};

use crate::api::{self, github::PrMetadata, near::PRInfo};

use self::{
    exclude::BotExcluded,
    merged::PullRequestMerged,
    pause::{BotPaused, BotUnpaused},
    score::BotScored,
    stale::PullRequestStale,
    start::BotIncluded,
};

pub(crate) mod common;
pub mod exclude;
pub mod merged;
pub mod pause;
pub mod score;
pub mod stale;
pub mod start;

#[derive(Clone, Debug)]
pub struct Context {
    pub github: Arc<api::github::GithubClient>,
    pub near: Arc<api::near::NearClient>,
}

#[derive(Debug, Clone)]
pub enum Command {
    Include(BotIncluded),
    Score(BotScored),
    Pause(BotPaused),
    Unpause(BotUnpaused),
    Excluded(BotExcluded),
}

impl Command {
    pub fn parse_command(
        bot_name: &str,
        pr_metadata: &PrMetadata,
        comment: &Comment,
    ) -> Option<Command> {
        let (command, arg) = common::extract_command_with_args(bot_name, comment)?;

        Some(match command.as_str() {
            "score" => BotScored::construct(pr_metadata, comment, arg),
            "pause" => BotPaused::construct(pr_metadata, comment),
            "unpause" => BotUnpaused::construct(pr_metadata, comment),
            "exclude" => BotExcluded::construct(pr_metadata, comment),
            "include" | "in" | "start" | "join" => BotIncluded::construct(pr_metadata, comment),
            _ => {
                info!(
                    "Unknown command: {} for PR: {}",
                    command, pr_metadata.full_id
                );
                return None;
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
        }
    }

    pub fn timestamp(&self) -> &chrono::DateTime<chrono::Utc> {
        match self {
            Command::Include(event) => &event.timestamp,
            Command::Score(event) => &event.timestamp,
            Command::Pause(event) => &event.timestamp,
            Command::Unpause(event) => &event.timestamp,
            Command::Excluded(event) => &event.timestamp,
        }
    }
}

impl Command {
    #[instrument(skip(self, context), fields(pr = self.pr().full_id))]
    pub async fn execute(&self, context: Context, check_info: PRInfo) -> anyhow::Result<()> {
        match self {
            Command::Include(event) => event.execute(context, check_info).await,
            Command::Score(event) => event.execute(context, check_info).await,
            Command::Pause(event) => event.execute(context, check_info).await,
            Command::Unpause(event) => event.execute(context, check_info).await,
            Command::Excluded(event) => event.execute(context, check_info).await,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Event {
    Command(Command),
    Merged(PullRequestMerged),
    Stale(PullRequestStale),
}

impl Event {
    pub async fn execute(&self, context: Context) -> anyhow::Result<()> {
        let pr = self.pr();
        let check_info = context.check_info(pr).await?;
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

            return Ok(());
        }

        if check_info.executed {
            info!(
                "Sloth called for a PR that is already executed: {}. Skipping",
                pr.full_id
            );

            return Ok(());
        }

        if !check_info.allowed_repo && !matches!(&self, Event::Command(Command::Unpause(_))) {
            info!(
                "Sloth called for a PR from paused repo: {}. Skipping",
                pr.full_id
            );

            return Ok(());
        }

        if check_info.excluded && !matches!(self, Event::Command(Command::Include(_))) {
            info!(
                "Sloth called for a PR from excluded repo: {}. Skipping",
                pr.full_id
            );

            return Ok(());
        }

        match self {
            Event::Command(command) => command.execute(context, check_info).await,
            Event::Merged(event) => event.execute(context, check_info).await,
            Event::Stale(event) => event.execute(context, check_info).await,
        }?;

        Ok(())
    }
}

impl Event {
    pub fn pr(&self) -> &PrMetadata {
        match self {
            Event::Command(command) => command.pr(),
            Event::Merged(event) => &event.pr_metadata,
            Event::Stale(event) => &event.pr_metadata,
        }
    }
}
