use self::api::github::{BotStarted, PrMetadata};

use super::*;

fn msg(user: &str) -> String {
    format!("This pull request is a part of Sloth race now. Dear maintainer, please use `@{user} score [1-10]` to rate it, or `@{user} pause` to stop the sloth for the repository.")
}

#[async_trait::async_trait]
impl Execute for api::github::BotStarted {
    async fn execute(&self, context: Context) -> anyhow::Result<()> {
        let info = context.check_info(&self.pr_metadata).await?;
        if info.exist || !info.allowed {
            return context
                .github
                .mark_notification_as_read(self.notification_id)
                .await;
        }

        context.near.send_start(&self.pr_metadata).await?;

        context
            .github
            .subscribe_to_repo(&self.pr_metadata.owner, &self.pr_metadata.repo)
            .await?;
        context
            .github
            .reply(
                &self.pr_metadata.owner,
                &self.pr_metadata.repo,
                self.pr_metadata.number,
                &msg(&context.github.user_handle),
            )
            .await?;
        context
            .github
            .like_comment(
                &self.pr_metadata.owner,
                &self.pr_metadata.repo,
                self.comment_id,
            )
            .await?;
        context
            .github
            .mark_notification_as_read(self.notification_id)
            .await
    }
}

impl ParseComment for api::github::BotStarted {
    type Command = api::github::BotStarted;

    fn parse_comment(
        bot_name: &str,
        notification: &Notification,
        pr_metadata: &PrMetadata,
        comment: &Comment,
    ) -> Option<Self::Command> {
        let body = comment
            .body
            .as_ref()
            .or(comment.body_html.as_ref())
            .or(comment.body_text.as_ref())
            .cloned()
            .unwrap_or_default();

        if body.contains(format!("@{} include", bot_name).as_str()) {
            Some(BotStarted::new(
                comment.user.login.clone(),
                pr_metadata.clone(),
                notification.updated_at,
                comment.id.0,
                notification.id.0,
            ))
        } else {
            None
        }
    }
}
