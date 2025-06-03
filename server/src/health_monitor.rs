use rocket::tokio::sync::mpsc;
use shared::telegram::TelegramSubscriber;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

pub struct HealthMonitor {
    sender: mpsc::UnboundedSender<String>,
}

impl HealthMonitor {
    pub fn new(telegram: Arc<TelegramSubscriber>) -> Self {
        let (sender, mut receiver) = mpsc::unbounded_channel::<String>();

        rocket::tokio::spawn(async move {
            let mut map: HashMap<String, Instant> = HashMap::new();
            let mut interval = rocket::tokio::time::interval(Duration::from_secs(15));
            loop {
                rocket::tokio::select! {
                    _ = interval.tick() => {
                        for (task_name, last_heartbeat) in &map {
                            if last_heartbeat.elapsed() > Duration::from_secs(600) { // 10 minutes
                                crate::error(&telegram, &format!("🚨 No health reports for {} for 10 minutes - shutting down application", task_name));
                                // Probably, it should be a more elegant way to do this, but we will crash the app for now to restart it
                                std::process::exit(1);
                            }
                        }
                    }
                    Some(task_name) = receiver.recv() => {
                        map.insert(task_name, Instant::now());
                    }
                }
            }
        });

        Self { sender }
    }

    pub fn im_alive(&self, task_name: &str) {
        let _ = self.sender.send(task_name.to_string());
    }
}
