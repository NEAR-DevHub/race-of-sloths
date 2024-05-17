use tracing::{debug, instrument};

use crate::messages::MsgCategory;

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

    pub fn score(&self) -> (u8, bool) {
        let score = self.score.parse::<u8>().ok();

        match score {
            None => (0, true),
            Some(score) => match score {
                0 | 1 | 2 | 3 | 5 | 8 | 13 => (score, false),
                // edit to nearest valid score
                number => {
                    let mut valid_scores: Vec<i32> = vec![0, 1, 2, 3, 5, 8, 13];
                    valid_scores.sort_by_key(|&x| (x - number as i32).abs());
                    (valid_scores[0] as u8, true)
                }
            },
        }
    }
}

impl BotScored {
    #[instrument(skip(self, context, info), fields(pr = self.pr_metadata.full_id, score = self.score))]
    pub async fn execute(&self, context: Context, info: PRInfo) -> anyhow::Result<bool> {
        if !info.exist || info.executed {
            debug!(
                "Sloth is not included before or PR is already executed in: {}. Skipping.",
                self.pr_metadata.full_id,
            );
            return Ok(false);
        }

        let (number, edited) = self.score();

        if self.pr_metadata.author.login == self.sender.login {
            debug!(
                "Author tried to score their own PR {}. Skipping.",
                self.pr_metadata.full_id,
            );
            context
                .reply_with_error(&self.pr_metadata, MsgCategory::ErrorSelfScore, vec![])
                .await?;
            return Ok(false);
        }

        if !self.sender.is_maintainer() {
            debug!(
                "Non-maintainer tried to score PR {}. Skipping.",
                self.pr_metadata.full_id,
            );
            context
                .reply_with_error(
                    &self.pr_metadata,
                    MsgCategory::ErrorRightsViolationMessage,
                    vec![],
                )
                .await?;
            return Ok(false);
        }

        context
            .near
            .send_scored(&self.pr_metadata, &self.sender.login, number as u64)
            .await?;

        let (category, args) = match (number, edited) {
            (num, true) => (
                MsgCategory::CorrectableScoringMessage,
                vec![
                    ("corrected_score".to_string(), num.to_string()),
                    ("score".to_string(), self.score.clone()),
                ],
            ),
            (0, _) => (MsgCategory::CorrectZeroScoringMessage, vec![]),
            (_, _) => (MsgCategory::CorrectNonzeroScoringMessage, vec![]),
        };

        context
            .reply(&self.pr_metadata, Some(self.comment_id), category, args)
            .await?;
        Ok(true)
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
