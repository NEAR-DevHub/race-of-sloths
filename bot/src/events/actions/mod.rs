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

impl std::fmt::Display for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Action::Finalize(_) => write!(f, "Finalize"),
            Action::Merge(_) => write!(f, "Merge"),
            Action::Stale(_) => write!(f, "Stale"),
        }
    }
}

impl Action {
    pub fn finalize() -> Self {
        Self::Finalize(PullRequestFinalize {})
    }

    pub fn merge() -> Self {
        Self::Merge(PullRequestMerge {})
    }

    pub fn stale() -> Self {
        Self::Stale(PullRequestStale {})
    }

    pub async fn execute(
        &self,
        pr: &PrMetadata,
        context: Context,
        check_info: &mut PRInfo,
    ) -> anyhow::Result<EventResult> {
        if check_info.excluded {
            error!("Shouldn't happening. PR({}) is excluded, so should be removed, but we tracked action for it...", pr.full_id);
            return Ok(EventResult::Skipped);
        }
        if !check_info.exist {
            // Parsed notification but we weren't called before to include us
            return Ok(EventResult::Skipped);
        }

        match self {
            Action::Finalize(action) => action.execute(pr, context, check_info).await,
            Action::Merge(action) => action.execute(pr, context, check_info).await,
            Action::Stale(action) => action.execute(pr, context, check_info).await,
        }
    }
}
