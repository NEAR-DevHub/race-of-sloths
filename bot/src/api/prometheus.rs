use prometheus_client::encoding::text::encode;
use prometheus_client::encoding::{EncodeLabelSet, EncodeLabelValue};
use prometheus_client::metrics::counter::Counter;
use prometheus_client::metrics::family::Family;
use prometheus_client::metrics::gauge::Gauge;
use prometheus_client::metrics::histogram::Histogram;
use prometheus_client::registry::Registry;
use shared::github::PrMetadata;

use crate::events::EventResult;

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
    Update,
}

impl From<&crate::events::EventType> for EventType {
    fn from(e: &crate::events::EventType) -> Self {
        match e {
            crate::events::EventType::PRCommand { command, .. } => match command {
                crate::events::pr_commands::Command::Include(_) => EventType::Include,
                crate::events::pr_commands::Command::Score(_) => EventType::Score,
                crate::events::pr_commands::Command::Pause(_) => EventType::Pause,
                crate::events::pr_commands::Command::Unpause(_) => EventType::Unpause,
                crate::events::pr_commands::Command::Excluded(_) => EventType::Excluded,
                crate::events::pr_commands::Command::Unknown(_) => EventType::Unknown,
                crate::events::pr_commands::Command::Update(_) => EventType::Update,
            },
            crate::events::EventType::Action { action, .. } => match action {
                crate::events::actions::Action::Merge(_) => EventType::Merge,
                crate::events::actions::Action::Finalize(_) => EventType::Finalize,
                crate::events::actions::Action::Stale(_) => EventType::Stale,
            },
            crate::events::EventType::IssueCommand { command, .. } => match command {
                crate::events::issue_commands::Command::Unpause(_) => EventType::Unpause,
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
    pub result_text: String,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
pub struct TimeMetric {
    pub event_type: EventType,
    pub success: u32,
}

pub struct PrometheusClient {
    registry: Registry,
    event: Family<MetricRecord, Counter>,
    event_processing_time: Family<TimeMetric, Histogram>,

    // We can get this from the github api so we can track it as gauge
    github_api_read_request: Gauge,
    // Unfortunately, github api doesn't show secondary metrics in the rate-limit response
    // so we will increment this counter for each write request
    // TODO: actually, github shows it in the response headers, but the octocrab doesn't expose it
    // we might need to expose it to make it better
    github_api_write_request: Counter,
}

impl Default for PrometheusClient {
    fn default() -> Self {
        let mut registry = Registry::default();
        let event = Family::default();
        let github_api_read_request = Gauge::default();
        let github_api_write_request = Counter::default();
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

        registry.register(
            "github_api_read_requests",
            "Display used github read requests at a metric time",
            github_api_read_request.clone(),
        );
        registry.register(
            "github_api_write_requests",
            "Display total used github write requests from the start at a metric time",
            github_api_write_request.clone(),
        );

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
            github_api_read_request,
            github_api_write_request,
        }
    }
}

impl PrometheusClient {
    pub fn record_pr(
        &self,
        event: &crate::events::EventType,
        pr: &PrMetadata,
        result: &anyhow::Result<EventResult>,
        time: chrono::DateTime<chrono::Utc>,
    ) {
        let event_type = event.into();
        let result_text = match result {
            Ok(res) => res.to_string(),
            Err(_) => "Error".to_string(),
        };
        let record = MetricRecord {
            event_type,
            author: pr.author.login.clone(),
            organization: pr.repo_info.owner.clone(),
            repository: pr.repo_info.repo.clone(),
            pr_number: pr.repo_info.number,
            success: result.is_ok() as u32,
            result_text,
        };
        self.event.get_or_create(&record).inc();

        let time = chrono::Utc::now() - time;
        self.event_processing_time
            .get_or_create(&TimeMetric {
                event_type,
                success: result.is_ok() as u32,
            })
            .observe(time.num_milliseconds() as f64 / 1000.0);
    }

    pub fn add_write_request(&self) {
        self.github_api_write_request.inc();
    }

    pub fn set_read_requests(&self, value: i64) {
        self.github_api_read_request.set(value);
    }

    pub fn encode(&self) -> anyhow::Result<String> {
        let mut body = String::new();
        encode(&mut body, &self.registry)?;
        Ok(body)
    }
}
