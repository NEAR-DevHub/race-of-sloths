use std::sync::Arc;

use octocrab::models::issues::Comment;
use tracing::{info, instrument};

use crate::api::{self, github::PrMetadata, near::PRInfo};

use self::{actions::Action, commands::Command};

pub mod actions;
pub mod commands;
pub(crate) mod common;

#[derive(Clone, Debug)]
pub struct Context {
    pub github: Arc<api::github::GithubClient>,
    pub near: Arc<api::near::NearClient>,
}

#[derive(Debug, Clone)]
pub enum Event {
    Command(Command),
    Action(Action),
}

impl Event {
    pub async fn execute(&self, context: Context) -> anyhow::Result<bool> {
        let pr = self.pr();
        let check_info = context.check_info(pr).await?;

        match self {
            Event::Command(command) => command.execute(context, check_info).await,
            Event::Action(action) => action.execute(context, check_info).await,
        }
    }
}

impl Event {
    pub fn pr(&self) -> &PrMetadata {
        match self {
            Event::Command(command) => command.pr(),
            Event::Action(action) => action.pr(),
        }
    }
}
