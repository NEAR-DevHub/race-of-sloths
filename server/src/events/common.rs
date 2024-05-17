use std::collections::HashMap;

use tracing::trace;

use crate::messages::MsgCategory;

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
        msg: MsgCategory,
        args: Vec<(String, String)>,
    ) -> anyhow::Result<Comment> {
        let text = self
            .messages
            .get_message(msg)
            .ok_or_else(|| anyhow::anyhow!("Failed to get message for category: {msg}"))?;

        let text = text.format(args.into_iter().collect::<HashMap<_, _>>());

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
                &text,
            )
            .await
    }

    // It does the same, but maybe later we will add some additional logic
    // And it makes visual separation between different types of replies
    pub async fn reply_with_error(
        &self,
        pr_metadata: &PrMetadata,
        comment_id: Option<u64>,
        error: MsgCategory,
        args: Vec<(String, String)>,
    ) -> anyhow::Result<()> {
        self.reply(pr_metadata, comment_id, error, args).await?;
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
