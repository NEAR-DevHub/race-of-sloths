use shared::{
    github::{RepoInfo, User},
    PRInfo,
};
use tracing::{info, instrument};

use crate::{api::CommentRepr, messages::MsgCategory};

use super::{common, pr_commands::BotUnpaused, Context, EventResult};

#[derive(Debug, Clone)]
pub enum Command {
    Unpause(BotUnpaused),
}

impl Command {
    pub fn parse_command(bot_name: &str, comment: &CommentRepr) -> Option<Command> {
        let (command, _args) = common::extract_command_with_args(bot_name, comment)?;

        Some(match command.as_str() {
            "yes" | "approve" | "add" | "accept" => Command::Unpause(BotUnpaused {
                timestamp: comment.timestamp,
                comment_id: comment.comment_id,
                from_issue: true,
            }),
            _ => return None,
        })
    }

    pub fn timestamp(&self) -> &chrono::DateTime<chrono::Utc> {
        match self {
            Command::Unpause(event) => &event.timestamp,
        }
    }

    #[instrument(skip(self, context, repo_info), fields(pr = repo_info.full_id))]
    pub async fn execute(
        &self,
        repo_info: &RepoInfo,
        context: Context,
        // TODO: it's a bit weird as we don't have a PR here, but it's for pause/unpause commands
        check_info: &mut PRInfo,
        first_reply: bool,
        sender: &User,
    ) -> anyhow::Result<EventResult> {
        if check_info.blocked_repo {
            info!(
                "Sloth called for a PR from blocked repo: {}. Skipping",
                repo_info.full_id
            );
            if first_reply {
                context
                    .reply_with_error(
                        repo_info,
                        None,
                        MsgCategory::ErrorRepoIsBanned,
                        vec![("pr_author_username", sender.login.clone())],
                    )
                    .await?;
                return Ok(EventResult::RepliedWithError);
            }

            return Ok(EventResult::Skipped);
        }

        match self {
            Command::Unpause(event) => event.execute(repo_info, context, check_info, sender).await,
        }
    }
}

impl std::fmt::Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Command::Unpause(_) => {
                write!(f, "Repository approved")
            }
        }
    }
}
