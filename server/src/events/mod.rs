use std::sync::Arc;

use octocrab::models::{issues::Comment, NotificationId};
use tracing::{info, instrument};

use crate::{
    api::{self, github::PrMetadata, near::PRInfo},
    messages::MessageLoader,
};

use self::{actions::Action, commands::Command};

pub mod actions;
pub mod commands;
pub(crate) mod common;

#[derive(Clone, Debug)]
pub struct Context {
    pub github: Arc<api::github::GithubClient>,
    pub near: Arc<api::near::NearClient>,
    pub messages: Arc<MessageLoader>,
}

#[derive(Debug, Clone)]
pub enum Event {
    Command(Command),
    Action(Action),
}

impl Event {
    pub async fn execute(
        &self,
        context: Context,
        notification_id: Option<NotificationId>,
    ) -> anyhow::Result<bool> {
        let pr = self.pr();
        let check_info = context.check_info(pr).await?;

        let result = match self {
            Event::Command(command) => command.execute(context.clone(), check_info).await,
            Event::Action(action) => action.execute(context.clone(), check_info).await,
        }?;

        // TODO: this should be done somehow more properly
        if let Some(notification_id) = notification_id {
            context
                .github
                .mark_notification_as_read(notification_id)
                .await?;
        }

        Ok(result)
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
