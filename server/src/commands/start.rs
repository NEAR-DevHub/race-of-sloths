use tracing::{debug, instrument};

use crate::consts::INCLUDE_ALREADY_MERGED_MESSAGES;

use self::api::github::User;

use super::*;

#[derive(Debug, Clone)]
pub struct BotIncluded {
    pub sender: User,
    pub pr_metadata: PrMetadata,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub comment_id: u64,
}

impl BotIncluded {
    pub fn new(
        sender: User,
        pr_metadata: PrMetadata,
        timestamp: chrono::DateTime<chrono::Utc>,
        comment_id: u64,
    ) -> Self {
        Self {
            sender,
            pr_metadata,
            timestamp,
            comment_id,
        }
    }
}

impl BotIncluded {
    #[instrument(skip(self, context, info), fields(pr = self.pr_metadata.full_id))]
    pub async fn execute(&self, context: Context, info: PRInfo) -> anyhow::Result<()> {
        if info.exist {
            debug!(
                "Sloth is already included in {}. Skipping",
                self.pr_metadata.full_id,
            );
            return Ok(());
        }

        if self.pr_metadata.merged.is_some() {
            debug!(
                "PR {} is already merged. Skipping",
                self.pr_metadata.full_id,
            );
            return context
                .reply_with_error(&self.pr_metadata, &INCLUDE_ALREADY_MERGED_MESSAGES)
                .await;
        }

        debug!("Starting PR {}", self.pr_metadata.full_id);

        let status = PRInfo {
            exist: true,
            executed: false,
            merged: false,
            scored: false,
            votes: vec![],
            allowed_org: true,
            allowed_repo: true,
            excluded: false,
            comment_id: 0,
        };

        let comment = context
            .reply(
                &self.pr_metadata,
                Some(self.comment_id),
                &vec![status.status_message().as_str()],
            )
            .await?;

        context
            .near
            .send_start(&self.pr_metadata, self.sender.is_maintainer(), comment.id.0)
            .await
    }

    pub fn construct(pr_metadata: &PrMetadata, comment: &Comment) -> Command {
        Command::Include(BotIncluded::new(
            User::new(
                comment.user.login.clone(),
                comment.author_association.clone(),
            ),
            pr_metadata.clone(),
            comment.created_at,
            comment.id.0,
        ))
    }
}
