use reqwest::{multipart, Client, Response};
use std::fmt;
use tokio::sync::mpsc;
use tracing::{Event, Level, Subscriber};

pub enum MessageType {
    CsvFile((String, Vec<u8>)),
    Message((String, Level)),
}

#[derive(Clone)]
pub struct TelegramSubscriber {
    sender: mpsc::UnboundedSender<MessageType>,
}

async fn send_message(
    client: &Client,
    bot_token: &str,
    chat_id: &str,
    message: String,
    level: Level,
) -> anyhow::Result<Response> {
    let url = format!("https://api.telegram.org/bot{}/sendMessage", bot_token);

    let message = if level == Level::INFO {
        message.replace('-', "\\-")
    } else {
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

        format!("*{}*: `{}`", level.as_str(), message)
    };
    let params = [
        ("chat_id", chat_id),
        ("text", &message),
        ("parse_mode", "MarkdownV2"),
    ];

    Ok(client.post(&url).form(&params).send().await?)
}

async fn send_csv_to_telegram(
    client: &Client,
    bot_token: &str,
    chat_id: &str,
    csv_content: Vec<u8>,
    filename: String,
) -> anyhow::Result<Response> {
    let url = format!("https://api.telegram.org/bot{}/sendDocument", bot_token);

    let form = multipart::Form::new()
        .text("chat_id", chat_id.to_string())
        .part(
            "document",
            multipart::Part::bytes(csv_content)
                .file_name(filename)
                .mime_str("text/csv")?,
        );

    let response = client.post(&url).multipart(form).send().await?;

    Ok(response)
}

async fn sender_task(
    mut reader: mpsc::UnboundedReceiver<MessageType>,
    client: Client,
    bot_token: String,
    chat_id: String,
) {
    while let Some(msg) = reader.recv().await {
        let result = match msg {
            MessageType::Message((message, level)) => {
                send_message(&client, &bot_token, &chat_id, message, level).await
            }
            MessageType::CsvFile((file, csv)) => {
                send_csv_to_telegram(&client, &bot_token, &chat_id, csv, file).await
            }
        };

        match result {
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

    pub fn send_to_telegram(&self, message: &str, level: &Level) {
        let _ = self
            .sender
            .send(MessageType::Message((message.to_string(), *level)));
    }

    pub fn send_csv_file_to_telegram(&self, bytes: Vec<u8>, filename: String) {
        let _ = self.sender.send(MessageType::CsvFile((filename, bytes)));
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
