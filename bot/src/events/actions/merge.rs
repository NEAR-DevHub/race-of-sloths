use tracing::instrument;

use shared::{github::PrMetadata, GithubHandle, PRInfo};

use crate::{events::Context, messages::MsgCategory};

use super::EventResult;

#[derive(Debug, Clone)]
pub struct PullRequestMerge {
    pub merger: GithubHandle,
    pub reviewers: Vec<GithubHandle>,
}

impl PullRequestMerge {
    #[instrument(skip(self, pr, context, info), fields(pr = pr.full_id))]
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

        if !info.allowed_repo || info.paused {
            return Ok(EventResult::success(false));
        }

        if !info.votes.is_empty() {
            return Ok(EventResult::success(true));
        }

        let is_active = context
            .github
            .is_active_pr(&pr.owner, &pr.repo, &pr.author.login, pr.number)
            .await
            .unwrap_or_default();
        let autoscore = if is_active { 2 } else { 1 }.to_string();

        if self.merger != pr.author.login {
            context
                .reply(
                    pr,
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
                    pr,
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
                    pr,
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
