use shared::PRInfo;
use std::collections::HashMap;
use tracing::trace;

use crate::messages::MsgCategory;

use self::api::CommentRepr;

use super::*;

impl Context {
    pub async fn check_info(&self, pr_metadata: &PrMetadata) -> anyhow::Result<PRInfo> {
        self.near
            .check_info(&pr_metadata.owner, &pr_metadata.repo, pr_metadata.number)
            .await
    }

    pub async fn reply_with_text(
        &self,
        pr_metadata: &PrMetadata,
        comment_id: Option<u64>,
        text: &str,
    ) -> anyhow::Result<CommentRepr> {
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
                text,
            )
            .await
            .map(Into::into)
    }

    pub async fn reply(
        &self,
        pr_metadata: &PrMetadata,
        comment_id: Option<u64>,
        msg: MsgCategory,
        args: Vec<(&'static str, String)>,
    ) -> anyhow::Result<CommentRepr> {
        let text = self.messages.get_message(msg);

        let text = text.format(args.into_iter().collect::<HashMap<_, _>>())?;

        self.reply_with_text(pr_metadata, comment_id, &text).await
    }

    // It does the same, but maybe later we will add some additional logic
    // And it makes visual separation between different types of replies
    pub async fn reply_with_error(
        &self,
        pr_metadata: &PrMetadata,
        comment_id: Option<u64>,
        error: MsgCategory,
        args: Vec<(&'static str, String)>,
    ) -> anyhow::Result<()> {
        self.reply(pr_metadata, comment_id, error, args).await?;
        Ok(())
    }
}

pub fn extract_command_with_args(
    bot_name: &str,
    comment: &CommentRepr,
) -> Option<(String, String)> {
    let bot_name = format!("@{}", bot_name);

    for command in comment.text.lines() {
        let command = command.trim();
        if !command.starts_with(&bot_name) {
            continue;
        }

        let mut iter = command.split_whitespace();
        let _bot = iter.next();
        let command = iter.next();
        let args = iter.collect::<Vec<&str>>().join(" ");
        trace!("Extracted command: {command:?}, args: {args}");

        return Some((command.unwrap_or_default().to_string(), args));
    }

    None
}
