use shared::github::User;
use update::BotUpdated;

use crate::messages::MsgCategory;

use super::*;

pub mod exclude;
pub mod pause;
pub mod score;
pub mod start;
pub mod unknown;
pub mod update;

use self::api::CommentRepr;
pub use self::{exclude::*, pause::*, score::*, start::*, unknown::*};

#[derive(Debug, Clone)]
pub enum Command {
    Include(BotIncluded),
    Score(BotScored),
    Pause(BotPaused),
    Unpause(BotUnpaused),
    Excluded(BotExcluded),
    Unknown(UnknownCommand),
    Update(BotUpdated),
}

impl Command {
    pub fn parse_command(
        bot_name: &str,
        pr_metadata: &PrMetadata,
        comment: &CommentRepr,
    ) -> Option<Command> {
        let (command, args) = common::extract_command_with_args(bot_name, comment)?;

        Some(match command.as_str() {
            "score" | "score:" | "rate" | "value" => BotScored::construct(comment, args),
            "pause" | "block" => BotPaused::construct(comment),
            "unpause" | "resume" | "unblock" => BotUnpaused::construct(comment),
            "exclude" | "leave" => BotExcluded::construct(comment),
            "include" | "in" | "start" | "join" | "invite" => BotIncluded::construct(comment),
            "update" => BotUpdated::construct(comment),
            _ if command.chars().all(char::is_numeric) && !command.is_empty() => {
                BotScored::construct(comment, command)
            }
            _ => {
                info!(
                    "Unknown command: {} for PR: {}",
                    command, pr_metadata.repo_info.full_id
                );
                UnknownCommand::construct(comment, command, args)
            }
        })
    }

    pub fn parse_body(bot_name: &str, pr_metadata: &PrMetadata) -> Option<Command> {
        BotIncluded::parse_body(bot_name, pr_metadata)
    }

    pub fn timestamp(&self) -> &chrono::DateTime<chrono::Utc> {
        match self {
            Command::Include(event) => &event.timestamp,
            Command::Score(event) => &event.timestamp,
            Command::Pause(event) => &event.timestamp,
            Command::Unpause(event) => &event.timestamp,
            Command::Excluded(event) => &event.timestamp,
            Command::Unknown(event) => &event.timestamp,
            Command::Update(event) => &event.timestamp,
        }
    }

    #[instrument(skip(self, context, check_info, pr), fields(pr = pr.repo_info.full_id))]
    pub async fn execute(
        &self,
        pr: &PrMetadata,
        context: Context,
        check_info: &mut PRInfo,
        sender: &User,
        first_reply: bool,
    ) -> anyhow::Result<EventResult> {
        if !check_info.allowed_repo {
            info!(
                "Sloth called for a PR from not allowed org: {}. Skipping",
                pr.repo_info.full_id
            );
            if first_reply {
                context
                    .reply_with_error(
                        &pr.repo_info,
                        None,
                        MsgCategory::ErrorOrgNotInAllowedListMessage,
                        vec![("pr_author_username", pr.author.login.clone())],
                    )
                    .await?;
                return Ok(EventResult::RepliedWithError);
            }

            return Ok(EventResult::Skipped);
        }

        if check_info.paused && !matches!(self, Command::Unpause(_) | Command::Pause(_)) {
            info!(
                "Sloth called for a PR from paused repo: {}. Skipping",
                pr.repo_info.full_id
            );

            if first_reply {
                context
                    .reply_with_error(
                        &pr.repo_info,
                        None,
                        MsgCategory::ErrorPausedMessage,
                        vec![("user", sender.login.clone())],
                    )
                    .await?;
                return Ok(EventResult::RepliedWithError);
            }

            return Ok(EventResult::Skipped);
        }

        if check_info.executed {
            info!(
                "Sloth called for a PR that is already executed: {}. Skipping",
                pr.repo_info.full_id
            );
            if let Command::Score(event) = self {
                context
                    .reply_with_error(
                        &pr.repo_info,
                        event.comment_id,
                        MsgCategory::ErrorLateScoringMessage,
                        vec![],
                    )
                    .await?;
            }

            return Ok(EventResult::RepliedWithError);
        }

        if check_info.excluded && !matches!(self, Command::Include(_)) {
            info!(
                "Sloth called for a PR from excluded PR: {}. Skipping",
                pr.repo_info.full_id
            );

            return Ok(EventResult::Skipped);
        }

        match self {
            Command::Include(event) => event.execute(pr, context, check_info, sender).await,
            Command::Score(event) => event.execute(pr, context, check_info, sender).await,
            Command::Pause(event) => event.execute(pr, context, check_info, sender).await,
            Command::Unpause(event) => {
                event
                    .execute(&pr.repo_info, context, check_info, sender)
                    .await
            }
            Command::Excluded(event) => event.execute(pr, context, check_info).await,
            Command::Unknown(event) => event.execute(pr, context, check_info, sender).await,
            Command::Update(event) => event.execute(pr, context, check_info, sender).await,
        }
    }
}

