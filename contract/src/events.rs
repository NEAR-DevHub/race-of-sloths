use near_sdk::env;
use shared::Event;

pub fn log_event(event: Event) {
    env::log_str(&near_sdk::serde_json::to_string(&event).unwrap());
}
