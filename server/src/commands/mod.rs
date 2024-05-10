use std::sync::Arc;

use octocrab::models::{activity::Notification, issues::Comment};

use crate::api::{self, github::PrMetadata};

use self::{
    merged::PullRequestMerged,
    pause::{BotPaused, BotUnpaused},
    score::BotScored,
    start::BotIncluded,
};

pub(crate) mod common;
pub mod merged;
pub mod pause;
pub mod score;
pub mod start;

pub type Context = Arc<ContextStruct>;

#[derive(Clone)]
pub struct ContextStruct {
    pub github: api::github::GithubClient,
    pub near: api::near::NearClient,
}

#[async_trait::async_trait]
pub trait Execute {
    async fn execute(&self, context: Context) -> anyhow::Result<()>;
}

pub trait ParseCommand {
    fn parse_command(
        bot_name: &str,
        notification: &Notification,
        pr_metadata: &PrMetadata,
        comment: &Comment,
    ) -> Option<Command>;
}

impl ParseCommand for Command {
    fn parse_command(
        bot_name: &str,
        notification: &Notification,
        pr_metadata: &PrMetadata,
        comment: &Comment,
    ) -> Option<Command> {
        type F = fn(&str, &Notification, &PrMetadata, &Comment) -> Option<Command>;

        let parse_command: [F; 4] = [
            BotIncluded::parse_command,
            BotScored::parse_command,
            BotPaused::parse_command,
            BotUnpaused::parse_command,
        ];

        for parse in parse_command.iter() {
            if let Some(command) = parse(bot_name, notification, pr_metadata, comment) {
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
}

impl Command {
    pub fn pr(&self) -> &PrMetadata {
        match self {
            Command::Include(event) => &event.pr_metadata,
            Command::Score(event) => &event.pr_metadata,
            Command::Pause(event) => &event.pr_metadata,
            Command::Unpause(event) => &event.pr_metadata,
        }
    }

    pub fn timestamp(&self) -> &chrono::DateTime<chrono::Utc> {
        match self {
            Command::Include(event) => &event.timestamp,
            Command::Score(event) => &event.timestamp,
            Command::Pause(event) => &event.timestamp,
            Command::Unpause(event) => &event.timestamp,
        }
    }
}

#[async_trait::async_trait]
impl Execute for Command {
    async fn execute(&self, context: Context) -> anyhow::Result<()> {
        match self {
            Command::Include(event) => event.execute(context).await,
            Command::Score(event) => event.execute(context).await,
            Command::Pause(event) => event.execute(context).await,
            Command::Unpause(event) => event.execute(context).await,
        }
    }
}

pub enum Event {
    Command(Command),
    Merged(PullRequestMerged),
}

#[async_trait::async_trait]
impl Execute for Event {
    async fn execute(&self, context: Context) -> anyhow::Result<()> {
        match self {
            Event::Command(command) => command.execute(context).await,
            Event::Merged(event) => event.execute(context).await,
        }
    }
}

impl Event {
    pub fn pr(&self) -> &PrMetadata {
        match self {
            Event::Command(command) => command.pr(),
            Event::Merged(event) => &event.pr_metadata,
        }
    }
}
