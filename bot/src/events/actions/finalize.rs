use tracing::{instrument, warn};

use shared::{github::PrMetadata, Event, PRInfo};

use crate::events::Context;

use super::EventResult;

#[derive(Debug, Clone)]
pub struct PullRequestFinalize {}

impl PullRequestFinalize {
    #[instrument(skip(self, pr, context, info), fields(pr = pr.full_id))]
    pub async fn execute(
        &self,
        pr: &PrMetadata,
        context: Context,
        info: &mut PRInfo,
    ) -> anyhow::Result<EventResult> {
        if info.executed {
            warn!("PR {} is already finalized. Skipping", pr.full_id);
            return Ok(EventResult::Skipped);
        }

        let events = context.near.send_finalize(&pr.full_id).await?;
        info.executed = true;

        if !info.allowed_repo || info.paused {
            return Ok(EventResult::success(false));
        }

        self.reply_depends_on_events(pr, context, events, info.average_score())
            .await?;

        Ok(EventResult::success(true))
    }

    async fn reply_depends_on_events(
        &self,
        pr: &PrMetadata,
        context: Context,
        events: Vec<Event>,
        score: u32,
    ) -> anyhow::Result<()> {
        let mut lifetime_reward = 0;
        let mut weekly_bonus = 0;
        let mut monthly_bonus = 0;
        let mut total_rating = 0;
        let mut total_lifetime_bonus = 0;
        let mut pr_this_week = 0;

        for e in events {
            match e {
                Event::StreakLifetimeRewarded { reward } => {
                    lifetime_reward = reward;
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
                Event::ExecutedWithRating {
                    rating,
                    applied_multiplier,
                    pr_number_this_week,
                } => {
                    total_rating = rating;
                    total_lifetime_bonus = applied_multiplier;
                    pr_this_week = pr_number_this_week;
                }
                Event::NewSloth { .. } => {}
            }
        }

        let message = context.messages.final_message(
            &pr.author.login,
            total_rating,
            score,
            weekly_bonus,
            monthly_bonus,
            lifetime_reward,
            total_lifetime_bonus,
            pr_this_week,
        )?;

        context
            .github
            .reply(&pr.owner, &pr.repo, pr.number, &message)
            .await?;

        Ok(())
    }
}
