use serde::Deserialize;
use slothrace::{
    api::github::{types::Event, GithubClient},
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
        let (events, max_time) = github_api.get_events(updated_from).await?;
        log::info!("Got {} events", events.len());

        for event in events {
            match event {
                Event::BotStarted(bot_started) => {
                    let message = if bot_started.is_accepted() {
                        format!(
                        "Hey hey. You called me, @{}? :).\nKeep up the good work @{}!\nDear maintainer please score this PR when the time comes to merge it with given syntax `@akorchyn score 1/2/3/4/5`. Please note that I will ignore incorrectly provided messages to not spam!",
                        bot_started.sender,
                        bot_started.pr_metadata.author.login,
                    )
                    } else {
                        format!(
                            "Hey hey. You called me, @{}? :).\nI'm sorry but maintainers and members can't get rewarded for work in their own projects.!",
                            bot_started.sender
                        )
                    };
                    github_api
                        .reply(
                            &bot_started.pr_metadata.owner,
                            &bot_started.pr_metadata.repo,
                            bot_started.pr_metadata.number,
                            &message,
                        )
                        .await
                        .unwrap()
                }
                Event::BotScored(bot_scored) => {
                    let message = if bot_scored.is_valid_score() {
                        format!(
                            "Hey hey.\nThank you for scoring @{}'s PR with {}!",
                            bot_scored.pr_metadata.author.login, bot_scored.score
                        )
                    } else {
                        "Hey hey :).\nAre you sure that you are the one who is able to score? :)"
                            .to_string()
                    };
                    github_api
                        .reply(
                            &bot_scored.pr_metadata.owner,
                            &bot_scored.pr_metadata.repo,
                            bot_scored.pr_metadata.number,
                            &message,
                        )
                        .await
                        .unwrap()
                }
                Event::PullRequestMerged(pull_request_merged) => {
                    let message = format!(
                        "Hey hey.\nCongratulations @{}! Your PR has been merged!",
                        pull_request_merged.pr_metadata.author.login
                    );
                    github_api
                        .reply(
                            &pull_request_merged.pr_metadata.owner,
                            &pull_request_merged.pr_metadata.repo,
                            pull_request_merged.pr_metadata.number,
                            &message,
                        )
                        .await
                        .unwrap()
                }
            }
        }
        updated_from = max_time;
    }

    Ok(())
}
