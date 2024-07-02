use std::sync::Arc;

use shared::telegram::TelegramSubscriber;

pub mod contract_pull;
pub mod db;
pub mod github_pull;
pub mod svg;

pub fn error(telegram: &Arc<TelegramSubscriber>, message: &str) {
    telegram.send_to_telegram(message, &tracing::Level::ERROR);
    rocket::error!("{}", message);
}