impl std::fmt::Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Command::Include(_) => write!(f, "Include"),
            Command::Score(_) => write!(f, "Score"),
            Command::Pause(_) => write!(f, "Pause"),
            Command::Unpause(_) => write!(f, "Unpause"),
            Command::Excluded(_) => write!(f, "Excluded"),
            Command::Unknown(_) => write!(f, "Unknown"),
            Command::Update(_) => write!(f, "Update"),
        }
    }
}

#[cfg(test)]
pub mod tests {
    use shared::github::{PrMetadata, RepoInfo, User};

    use crate::api::CommentRepr;

    use super::Command;

    fn generate_comment(text: &str) -> CommentRepr {
        CommentRepr {
            id: 0,
            user: User::new(
                "username".to_string(),
                octocrab::models::AuthorAssociation::Contributor,
            ),
            timestamp: chrono::Utc::now(),
            comment_id: Some(111),
            text: text.to_string(),
        }
    }

    fn generate_command_comment(command: &str) -> CommentRepr {
        generate_comment(&format!("@{NAME} {command}"))
    }

    const NAME: &str = "@name";

    fn default_pr_metadata() -> PrMetadata {
        PrMetadata {
            repo_info: RepoInfo {
                owner: "a".to_string(),
                repo: "b".to_string(),
                number: 1,
                full_id: "a/b/1".to_string(),
            },
            author: User::new(
                "a-u".to_string(),
                octocrab::models::AuthorAssociation::Contributor,
            ),
            created: chrono::Utc::now(),
            merged: None,
            updated_at: chrono::Utc::now(),
            body: "abc".to_string(),
            closed: false,
        }
    }

    #[test]
    pub fn dont_parse_in_the_middle() {
        let comment = generate_comment(&format!("Hello @{NAME} world"));
        let command = Command::parse_command(NAME, &default_pr_metadata(), &comment);

        assert!(command.is_none());
    }

    #[test]
    pub fn multi_line_comment_works() {
        let comment = generate_comment(&format!(
            "Hello buddy, i'm working here on super duper pr.\nAhey\n   @{NAME} score 8"
        ));
        let command = Command::parse_command(NAME, &default_pr_metadata(), &comment);

        assert!(matches!(command, Some(Command::Score(_))));
    }

    #[test]
    pub fn correct_include() {
        let aliases = vec!["include", "in", "start", "join"];
        for alias in aliases {
            let include_comment = generate_command_comment(alias);
            let command =
                Command::parse_command(NAME, &default_pr_metadata(), &include_comment).unwrap();

            assert!(matches!(command, Command::Include(_)))
        }
    }

    #[test]
    pub fn correct_score() {
        let aliases = vec!["score", "rate", "value", "score 12", "12"];
        for alias in aliases {
            let score_comment = generate_command_comment(alias);
            let command =
                Command::parse_command(NAME, &default_pr_metadata(), &score_comment).unwrap();

            assert!(matches!(command, Command::Score(_)))
        }
    }

    #[test]
    pub fn correct_pause() {
        let aliases = vec!["pause", "block"];
        for alias in aliases {
            let pause_comment = generate_command_comment(alias);
            let command =
                Command::parse_command(NAME, &default_pr_metadata(), &pause_comment).unwrap();

            assert!(matches!(command, Command::Pause(_)))
        }
    }

    #[test]
    pub fn correct_unpause() {
        let aliases = vec!["unpause", "unblock"];
        for alias in aliases {
            let unpause_comment = generate_command_comment(alias);
            let command =
                Command::parse_command(NAME, &default_pr_metadata(), &unpause_comment).unwrap();

            assert!(matches!(command, Command::Unpause(_)))
        }
    }

    #[test]
    pub fn correct_exclude() {
        let aliases = vec!["exclude", "leave"];
        for alias in aliases {
            let exclude_comment = generate_command_comment(alias);
            let command =
                Command::parse_command(NAME, &default_pr_metadata(), &exclude_comment).unwrap();

            assert!(matches!(command, Command::Excluded(_)))
        }
    }

    #[test]
    pub fn correct_unknown() {
        let aliases = vec!["", "asdasdasdas", "hello workld"];
        for alias in aliases {
            let unknown_command = generate_command_comment(alias);
            let command =
                Command::parse_command(NAME, &default_pr_metadata(), &unknown_command).unwrap();

            assert!(matches!(command, Command::Unknown(_)))
        }

        let unknown_command = generate_comment(&format!("@{NAME}"));
        let command =
            Command::parse_command(NAME, &default_pr_metadata(), &unknown_command).unwrap();

        assert!(matches!(command, Command::Unknown(_)))
    }
}
