use tracing::{debug, instrument};

use self::api::github::User;

use super::*;

#[derive(Debug, Clone)]
pub struct BotExcluded {
    pub pr_metadata: PrMetadata,
    pub author: User,
    pub comment_id: u64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl BotExcluded {
    #[instrument(skip(self, context, _check_info), fields(pr = self.pr_metadata.full_id))]
    pub async fn execute(&self, context: Context, _check_info: PRInfo) -> anyhow::Result<()> {
        if !self.author.is_maintainer() {
            info!(
                "Tried to exclude a PR from not maintainer: {}. Skipping",
                self.pr_metadata.full_id
            );
            return Ok(());
        }

        debug!("Excluding PR {}", self.pr_metadata.full_id);

        context.near.send_exclude(&self.pr_metadata).await?;
        context.reply(
            &self.pr_metadata.owner,
            &self.pr_metadata.repo,
            self.pr_metadata.number,
            self.comment_id,
            "The PR has been excluded. If you want to include it again, please restart the bot with `include` command",
        ).await?;
        Ok(())
    }

    pub fn construct(pr_metadata: &PrMetadata, comment: &Comment) -> Command {
        let author = User::new(
            comment.user.login.clone(),
            comment.author_association.clone(),
        );
        let timestamp = comment.created_at;

        Command::Excluded(BotExcluded {
            pr_metadata: pr_metadata.clone(),
            author,
            comment_id: comment.id.0,
            timestamp,
        })
    }
}
