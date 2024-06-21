use tracing::{instrument, warn};

use shared::{github::PrMetadata, Event, PRInfo};

use crate::{events::Context, messages::MsgCategory};

#[derive(Debug, Clone)]
pub struct PullRequestFinalize {}

impl PullRequestFinalize {
    #[instrument(skip(self, pr, context, info), fields(pr = pr.full_id))]
    pub async fn execute(
        &self,
        pr: &PrMetadata,
        context: Context,
        info: PRInfo,
    ) -> anyhow::Result<bool> {
        if info.executed {
            warn!("PR {} is already finalized. Skipping", pr.full_id);
            return Ok(false);
        }

        let events = context.near.send_finalize(&pr.full_id).await?;

        if !info.allowed_repo {
            return Ok(true);
        }

        self.reply_depends_on_events(pr, context, events, info.average_score())
            .await?;

        Ok(true)
    }

    async fn reply_depends_on_events(
        &self,
        pr: &PrMetadata,
        context: Context,
        events: Vec<Event>,
        score: u32,
    ) -> anyhow::Result<()> {
        let mut total_lifetime_bonus = 0;
        let mut lifetime_reward = 0;
        let mut weekly_bonus = 0;
        let mut monthly_bonus = 0;

        for e in events {
            match e {
                Event::StreakLifetimeRewarded {
                    total_lifetime_percent,
                    lifetime_percent,
                } => {
                    total_lifetime_bonus = total_lifetime_percent;
                    lifetime_reward = lifetime_percent;
                    // This is a superior message, so we can break here
                    break;
                }
                Event::StreakFlatRewarded {
                    streak_id,
                    bonus_rating,
                    ..
                } => {
                    if streak_id == 0 {
                        weekly_bonus = bonus_rating;
                    } else if streak_id == 1 {
                        monthly_bonus = bonus_rating;
                    }
                }
                Event::NewSloth { .. } => {}
            }
        }

        if total_lifetime_bonus > 5 {
            let rank: &str = match total_lifetime_bonus {
                a if a >= 25 => "Rust",
                a if a >= 20 => "Platinum",
                a if a >= 15 => "Gold",
                a if a >= 10 => "Silver",
                a => {
                    tracing::error!(
                        "Expected total_lifetime_bonus as one of predefined values, but got: {a}. Recovering to Bronze",
                    );
                    "Bronze"
                }
            };
            context
                .reply(
                    pr,
                    None,
                    MsgCategory::FinalMessagesLifetimeBonus,
                    vec![
                        (
                            "total_lifetime_percent".to_string(),
                            total_lifetime_bonus.to_string(),
                        ),
                        ("lifetime_percent".to_string(), lifetime_reward.to_string()),
                        ("pr_author_username".to_string(), pr.author.login.clone()),
                        ("rank_name".to_string(), rank.to_string()),
                    ],
                )
                .await?;
        } else if total_lifetime_bonus > 0 {
            context
                .reply(
                    pr,
                    None,
                    MsgCategory::FinalMessagesFirstLifetimeBonus,
                    vec![("pr_author_username".to_string(), pr.author.login.clone())],
                )
                .await?;
        } else if monthly_bonus > 0 {
            context
                .reply(
                    pr,
                    None,
                    MsgCategory::FinalMessagesMonthlyStreak,
                    vec![
                        ("pr_author_username".to_string(), pr.author.login.clone()),
                        ("bonus_rating".to_string(), monthly_bonus.to_string()),
                    ],
                )
                .await?;
        } else if weekly_bonus > 0 {
            context
                .reply(
                    pr,
                    None,
                    MsgCategory::FinalMessagesWeeklyStreak,
                    vec![
                        ("pr_author_username".to_string(), pr.author.login.clone()),
                        ("bonus_rating".to_string(), monthly_bonus.to_string()),
                    ],
                )
                .await?;
        } else {
            context
                .reply(
                    pr,
                    None,
                    MsgCategory::FinalMessage,
                    vec![
                        ("pr_author_username".to_string(), pr.author.login.clone()),
                        ("score".to_string(), score.to_string()),
                    ],
                )
                .await?;
        }
        Ok(())
    }
}
