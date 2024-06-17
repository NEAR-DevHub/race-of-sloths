use shared::github::User;

use crate::messages::MsgCategory;

use super::*;

pub mod exclude;
pub mod pause;
pub mod score;
pub mod start;
pub mod unknown;

pub use self::{exclude::*, pause::*, score::*, start::*, unknown::*};

#[derive(Debug, Clone)]
pub enum Command {
    Include(BotIncluded),
    Score(BotScored),
    Pause(BotPaused),
    Unpause(BotUnpaused),
    Excluded(BotExcluded),
    Unknown(UnknownCommand),
}

impl Command {
    pub fn parse_command(
        bot_name: &str,
        pr_metadata: &PrMetadata,
        comment: &Comment,
    ) -> Option<Command> {
        let (command, args) = common::extract_command_with_args(bot_name, comment)?;

        Some(match command.as_str() {
            "score" | "rate" | "value" => BotScored::construct(comment, args),
            "pause" | "block" => BotPaused::construct(comment),
            "unpause" | "unblock" => BotUnpaused::construct(comment),
            "exclude" | "leave" => BotExcluded::construct(comment),
            "include" | "in" | "start" | "join" => BotIncluded::construct(comment),

            _ => {
                info!(
                    "Unknown command: {} for PR: {}",
                    command, pr_metadata.full_id
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
        }
    }

    #[instrument(skip(self, context, check_info, pr), fields(pr = pr.full_id))]
    pub async fn execute(
        &self,
        pr: &PrMetadata,
        context: Context,
        check_info: PRInfo,
        sender: &User,
        first_reply: bool,
    ) -> anyhow::Result<bool> {
        if !check_info.allowed_org {
            info!(
                "Sloth called for a PR from not allowed org: {}. Skipping",
                pr.full_id
            );
            context
                .reply_with_error(
                    pr,
                    None,
                    MsgCategory::ErrorOrgNotInAllowedListMessage,
                    vec![("pr_author_username".to_string(), pr.author.login.clone())],
                )
                .await?;

            return Ok(false);
        }

        if check_info.executed {
            info!(
                "Sloth called for a PR that is already executed: {}. Skipping",
                pr.full_id
            );
            if let Command::Score(event) = self {
                context
                    .reply_with_error(
                        pr,
                        Some(event.comment_id),
                        MsgCategory::ErrorLateScoringMessage,
                        vec![],
                    )
                    .await?;
            }

            return Ok(false);
        }

        if !check_info.allowed_repo && !matches!(self, Command::Unpause(_) | Command::Pause(_)) {
            info!(
                "Sloth called for a PR from paused repo: {}. Skipping",
                pr.full_id
            );

            if first_reply {
                context
                    .reply_with_error(
                        pr,
                        None,
                        MsgCategory::ErrorPausedMessage,
                        vec![("user".to_string(), sender.login.clone())],
                    )
                    .await?;
            }

            return Ok(false);
        }

        if check_info.excluded && !matches!(self, Command::Include(_)) {
            info!(
                "Sloth called for a PR from excluded PR: {}. Skipping",
                pr.full_id
            );

            return Ok(false);
        }

        match self {
            Command::Include(event) => event.execute(pr, context, check_info, sender).await,
            Command::Score(event) => event.execute(pr, context, check_info, sender).await,
            Command::Pause(event) => event.execute(pr, context, check_info, sender).await,
            Command::Unpause(event) => event.execute(pr, context, check_info, sender).await,
            Command::Excluded(event) => event.execute(pr, context, check_info).await,
            Command::Unknown(event) => event.execute(pr, context, check_info, sender).await,
        }
    }
}

#[cfg(test)]
pub mod tests {
    use octocrab::models::issues::Comment;
    use shared::github::{PrMetadata, User};

    use super::Command;

    fn generate_comment(text: &str) -> Comment {
        let string = format!(
            r#"
            {{
            "id": 222,
            "node_id": "111",
            "url": "https://example.com/comment",
            "html_url": "https://example.com/comment/html",
            "issue_url": "https://example.com/issue",
            "body": "{text}",
            "author_association": "CONTRIBUTOR",
            "user": {{
              "login": "username",
              "id": 333,
              "node_id": "111",
              "avatar_url": "https://example.com/avatar",
              "gravatar_id": "",
              "url": "https://example.com/user",
              "html_url": "https://example.com/user/html",
              "followers_url": "https://example.com/user/followers",
              "following_url": "https://example.com/user/following",
              "gists_url": "https://example.com/user/gists",
              "starred_url": "https://example.com/user/starred",
              "subscriptions_url": "https://example.com/user/subscriptions",
              "organizations_url": "https://example.com/user/orgs",
              "repos_url": "https://example.com/user/repos",
              "events_url": "https://example.com/user/events",
              "received_events_url": "https://example.com/user/received_events",
              "type": "User",
              "site_admin": false
            }},
            "created_at": "2023-01-01T00:00:00Z",
            "updated_at": "2023-01-02T00:00:00Z"
          }}
          "#
        );

        serde_json::from_str(&string).unwrap()
    }

    fn generate_command_comment(command: &str) -> Comment {
        generate_comment(&format!("@{NAME} {command}"))
    }

    const NAME: &str = "@name";

    fn default_pr_metadata() -> PrMetadata {
        PrMetadata {
            owner: "a".to_string(),
            repo: "b".to_string(),
            number: 1,
            author: User::new(
                "a-u".to_string(),
                octocrab::models::AuthorAssociation::Contributor,
            ),
            started: chrono::Utc::now(),
            merged: None,
            updated_at: chrono::Utc::now(),
            full_id: "a/b/1".to_string(),
            body: "abc".to_string(),
            closed: false,
        }
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
        let aliases = vec!["score", "rate", "value", "score 12"];
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
