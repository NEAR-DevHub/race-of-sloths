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
pub trait BotCommand {
    type Command: BotCommand;

    async fn execute(&self, context: Context) -> anyhow::Result<()>;
    fn parse_comment(
        bot_name: &str,
        notification: &Notification,
        pr_metadata: &PrMetadata,
        comment: &Comment,
    ) -> Option<Self::Command>;
}

#[async_trait::async_trait]
impl BotCommand for api::github::Event {
    type Command = api::github::Event;

    async fn execute(&self, context: Context) -> anyhow::Result<()> {
        match self {
            api::github::Event::BotStarted(event) => event.execute(context).await,
            api::github::Event::BotScored(event) => event.execute(context).await,
            api::github::Event::PullRequestMerged(event) => event.execute(context).await,
        }
    }

    fn parse_comment(
        bot_name: &str,
        notification: &Notification,
        pr_metadata: &PrMetadata,
        comment: &Comment,
    ) -> Option<Self::Command> {
        if let Some(command) =
            api::github::BotStarted::parse_comment(bot_name, notification, pr_metadata, comment)
        {
            return Some(Self::Command::BotStarted(command));
        }

        if let Some(command) =
            api::github::BotScored::parse_comment(bot_name, notification, pr_metadata, comment)
        {
            return Some(Self::Command::BotScored(command));
        }

        // Pull request merged is a special case, because it doesn't require a comment

        None
    }
}
