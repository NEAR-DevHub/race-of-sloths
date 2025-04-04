use tracing::{error, instrument};

use shared::{github::PrMetadata, GithubHandle, PRInfo};

use crate::{events::Context, messages::MsgCategory};

use super::EventResult;

#[derive(Debug, Clone)]
pub struct PullRequestMerge {
    pub merger: GithubHandle,
    pub reviewers: Vec<GithubHandle>,
}

impl PullRequestMerge {
    #[instrument(skip(self, pr, context, info), fields(pr = pr.repo_info.full_id))]
    pub async fn execute(
        &self,
        pr: &PrMetadata,
        context: Context,
        info: &mut PRInfo,
    ) -> anyhow::Result<EventResult> {
        if info.merged {
            return Ok(EventResult::Skipped);
        }

        context.near.send_merge(pr).await?;
        info.merged = true;

        if info.paused_repo || info.blocked_repo {
            return Ok(EventResult::success(false));
        }

        if !info.votes.is_empty() {
            return Ok(EventResult::success(true));
        }

        let (scores, is_active) = context
            .github
            .get_scores_and_active_pr_status(&pr)
            .await
            .unwrap_or_default();

        let autoscore = if is_active { 2 } else { 1 }.to_string();

        if !scores.is_empty() {
            let scores_len = scores.len();
            for (index, (command, user)) in scores.into_iter().enumerate() {
                let result = command
                    .muted()
                    .execute(pr, context.clone(), info, &user)
                    .await;
                if let Err(e) = result {
                    error!(
                        "Failed to apply previously applied score {}/{}: {}",
                        index, scores_len, e
                    );
                }
            }
        } else if self.merger != pr.author.login {
            context
                .reply(
                    &pr.repo_info,
                    None,
                    MsgCategory::MergeWithoutScoreMessageByOtherParty,
                    vec![
                        ("maintainer", self.merger.clone()),
                        ("potential_score", autoscore),
                    ],
                )
                .await?;
        } else if !self.reviewers.is_empty() {
            context
                .reply(
                    &pr.repo_info,
                    None,
                    MsgCategory::MergeWithoutScoreMessageByOtherParty,
                    vec![
                        ("maintainer", self.reviewers.join(" @")),
                        ("potential_score", autoscore),
                    ],
                )
                .await?;
        } else {
            context
                .reply(
                    &pr.repo_info,
                    None,
                    MsgCategory::MergeWithoutScoreMessageByAuthorWithoutReviewers,
                    vec![
                        ("pr_author_username", pr.author.login.clone()),
                        ("potential_score", autoscore),
                    ],
                )
                .await?;
        }

        Ok(EventResult::success(true))
    }
}
