use std::sync::Arc;

use octocrab::models::{activity::Notification, issues::Comment};

use crate::api::{self, github::PrMetadata};

pub(crate) mod common;
pub mod merged;
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

pub trait ParseComment {
    type Command: Execute;

    fn parse_comment(
        bot_name: &str,
        notification: &Notification,
        pr_metadata: &PrMetadata,
        comment: &Comment,
    ) -> Option<Self::Command>;
}

#[async_trait::async_trait]
impl Execute for api::github::Command {
    async fn execute(&self, context: Context) -> anyhow::Result<()> {
        match self {
            api::github::Command::Include(event) => event.execute(context).await,
            api::github::Command::Score(event) => event.execute(context).await,
        }
    }
}

impl ParseComment for api::github::Command {
    type Command = api::github::Command;

    fn parse_comment(
        bot_name: &str,
        notification: &Notification,
        pr_metadata: &PrMetadata,
        comment: &Comment,
    ) -> Option<Self::Command> {
        if let Some(command) =
            api::github::BotStarted::parse_comment(bot_name, notification, pr_metadata, comment)
        {
            return Some(Self::Command::Include(command));
        }

        if let Some(command) =
            api::github::BotScored::parse_comment(bot_name, notification, pr_metadata, comment)
        {
            return Some(Self::Command::Score(command));
        }

        None
    }
}

pub enum Event {
    Command(api::github::Command),
    Merged(api::github::PullRequestMerged),
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
