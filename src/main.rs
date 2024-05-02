use serde::Deserialize;
use slothrace::{
    api::github::GithubClient,
    consts::{SCORE_PHRASE, START_PHRASE},
};

#[derive(Deserialize)]
struct Env {
    github_token: String,
    contract: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    env_logger::init();

    let env = envy::from_env::<Env>()?;

    let github_api = GithubClient::new(env.github_token)?;

    let minute = tokio::time::Duration::from_secs(60);
    let mut interval: tokio::time::Interval = tokio::time::interval(minute);

    // TODO: load from database last time we checked
    let mut updated_from = chrono::Utc::now() - chrono::Duration::days(1);
    loop {
        interval.tick().await;
        let (events, max_time) = github_api.get_mentions(updated_from).await?;
        log::info!("Got {} events", events.len());

        for event in events {
            let comment = github_api.get_comment(&event).await?;
            // TODO:  research if we need to parse body, body_text and body_html
            let body = comment.body;
            if body.is_none() {
                log::debug!("No body in comment");
                continue;
            }
            let pr = github_api.get_pull_request(&event).await?;
            let author = pr.user;
            if author.is_none() {
                log::debug!("No author in PR");
                continue;
            }

            let body = body.unwrap();
            let body = body.trim();
            // TODO: proper message string
            if body.starts_with(START_PHRASE) {
                log::debug!("Found start phrase in comment: {}", body);

                let message = format!(
                    "Hey hey. You called me ? :).\nKeep up the good work {}!\nDear maintainer please score this PR when the time comes to merge it with given syntax `@akorchyn score 1/2/3/4/5. Please note that I will ignore incorrectly provided messages to not spam!",
                    author.unwrap().login,
                );
                github_api
                    .reply(&event.repository, pr.number, &message)
                    .await?;
            } else if body.starts_with(SCORE_PHRASE) {
                log::debug!("Found score phrase in comment: {}", body);
                let message = format!("Good job {}! :)", author.unwrap().login,);
                github_api
                    .reply(&event.repository, pr.number, &message)
                    .await?;
            } else {
                log::debug!("No phrase in comment: {}", body);
            }
        }
        updated_from = max_time;
    }

    Ok(())
}
