use tracing::trace;

use self::api::near::PRInfo;

use super::*;

impl Context {
    pub async fn check_info(&self, pr_metadata: &PrMetadata) -> anyhow::Result<PRInfo> {
        self.near
            .check_info(&pr_metadata.owner, &pr_metadata.repo, pr_metadata.number)
            .await
    }

    pub async fn reply(
        &self,
        owner: &str,
        repo: &str,
        number: u64,
        comment_id: u64,
        text: &str,
    ) -> anyhow::Result<Comment> {
        self.github.like_comment(owner, repo, comment_id).await?;
        self.github.reply(owner, repo, number, text).await
    }

    pub async fn reply_with_error(
        &self,
        owner: &str,
        repo: &str,
        number: u64,
        error: &str,
    ) -> anyhow::Result<()> {
        self.github
            .reply(
                owner,
                repo,
                number,
                &format!("Hey, I'm sorry, but I can't process that: {}", error),
            )
            .await?;
        Ok(())
    }
}

pub fn extract_command_with_args(bot_name: &str, comment: &Comment) -> Option<(String, String)> {
    let body = comment
        .body
        .as_ref()
        .or(comment.body_html.as_ref())
        .or(comment.body_text.as_ref())?
        .to_lowercase();

    let bot_name = format!("@{} ", bot_name);
    let position = body.find(&bot_name)?;

    let commands = body[position + bot_name.len()..]
        .split_whitespace()
        .collect::<Vec<&str>>();

    let command = commands[0].to_string();
    let args = commands[1..].join(" ");

    trace!("Extracted command: {command}, args: {args}");

    Some((command, args))
}
