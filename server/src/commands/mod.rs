use std::sync::Arc;

use crate::api;

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

#[async_trait::async_trait]
impl Execute for api::github::Event {
    async fn execute(&self, context: Context) -> anyhow::Result<()> {
        match self {
            api::github::Event::BotStarted(event) => event.execute(context).await,
            api::github::Event::BotScored(event) => event.execute(context).await,
            api::github::Event::PullRequestMerged(event) => event.execute(context).await,
        }
    }
}
