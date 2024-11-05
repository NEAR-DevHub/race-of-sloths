use shared::PRInfo;
use std::collections::HashMap;
use tracing::trace;

use crate::messages::MsgCategory;

use self::api::CommentRepr;

use super::*;

impl Context {
    pub async fn check_info(&self, repo_info: &RepoInfo) -> anyhow::Result<PRInfo> {
        self.near
            .check_info(&repo_info.owner, &repo_info.repo, repo_info.number)
            .await
    }

    pub async fn add_repo(&self, repo_info: &RepoInfo) -> anyhow::Result<()> {
        let result = self.near.add_repo(&repo_info.owner, &repo_info.repo).await;
        info!("Added repo {repo_info:?} to near");
        let message = format!(
            "New repo in the [{}](https://github.com/{}/{}/pull/{}) was {}",
            repo_info.full_id,
            repo_info.owner,
            repo_info.repo,
            repo_info.number,
            result.as_ref().map(|_| "added").unwrap_or("failed")
        );
        self.telegram.send_to_telegram(&message, &Level::INFO);

        result.map(|_| ())
    }

    pub async fn reply_with_text(
        &self,
        repo_info: &RepoInfo,
        comment_id: Option<u64>,
        text: &str,
    ) -> anyhow::Result<CommentRepr> {
        if let Some(comment_id) = comment_id {
            self.github
                .like_comment(&repo_info.owner, &repo_info.repo, comment_id)
                .await?;
        }

        self.github
            .reply(&repo_info.owner, &repo_info.repo, repo_info.number, text)
            .await
            .map(Into::into)
    }

    pub async fn reply(
        &self,
        repo_info: &RepoInfo,
        comment_id: Option<u64>,
        msg: MsgCategory,
        args: Vec<(&'static str, String)>,
    ) -> anyhow::Result<CommentRepr> {
        let text = self.messages.get_message(msg);

        let text = text.format(args.into_iter().collect::<HashMap<_, _>>())?;

        self.reply_with_text(repo_info, comment_id, &text).await
    }

    // It does the same, but maybe later we will add some additional logic
    // And it makes visual separation between different types of replies
    pub async fn reply_with_error(
        &self,
        repo_info: &RepoInfo,
        comment_id: Option<u64>,
        error: MsgCategory,
        args: Vec<(&'static str, String)>,
    ) -> anyhow::Result<()> {
        self.reply(repo_info, comment_id, error, args).await?;
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
        let mut iter = command.split_whitespace();
        let bot = iter.next();

        match bot {
            Some(bot) if bot == bot_name => (),
            _ => continue,
        }

        let command = iter.next();
        let args = iter.collect::<Vec<&str>>().join(" ");
        trace!("Extracted command: {command:?}, args: {args}");

        return Some((command.unwrap_or_default().to_string(), args));
    }

    None
}
