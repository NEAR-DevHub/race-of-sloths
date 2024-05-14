use tracing::{debug, instrument};

use self::api::{github::User, near::PRInfo};

use super::*;

#[derive(Debug, Clone)]
pub struct BotScored {
    pub sender: User,
    pub pr_metadata: PrMetadata,
    score: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub comment_id: u64,
}

impl BotScored {
    pub fn new(
        sender: User,
        pr_metadata: PrMetadata,
        score: String,
        timestamp: chrono::DateTime<chrono::Utc>,
        comment_id: u64,
    ) -> Self {
        Self {
            sender,
            pr_metadata,
            score,
            timestamp,
            comment_id,
        }
    }

    pub fn score(&self) -> Option<u8> {
        let score = self.score.parse::<u8>().ok()?;
        match score {
            1 | 2 | 3 | 5 | 8 | 13 => Some(score),
            _ => None,
        }
    }
}

impl BotScored {
    #[instrument(skip(self, context, info), fields(pr = self.pr_metadata.full_id, score = self.score))]
    pub async fn execute(&self, context: Context, info: PRInfo) -> anyhow::Result<()> {
        if !info.exist || info.executed {
            debug!(
                "Sloth is not included before or PR is already executed in: {}. Skipping.",
                self.pr_metadata.full_id,
            );
            return Ok(());
        }

        let score = self.score();
        if score.is_none() {
            debug!(
                "Invalid score for PR {}. Skipping.",
                self.pr_metadata.full_id,
            );
            return context
                .reply_with_error(
                    &self.pr_metadata.owner,
                    &self.pr_metadata.repo,
                    self.pr_metadata.number,
                    "Score should be a fibonacci number: 1, 2, 3, 5, 8, or 13.",
                )
                .await;
        }
        let score = score.unwrap();

        if self.pr_metadata.author.login == self.sender.login {
            debug!(
                "Author tried to score their own PR {}. Skipping.",
                self.pr_metadata.full_id,
            );
            return context
                .reply_with_error(
                    &self.pr_metadata.owner,
                    &self.pr_metadata.repo,
                    self.pr_metadata.number,
                    "You can't score your own PR.",
                )
                .await;
        }

        if !self.sender.is_maintainer() {
            debug!(
                "Non-maintainer tried to score PR {}. Skipping.",
                self.pr_metadata.full_id,
            );
            return context
                .reply_with_error(
                    &self.pr_metadata.owner,
                    &self.pr_metadata.repo,
                    self.pr_metadata.number,
                    "Only maintainers can score PRs.",
                )
                .await;
        }

        context
            .near
            .send_scored(&self.pr_metadata, &self.sender.login, score as u64)
            .await?;

        context
            .reply(
                &self.pr_metadata.owner,
                &self.pr_metadata.repo,
                self.pr_metadata.number,
                self.comment_id,
                "Thanks for submitting your score for the Sloth race.",
            )
            .await?;
        Ok(())
    }

    pub fn construct(pr_metadata: &PrMetadata, comment: &Comment, input: String) -> Command {
        Command::Score(BotScored::new(
            User {
                login: comment.user.login.clone(),
                contributor_type: comment.author_association.clone(),
            },
            pr_metadata.clone(),
            input,
            comment.created_at,
            comment.id.0,
        ))
    }
}
