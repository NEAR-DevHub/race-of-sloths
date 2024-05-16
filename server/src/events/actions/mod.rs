mod finalize;
mod merge;
mod stale;

use super::*;

pub use finalize::*;
pub use merge::*;
pub use stale::*;
use tracing::error;

#[derive(Debug, Clone)]
pub enum Action {
    Finalize(PullRequestFinalize),
    Merge(PullRequestMerge),
    Stale(PullRequestStale),
}

impl Action {
    pub fn finalize(pr_metadata: PrMetadata) -> Self {
        Self::Finalize(PullRequestFinalize { pr_metadata })
    }

    pub fn merge(pr_metadata: PrMetadata) -> Option<Self> {
        PullRequestMerge::new(pr_metadata).map(Self::Merge)
    }

    pub fn stale(pr_metadata: PrMetadata) -> Self {
        Self::Stale(PullRequestStale { pr_metadata })
    }

    pub async fn execute(&self, context: Context, check_info: PRInfo) -> anyhow::Result<bool> {
        if check_info.excluded {
            error!("Shouldn't happening. PR({}) is excluded, so should be removed, but we tracked action for it...", self.pr().full_id);
            return Ok(false);
        }

        match self {
            Action::Finalize(action) => action.execute(context, check_info).await,
            Action::Merge(action) => action.execute(context, check_info).await,
            Action::Stale(action) => action.execute(context, check_info).await,
        }
    }

    pub fn pr(&self) -> &PrMetadata {
        match self {
            Action::Finalize(action) => &action.pr_metadata,
            Action::Merge(action) => &action.pr_metadata,
            Action::Stale(action) => &action.pr_metadata,
        }
    }
}
