#[macro_use]
extern crate rocket;

mod entrypoints;

use std::sync::Arc;
use std::time::Duration;

use entrypoints::ApiDoc;
use rocket::{fairing::AdHoc, fs::NamedFile, State};
use rocket_cors::AllowedOrigins;
use shared::{near::NearClient, telegram};

use race_of_sloths_server::{contract_pull, db, github_pull, weekly_stats};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

#[derive(Debug, serde::Deserialize)]
pub struct Env {
    contract: String,
    rpc_addr: Option<String>,
    secret_key: String,
    is_mainnet: bool,
    near_timeout_in_seconds: Option<u64>,
    github_timeout_in_minutes: Option<u32>,
    github_token: String,
    telegram_token: String,
    telegram_chat_id: String,
    font: String,
}

// Allow robots to crawl the site
#[get("/robots.txt")]
fn robots() -> &'static str {
    "User-agent: *\nDisallow: /"
}

#[get("/favicon.ico")]
async fn favicon(telegram: &State<Arc<telegram::TelegramSubscriber>>) -> Option<NamedFile> {
    match NamedFile::open("./public/favicon.ico").await {
        Ok(file) => Some(file),
        Err(e) => {
            race_of_sloths_server::error(telegram, &format!("Failed to load favicon.ico: {}", e));
            None
        }
    }
}

const WEEK_IN_SECONDS: u64 = 7 * 24 * 60 * 60;

#[launch]
async fn rocket() -> _ {
    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .expect("Failed to install AWS LC provider");
    dotenv::dotenv().ok();

    let env = envy::from_env::<Env>().expect("Failed to load environment variables");
    let near_sleep = Duration::from_secs(env.near_timeout_in_seconds.unwrap_or(20));
    let github_sleep = Duration::from_secs(env.github_timeout_in_minutes.unwrap_or(60) as u64 * 60);
    let weekly_sleep = Duration::from_secs(WEEK_IN_SECONDS);
    let atomic_bool = Arc::new(std::sync::atomic::AtomicBool::new(true));
    let prometheus = rocket_prometheus::PrometheusMetrics::new();

    let telegram: telegram::TelegramSubscriber =
        telegram::TelegramSubscriber::new(env.telegram_token, env.telegram_chat_id).await;

    let near_client = NearClient::new(
        env.contract.clone(),
        env.secret_key.clone(),
        env.is_mainnet,
        env.rpc_addr,
    )
    .await
    .expect("Failed to create Near client");

    let allowed_origins = AllowedOrigins::some_exact(&[
        "http://localhost:3000",
        "https://race-of-sloths.ai",
        "https://race-of-sloths.org",
        "https://race-of-sloths.io",
        "https://race-of-sloths.com",
        "https://race-of-sloths-website.vercel.app",
        "https://race-of-sloths-website-three.vercel.app",
    ]);
    let cors = rocket_cors::CorsOptions {
        allowed_origins,
        ..Default::default()
    }
    .to_cors()
    .expect("Failed to create cors config");

    // TODO: after 0.6.0 release, we should use tracing for redirecting warns and errors to the telegram

    rocket::build()
        .attach(cors)
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
        .attach(weekly_stats::stage(weekly_sleep, atomic_bool.clone()))
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
        .mount("/", routes![robots, favicon])
        .attach(prometheus.clone())
        .attach(entrypoints::stage(env.font))
        .mount("/metrics", prometheus)
        .manage(Arc::new(telegram))
        .attach(AdHoc::on_response(
            "Telegram notification about failed resposnes",
            |req, resp: &mut rocket::Response<'_>| {
                Box::pin(async move {
                    let telegram = req
                        .guard::<&State<Arc<telegram::TelegramSubscriber>>>()
                        .await;
                    if telegram.is_error() {
                        return;
                    }
                    let telegram = telegram.unwrap();
                    match resp.status().class() {
                        rocket::http::StatusClass::ServerError
                        | rocket::http::StatusClass::Unknown => {
                            telegram.send_to_telegram(
                                &format!(
                                    "Request {}{} failed with code {}\n",
                                    req.method(),
                                    req.uri(),
                                    resp.status(),
                                ),
                                &tracing::Level::ERROR,
                            );
                        }
                        _ => {}
                    }
                })
            },
        ))
}
