use tracing::{instrument, warn};

use shared::{github::PrMetadata, Event, PRInfo, Score};

use crate::events::Context;

use super::{EventResult, FinalMessageData};

#[derive(Debug, Clone)]
pub struct PullRequestFinalize {}

impl PullRequestFinalize {
    #[instrument(skip(self, pr, context, info), fields(pr = pr.repo_info.full_id))]
    pub async fn execute(
        &self,
        pr: &PrMetadata,
        context: Context,
        info: &mut PRInfo,
    ) -> anyhow::Result<EventResult> {
        if info.executed {
            warn!("PR {} is already finalized. Skipping", pr.repo_info.full_id);
            return Ok(EventResult::Skipped);
        }

        let is_active_pr = if !info.votes.is_empty() {
            // We don't need to check if PR is active if we have votes
            None
        } else {
            context
                .github
                .get_scores_and_active_pr_status(pr)
                .await
                .ok()
                .map(|(_, active)| (active, context.bot_name.clone()))
        };
        let events = context
            .near
            .send_finalize(&pr.repo_info.full_id, is_active_pr)
            .await?;
        info.executed = true;

        if info.paused_repo || info.blocked_repo {
            return Ok(EventResult::success(false));
        }

        self.react_on_events(pr, context, events, info).await?;

        Ok(EventResult::success(false))
    }

    async fn react_on_events(
        &self,
        pr: &PrMetadata,
        context: Context,
        events: Vec<Event>,
        info: &mut PRInfo,
    ) -> anyhow::Result<()> {
        let mut final_data = FinalMessageData::from_name(&pr.author.login);
        final_data.score = info.average_score();

        for e in events {
            match e {
                Event::StreakLifetimeRewarded { reward } => {
                    final_data.lifetime_percent_reward = reward;
                }
                Event::StreakFlatRewarded {
                    streak_id,
                    bonus_rating,
                    ..
                } => {
                    if streak_id == 0 {
                        final_data.weekly_streak_bonus = bonus_rating;
                    } else if streak_id == 1 {
                        final_data.monthly_streak_bonus = bonus_rating;
                    }
                }
                Event::ExecutedWithRating {
                    rating,
                    applied_multiplier,
                    pr_number_this_week,
                } => {
                    final_data.total_rating = rating;
                    final_data.total_lifetime_percent = applied_multiplier;
                    final_data.pr_number_this_week = pr_number_this_week;
                }
                Event::Autoscored { score } => {
                    final_data.score = score;
                    info.votes.push(Score {
                        user: context.bot_name.clone(),
                        score,
                    });
                }
                Event::NewSloth { .. } => {}
            }
        }

        context
            .status_message(pr, None, info.clone(), Some(final_data))
            .await;
        Ok(())
    }
}
