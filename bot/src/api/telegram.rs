use reqwest::Client;
use std::fmt;
use tokio::sync::mpsc;
use tracing::{Event, Level, Subscriber};

pub struct TelegramSubscriber {
    sender: mpsc::UnboundedSender<(String, Level)>,
}

async fn sender_task(
    mut reader: mpsc::UnboundedReceiver<(String, Level)>,
    client: Client,
    bot_token: String,
    chat_id: String,
) {
    while let Some((message, level)) = reader.recv().await {
        let url = format!("https://api.telegram.org/bot{}/sendMessage", bot_token);

        let message = message
            .replace('_', "\\_")
            .replace('*', "\\*")
            .replace('[', "\\[")
            .replace(']', "\\]")
            .replace('(', "\\(")
            .replace(')', "\\)")
            .replace('~', "\\~")
            .replace('`', "\\`")
            .replace('>', "\\>")
            .replace('#', "\\#")
            .replace('+', "\\+")
            .replace('-', "\\-")
            .replace('=', "\\=")
            .replace('|', "\\|")
            .replace('{', "\\{")
            .replace('}', "\\}")
            .replace('.', "\\.")
            .replace('!', "\\!");

        let message = format!("*{}*: `{message}`", level.as_str());
        let params = [
            ("chat_id", chat_id.as_str()),
            ("text", &message),
            ("parse_mode", "MarkdownV2"),
        ];
        match client.post(&url).form(&params).send().await {
            Ok(response) if response.status().is_success() => {}
            // We use eprintln! here because it doesn't make sense to send back a message to the chat
            Ok(response) => eprintln!(
                "Failed to send message: Received HTTP {}:",
                response.status()
            ),
            Err(e) => eprintln!("Failed to send message: {}", e),
        }
    }
}

impl TelegramSubscriber {
    pub async fn new(bot_token: String, chat_id: String) -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        tokio::spawn(sender_task(
            receiver,
            Client::new(),
            bot_token.clone(),
            chat_id.clone(),
        ));
        Self { sender }
    }

    fn send_to_telegram(&self, message: &str, level: &Level) {
        let _ = self.sender.send((message.to_string(), *level));
    }
}

impl<S: Subscriber> tracing_subscriber::Layer<S> for TelegramSubscriber {
    fn on_event(&self, event: &Event<'_>, _ctx: tracing_subscriber::layer::Context<'_, S>) {
        let mut visitor = MessageVisitor::default();
        event.record(&mut visitor);

        let message = format!("{}", visitor);

        // Currently, we don't have a solution to store logs
        // but we want to have a way to notify and react on warnings and errors
        // so we send them to the telegram chat
        let level = event.metadata().level();
        if level <= &Level::WARN {
            self.send_to_telegram(&message, level);
        }
    }
}

#[derive(Default)]
struct MessageVisitor {
    message: String,
}

impl fmt::Display for MessageVisitor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl tracing::field::Visit for MessageVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn fmt::Debug) {
        if field.name() == "message" {
            self.message = format!("{:?}", value);
        }
    }
}
