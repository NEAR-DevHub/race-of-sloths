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
        pr_metadata: &PrMetadata,
        comment_id: u64,
        text: &str,
    ) -> anyhow::Result<Comment> {
        self.github
            .like_comment(&pr_metadata.owner, &pr_metadata.repo, comment_id)
            .await?;
        self.github
            .reply(
                &pr_metadata.owner,
                &pr_metadata.repo,
                pr_metadata.number,
                &format!("#### {text}"),
            )
            .await
    }

    pub async fn reply_with_error(
        &self,
        pr_metadata: &PrMetadata,
        error: &str,
    ) -> anyhow::Result<()> {
        self.github
            .reply(
                &pr_metadata.owner,
                &pr_metadata.repo,
                pr_metadata.number,
                &format!("#### {error}"),
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

    if commands.is_empty() {
        return Some((String::new(), String::new()));
    }

    let command = commands[0].to_string();
    let args = commands[1..].join(" ");

    trace!("Extracted command: {command}, args: {args}");

    Some((command, args))
}
