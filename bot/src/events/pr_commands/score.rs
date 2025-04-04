use tracing::{debug, instrument};

use crate::messages::MsgCategory;

use shared::{github::User, PRInfo, Score};

use super::*;

#[derive(Debug, Clone)]
pub struct BotScored {
    score: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub comment_id: Option<u64>,
    pub muted: bool,
}

impl BotScored {
    pub fn new(
        score: String,
        timestamp: chrono::DateTime<chrono::Utc>,
        comment_id: Option<u64>,
    ) -> Self {
        Self {
            score,
            timestamp,
            comment_id,
            muted: false,
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
    #[instrument(skip(self, pr, context, info, sender), fields(pr = pr.repo_info.full_id, score = self.score))]
    pub async fn execute(
        &self,
        pr: &PrMetadata,
        context: Context,
        info: &mut PRInfo,
        sender: &User,
    ) -> anyhow::Result<EventResult> {
        if info.executed {
            debug!(
                "Sloth is not included before or PR is already executed in: {}. Skipping.",
                pr.repo_info.full_id,
            );
            return Ok(EventResult::Skipped);
        }

        if !info.exist {
            BotIncluded::new(self.timestamp, self.comment_id)
                .execute(pr, context.clone(), info, sender)
                .await?;
        }

        // Info is updated in the previous call
        if !info.exist {
            debug!(
                "Sloth is not included in {}. Skipping.",
                pr.repo_info.full_id
            );
            return Ok(EventResult::Skipped);
        }

        let (number, edited) = self.score();

        if pr.author.login == sender.login {
            debug!(
                "Author tried to score their own PR {}. Skipping.",
                pr.repo_info.full_id,
            );
            if !self.muted {
                context
                    .reply_with_error(
                        &pr.repo_info,
                        self.comment_id,
                        MsgCategory::ErrorSelfScore,
                        vec![],
                    )
                    .await?;
            }
            return Ok(EventResult::RepliedWithError);
        }

        context
            .near
            .send_scored(pr, &sender.login, number as u64)
            .await?;

        if let Some(vote) = info.votes.iter_mut().find(|v| v.user == sender.login) {
            vote.score = number as u32;
        } else {
            info.votes.push(Score {
                user: sender.login.clone(),
                score: number as u32,
            });
        }

        if !self.muted && edited {
            context
                .reply(
                    &pr.repo_info,
                    self.comment_id,
                    MsgCategory::CorrectableScoringMessage,
                    vec![
                        ("reviewer", sender.login.clone()),
                        ("corrected_score", number.to_string()),
                        ("score", self.score.clone()),
                    ],
                )
                .await?;
        } else if let Some(comment) = self.comment_id {
            context
                .github
                .like_comment(&pr.repo_info.owner, &pr.repo_info.repo, comment)
                .await?;
        }

        Ok(EventResult::success(true))
    }

    pub fn muted(self) -> BotScored {
        BotScored {
            score: self.score,
            timestamp: self.timestamp,
            comment_id: self.comment_id,
            muted: true,
        }
    }

    pub fn construct(comment: &CommentRepr, input: String) -> Command {
        Command::Score(BotScored::new(input, comment.timestamp, comment.comment_id))
    }
}

#[cfg(test)]
mod tests {
    use super::pr_commands::BotScored;

    #[test]
    pub fn score_parsing() {
        assert_eq!(
            (5, false),
            BotScored::new("5".to_string(), chrono::Utc::now(), Some(1)).score()
        );

        assert_eq!(
            (5, false),
            BotScored::new("5 ".to_string(), chrono::Utc::now(), Some(1)).score()
        );

        assert_eq!(
            (5, false),
            BotScored::new("5 asdasdas".to_string(), chrono::Utc::now(), Some(1)).score()
        );

        assert_eq!(
            (0, true),
            BotScored::new("as".to_string(), chrono::Utc::now(), Some(1)).score()
        );

        assert_eq!(
            (0, false),
            BotScored::new("0".to_string(), chrono::Utc::now(), Some(1)).score()
        );

        assert_eq!(
            (8, true),
            BotScored::new("9".to_string(), chrono::Utc::now(), Some(1)).score()
        );

        assert_eq!(
            (8, true),
            BotScored::new("7".to_string(), chrono::Utc::now(), Some(1)).score()
        );

        assert_eq!(
            (0, true),
            BotScored::new("".to_string(), chrono::Utc::now(), Some(1)).score()
        );
    }
}
