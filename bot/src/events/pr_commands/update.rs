use tracing::instrument;

use shared::github::User;

use super::*;

#[derive(Debug, Clone)]
pub struct BotUpdated {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub user_comment_id: Option<u64>,
}

impl BotUpdated {
    pub fn new(timestamp: chrono::DateTime<chrono::Utc>, comment_id: Option<u64>) -> Self {
        Self {
            timestamp,
            user_comment_id: comment_id,
        }
    }

    #[instrument(skip(self, _pr, _context, _info, _sender), fields(pr = _pr.repo_info.full_id))]
    pub async fn execute(
        &self,
        _pr: &PrMetadata,
        _context: Context,
        _info: &mut PRInfo,
        _sender: &User,
    ) -> anyhow::Result<EventResult> {
        return Ok(EventResult::success(true));
    }

    pub fn construct(comment: &CommentRepr) -> Command {
        Command::Update(BotUpdated::new(comment.timestamp, comment.comment_id))
    }
}
