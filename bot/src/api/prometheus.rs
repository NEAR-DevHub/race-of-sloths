use prometheus_client::encoding::text::encode;
use prometheus_client::encoding::{EncodeLabelSet, EncodeLabelValue};
use prometheus_client::metrics::counter::Counter;
use prometheus_client::metrics::family::Family;
use prometheus_client::registry::Registry;
use shared::github::PrMetadata;

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelValue)]

pub enum EventType {
    Include,
    Score,
    Pause,
    Unpause,
    Excluded,
    Unknown,
    Merge,
    Finalize,
    Stale,
}

impl From<&crate::events::EventType> for EventType {
    fn from(e: &crate::events::EventType) -> Self {
        match e {
            crate::events::EventType::Command { command, .. } => match command {
                crate::events::commands::Command::Include(_) => EventType::Include,
                crate::events::commands::Command::Score(_) => EventType::Score,
                crate::events::commands::Command::Pause(_) => EventType::Pause,
                crate::events::commands::Command::Unpause(_) => EventType::Unpause,
                crate::events::commands::Command::Excluded(_) => EventType::Excluded,
                crate::events::commands::Command::Unknown(_) => EventType::Unknown,
            },
            crate::events::EventType::Action(action) => match action {
                crate::events::actions::Action::Merge(_) => EventType::Merge,
                crate::events::actions::Action::Finalize(_) => EventType::Finalize,
                crate::events::actions::Action::Stale(_) => EventType::Stale,
            },
        }
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
pub struct MetricRecord {
    pub event_type: EventType,
    pub author: String,
    pub organization: String,
    pub repository: String,
    pub pr_number: u64,
    pub success: u32,
    pub timestamp: u64,
}

#[derive(Debug)]
pub struct PrometheusClient {
    registry: Registry,
    event: Family<MetricRecord, Counter>,
}

impl Default for PrometheusClient {
    fn default() -> Self {
        let mut registry = Registry::default();
        let event = Family::default();

        registry.register("bot_event", "Processing event that happened", event.clone());

        Self { registry, event }
    }
}

impl PrometheusClient {
    pub fn record(&self, event: &crate::events::EventType, pr: &PrMetadata, success: bool) {
        let event_type = event.into();
        let record = MetricRecord {
            event_type,
            author: pr.author.login.clone(),
            organization: pr.owner.clone(),
            repository: pr.repo.clone(),
            pr_number: pr.number,
            success: success as u32,
            timestamp: chrono::Utc::now().timestamp() as u64,
        };
        self.event.get_or_create(&record).inc();
    }

    pub fn encode(&self) -> anyhow::Result<String> {
        let mut body = String::new();
        encode(&mut body, &self.registry)?;
        self.event.clear();

        Ok(body)
    }
}
