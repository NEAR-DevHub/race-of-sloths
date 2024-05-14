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

pub trait ParseCommand {
    fn parse_command(
        bot_name: &str,
        pr_metadata: &PrMetadata,
        comment: &Comment,
    ) -> Option<Command>;
}

impl ParseCommand for Command {
    fn parse_command(
        bot_name: &str,
        pr_metadata: &PrMetadata,
        comment: &Comment,
    ) -> Option<Command> {
        type F = fn(&str, &PrMetadata, &Comment) -> Option<Command>;

        let parse_command: [F; 5] = [
            BotIncluded::parse_command,
            BotScored::parse_command,
            BotPaused::parse_command,
            BotUnpaused::parse_command,
            BotExcluded::parse_command,
        ];

        for parse in parse_command.iter() {
            if let Some(command) = parse(bot_name, pr_metadata, comment) {
                return Some(command);
            }
        }

        None
    }
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
