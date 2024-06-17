use tracing::{debug, instrument};

use crate::messages::MsgCategory;

use shared::{github::User, PRInfo};

use super::*;

#[derive(Debug, Clone)]
pub struct BotScored {
    score: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub comment_id: u64,
}

impl BotScored {
    pub fn new(score: String, timestamp: chrono::DateTime<chrono::Utc>, comment_id: u64) -> Self {
        Self {
            score,
            timestamp,
            comment_id,
        }
    }

    pub fn score(&self) -> (u8, bool) {
        let score: Option<u32> = self
            .score
            .split_whitespace()
            .next()
            .and_then(|s| s.parse::<u32>().ok());

        match score {
            None => (0, true),
            Some(score) => match score {
                0 | 1 | 2 | 3 | 5 | 8 | 13 => (score as u8, false),
                // edit to nearest valid score
                number => {
                    let mut valid_scores: Vec<i64> = vec![0, 1, 2, 3, 5, 8, 13];
                    valid_scores.sort_by_key(|&x| (x - number as i64).abs());
                    (valid_scores[0] as u8, true)
                }
            },
        }
    }
}

impl BotScored {
    #[instrument(skip(self, pr, context, info, sender), fields(pr = pr.full_id, score = self.score))]
    pub async fn execute(
        &self,
        pr: &PrMetadata,
        context: Context,
        info: PRInfo,
        sender: &User,
    ) -> anyhow::Result<bool> {
        if !info.exist || info.executed {
            debug!(
                "Sloth is not included before or PR is already executed in: {}. Skipping.",
                pr.full_id,
            );
            return Ok(false);
        }

        let (number, edited) = self.score();

        if pr.author.login == sender.login {
            debug!(
                "Author tried to score their own PR {}. Skipping.",
                pr.full_id,
            );
            context
                .reply_with_error(
                    pr,
                    Some(self.comment_id),
                    MsgCategory::ErrorSelfScore,
                    vec![],
                )
                .await?;
            return Ok(false);
        }

        if !sender.is_maintainer() {
            debug!("Non-maintainer tried to score PR {}. Skipping.", pr.full_id,);
            context
                .reply_with_error(
                    pr,
                    Some(self.comment_id),
                    MsgCategory::ErrorRightsViolationMessage,
                    vec![],
                )
                .await?;
            return Ok(false);
        }

        context
            .near
            .send_scored(pr, &sender.login, number as u64)
            .await?;

        let (category, args) = match (number, edited) {
            (num, true) => (
                MsgCategory::CorrectableScoringMessage,
                vec![
                    ("reviewer".to_string(), sender.login.clone()),
                    ("corrected_score".to_string(), num.to_string()),
                    ("score".to_string(), self.score.clone()),
                ],
            ),
            (0, _) => (
                MsgCategory::CorrectZeroScoringMessage,
                vec![("pr_author_username".to_string(), pr.author.login.clone())],
            ),
            (_, _) => (
                MsgCategory::CorrectNonzeroScoringMessage,
                vec![("reviewer".to_string(), sender.login.clone())],
            ),
        };

        context
            .reply(pr, Some(self.comment_id), category, args)
            .await?;
        Ok(true)
    }

    pub fn construct(comment: &Comment, input: String) -> Command {
        Command::Score(BotScored::new(input, comment.created_at, comment.id.0))
    }
}

#[cfg(test)]
mod tests {
    use super::commands::BotScored;

    #[test]
    pub fn score_parsing() {
        assert_eq!(
            (5, false),
            BotScored::new("5".to_string(), chrono::Utc::now(), 1).score()
        );

        assert_eq!(
            (5, false),
            BotScored::new("5 ".to_string(), chrono::Utc::now(), 1).score()
        );

        assert_eq!(
            (5, false),
            BotScored::new("5 asdasdas".to_string(), chrono::Utc::now(), 1).score()
        );

        assert_eq!(
            (0, true),
            BotScored::new("as".to_string(), chrono::Utc::now(), 1).score()
        );

        assert_eq!(
            (0, false),
            BotScored::new("0".to_string(), chrono::Utc::now(), 1).score()
        );

        assert_eq!(
            (8, true),
            BotScored::new("9".to_string(), chrono::Utc::now(), 1).score()
        );

        assert_eq!(
            (8, true),
            BotScored::new("7".to_string(), chrono::Utc::now(), 1).score()
        );

        assert_eq!(
            (0, true),
            BotScored::new("".to_string(), chrono::Utc::now(), 1).score()
        );
    }
}
