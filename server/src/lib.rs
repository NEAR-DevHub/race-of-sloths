use std::sync::Arc;

use shared::telegram::TelegramSubscriber;

pub mod contract_pull;
pub mod db;
pub mod github_pull;
pub mod health_monitor;
pub mod svg;
pub mod types;
pub mod weekly_stats;

// TODO: after 0.6.0 release, we should use tracing for redirecting warns and errors to the telegram
pub fn error(telegram: &Arc<TelegramSubscriber>, message: &str) {
    telegram.send_to_telegram(message, &tracing::Level::ERROR);
    rocket::error!("{}", message);
}
