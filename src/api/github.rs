use chrono::DateTime;
use octocrab::models::{
    self,
    activity::Notification,
    issues::{Comment, Issue},
    pulls::PullRequest,
    PullRequestId, Repository,
};

pub struct GithubClient {
    octocrab: octocrab::Octocrab,
}

impl GithubClient {
    pub fn new(github_token: String) -> anyhow::Result<Self> {
        let octocrab = octocrab::Octocrab::builder()
            .personal_token(github_token)
            .build()?;

        Ok(Self { octocrab })
    }

    pub async fn get_mentions(
        &self,
        since: chrono::DateTime<chrono::Utc>,
    ) -> anyhow::Result<(Vec<Notification>, DateTime<chrono::Utc>)> {
        log::debug!("Getting mentions since: {:?}", since);
        let page = self
            .octocrab
            .activity()
            .notifications()
            .list()
            .all(true)
            .participating(true)
            .per_page(50)
            .since(since)
            .page(0)
            .send()
            .await?;

        let mut updated_at = since;
        let events = self.octocrab.all_pages(page).await?;
        let results = events
            .into_iter()
            .filter(|notification| {
                updated_at = updated_at.max(notification.updated_at);
                notification.reason == "mention" && notification.subject.r#type == "PullRequest"
            })
            .collect();

        Ok((results, updated_at))
    }

    pub async fn get_comment(&self, notification: &Notification) -> anyhow::Result<Comment> {
        assert_eq!(notification.reason, "mention");
        log::debug!(
            "Getting comment: {:?}",
            notification.subject.latest_comment_url.as_ref().unwrap()
        );
        let comment = self
            .octocrab
            .get(
                notification
                    .subject
                    .latest_comment_url
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("No comment url"))?,
                None::<&()>,
            )
            .await?;

        Ok(comment)
    }

    pub async fn get_pull_request(
        &self,
        notification: &Notification,
    ) -> anyhow::Result<PullRequest> {
        assert_eq!(notification.subject.r#type, "PullRequest");
        log::debug!(
            "Getting PR: {:?}",
            notification.subject.url.as_ref().unwrap()
        );
        let pull_request = self
            .octocrab
            .get(
                notification
                    .subject
                    .url
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("No PR url"))?,
                None::<&()>,
            )
            .await?;

        Ok(pull_request)
    }

    pub async fn reply(&self, repo: &Repository, id: u64, text: &str) -> anyhow::Result<()> {
        log::debug!(
            "Replying to PR {}/{}/{id} with: {text}",
            repo.owner.as_ref().unwrap().login,
            repo.name,
        );
        self.octocrab
            .issues(
                repo.owner
                    .as_ref()
                    .map(|a| a.login.as_str())
                    .ok_or_else(|| anyhow::anyhow!("No login"))?,
                &repo.name,
            )
            .create_comment(id, text)
            .await?;

        Ok(())
    }
}
