use rand::seq::SliceRandom;
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
        comment_id: Option<u64>,
        text: &[&str],
    ) -> anyhow::Result<Comment> {
        let text = text.choose(&mut rand::thread_rng()).ok_or_else(|| {
            anyhow::anyhow!("Failed to choose a random message from the list. Is it empty?")
        })?;

        if let Some(comment_id) = comment_id {
            self.github
                .like_comment(&pr_metadata.owner, &pr_metadata.repo, comment_id)
                .await?;
        }

        self.github
            .reply(
                &pr_metadata.owner,
                &pr_metadata.repo,
                pr_metadata.number,
                &format!("#### {text}"),
            )
            .await
    }

    // It does the same, but maybe later we will add some additional logic
    // And it makes visual separation between different types of replies
    pub async fn reply_with_error(
        &self,
        pr_metadata: &PrMetadata,
        error: &[&str],
    ) -> anyhow::Result<()> {
        self.reply(pr_metadata, None, error).await?;
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
