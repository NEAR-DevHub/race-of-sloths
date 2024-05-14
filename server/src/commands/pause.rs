use tracing::{debug, info, instrument};

use self::api::github::User;

use super::*;

#[derive(Clone, Debug)]
pub struct BotPaused {
    pub pr_metadata: PrMetadata,
    pub sender: User,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub comment_id: u64,
}

impl BotPaused {
    #[instrument(skip(self, context, _check_info), fields(pr = self.pr_metadata.full_id))]
    pub async fn execute(&self, context: Context, _check_info: PRInfo) -> anyhow::Result<()> {
        debug!(
            "Pausing the repository in the PR: {}",
            self.pr_metadata.full_id
        );
        context
            .near
            .send_pause(&self.pr_metadata.owner, &self.pr_metadata.repo)
            .await?;
        context.reply(
                &self.pr_metadata.owner,
                &self.pr_metadata.repo,
                self.pr_metadata.number,
                self.comment_id,
                "We've paused this repository. From now on, we won't participate in this repository PRs but already scored PRs will be accepted after the merge",
            ).await?;
        Ok(())
    }

    pub fn construct(pr_metadata: &PrMetadata, comment: &Comment) -> Command {
        return Command::Pause(BotPaused {
            pr_metadata: pr_metadata.clone(),
            sender: User {
                login: comment.user.login.clone(),
                contributor_type: comment.author_association.clone(),
            },
            timestamp: comment.created_at,
            comment_id: comment.id.0,
        });
    }
}

#[derive(Clone, Debug)]
pub struct BotUnpaused {
    pub pr_metadata: PrMetadata,
    pub sender: User,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub comment_id: u64,
}

impl BotUnpaused {
    #[instrument(skip(self, context, info), fields(pr = self.pr_metadata.full_id))]
    pub async fn execute(&self, context: Context, info: PRInfo) -> anyhow::Result<()> {
        if !self.sender.is_maintainer() {
            info!(
                "Tried to unpause a PR from not maintainer: {}. Skipping",
                self.pr_metadata.full_id
            );
            return Ok(());
        }

        if !info.allowed_repo {
            context
                .near
                .send_unpause(&self.pr_metadata.owner, &self.pr_metadata.repo)
                .await?;
            debug!("Unpaused PR {}", self.pr_metadata.full_id);
            context.reply(
                &self.pr_metadata.owner,
                &self.pr_metadata.repo,
                self.pr_metadata.number,
                self.comment_id,
                "We've unpaused this repository. Please, start us to include us in the given PR.",
            ).await?;
            Ok(())
        } else {
            context
                .reply(
                    &self.pr_metadata.owner,
                    &self.pr_metadata.repo,
                    self.pr_metadata.number,
                    self.comment_id,
                    "Already unpaused.",
                )
                .await?;
            Ok(())
        }
    }

    pub fn construct(pr_metadata: &PrMetadata, comment: &Comment) -> Command {
        return Command::Unpause(BotUnpaused {
            pr_metadata: pr_metadata.clone(),
            sender: User {
                login: comment.user.login.clone(),
                contributor_type: comment.author_association.clone(),
            },
            timestamp: comment.created_at,
            comment_id: comment.id.0,
        });
    }
}
