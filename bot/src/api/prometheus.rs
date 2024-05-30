use prometheus_client::encoding::text::encode;
use prometheus_client::encoding::{EncodeLabelSet, EncodeLabelValue};
use prometheus_client::metrics::counter::Counter;
use prometheus_client::metrics::family::Family;
use prometheus_client::metrics::histogram::Histogram;
use prometheus_client::registry::Registry;
use shared::github::PrMetadata;

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, EncodeLabelValue)]

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
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
pub struct TimeMetric {
    pub event_type: EventType,
    pub success: u32,
}

#[derive(Debug)]
pub struct PrometheusClient {
    registry: Registry,
    event: Family<MetricRecord, Counter>,
    event_processing_time: Family<TimeMetric, Histogram>,
}

impl Default for PrometheusClient {
    fn default() -> Self {
        let mut registry = Registry::default();
        let event = Family::default();
        let event_processing_time: Family<TimeMetric, Histogram> =
            Family::new_with_constructor(|| {
                Histogram::new(
                    [
                        10.,
                        30.,
                        60.,
                        120.,
                        300.,
                        600.,
                        1800.,
                        3600.,
                        7200.,
                        86400.,
                        172800.,
                        f64::INFINITY,
                    ]
                    .into_iter(),
                )
            });

        registry.register("bot_event", "Processing event that happened", event.clone());
        registry.register(
            "bot_event_processing_time",
            "Processing time for events",
            event_processing_time.clone(),
        );
        Self {
            registry,
            event,
            event_processing_time,
        }
    }
}

impl PrometheusClient {
    pub fn record(
        &self,
        event: &crate::events::EventType,
        pr: &PrMetadata,
        success: bool,
        time: chrono::DateTime<chrono::Utc>,
    ) {
        let event_type = event.into();
        let record = MetricRecord {
            event_type,
            author: pr.author.login.clone(),
            organization: pr.owner.clone(),
            repository: pr.repo.clone(),
            pr_number: pr.number,
            success: success as u32,
        };
        self.event.get_or_create(&record).inc();

        let time = chrono::Utc::now() - time;
        self.event_processing_time
            .get_or_create(&TimeMetric {
                event_type,
                success: success as u32,
            })
            .observe(time.num_milliseconds() as f64 / 1000.0);
    }

    pub fn encode(&self) -> anyhow::Result<String> {
        let mut body = String::new();
        encode(&mut body, &self.registry)?;
        Ok(body)
    }
}
