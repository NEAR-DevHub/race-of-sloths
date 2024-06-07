#[macro_use]
extern crate rocket;

mod entrypoints;

use std::sync::Arc;
use std::time::Duration;

use entrypoints::ApiDoc;
use shared::near::NearClient;

use race_of_sloths_server::{contract_pull, db, github_pull};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

#[derive(Debug, serde::Deserialize)]
pub struct Env {
    contract: String,
    secret_key: String,
    is_mainnet: bool,
    near_timeout_in_minutes: Option<u32>,
    github_timeout_in_minutes: Option<u32>,
    github_token: String,
}

#[launch]
async fn rocket() -> _ {
    dotenv::dotenv().ok();

    let env = envy::from_env::<Env>().expect("Failed to load environment variables");
    let near_sleep = Duration::from_secs(env.near_timeout_in_minutes.unwrap_or(10) as u64 * 60);
    let github_sleep = Duration::from_secs(env.github_timeout_in_minutes.unwrap_or(60) as u64 * 60);
    let atomic_bool = Arc::new(std::sync::atomic::AtomicBool::new(true));
    let prometheus = rocket_prometheus::PrometheusMetrics::new();

    let near_client = NearClient::new(env.contract.clone(), env.secret_key.clone(), env.is_mainnet)
        .await
        .expect("Failed to create Near client");
    // TODO: after 0.6.0 release, we should use tracing for redirecting warns and errors to the telegram

    rocket::build()
        .attach(db::stage())
        .attach(contract_pull::stage(
            near_client,
            near_sleep,
            atomic_bool.clone(),
        ))
        .attach(github_pull::stage(
            github_pull::GithubClient::new(env.github_token.clone())
                .expect("Failed to create Github client"),
            github_sleep,
            atomic_bool.clone(),
        ))
        .attach(rocket::fairing::AdHoc::on_shutdown(
            "Stop loading users from Near and Github metadata",
            |_| {
                Box::pin(async move {
                    atomic_bool.store(false, std::sync::atomic::Ordering::Relaxed);
                })
            },
        ))
        .mount(
            "/",
            SwaggerUi::new("/swagger-ui/<_..>").url("/api-docs/openapi.json", ApiDoc::openapi()),
        )
        .attach(prometheus.clone())
        .attach(entrypoints::stage())
        .mount("/metrics", prometheus)
}
