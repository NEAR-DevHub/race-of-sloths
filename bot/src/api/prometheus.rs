use prometheus_client::encoding::text::encode;
use prometheus_client::encoding::{EncodeLabelSet, EncodeLabelValue};
use prometheus_client::metrics::counter::Counter;
use prometheus_client::metrics::family::Family;
use prometheus_client::registry::Registry;

use crate::events::EventType;

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelValue)]

pub enum Command {
    Include,
    Score,
    Pause,
    Unpause,
    Excluded,
    Unknown,
}

impl From<&crate::events::commands::Command> for Command {
    fn from(command: &crate::events::commands::Command) -> Self {
        match command {
            crate::events::commands::Command::Include(_) => Command::Include,
            crate::events::commands::Command::Score(_) => Command::Score,
            crate::events::commands::Command::Pause(_) => Command::Pause,
            crate::events::commands::Command::Unpause(_) => Command::Unpause,
            crate::events::commands::Command::Excluded(_) => Command::Excluded,
            crate::events::commands::Command::Unknown(_) => Command::Unknown,
        }
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
pub struct CommandLabels {
    pub method: Command,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelValue)]
pub enum Action {
    Merge,
    Finalize,
    Stale,
}

impl From<&crate::events::actions::Action> for Action {
    fn from(action: &crate::events::actions::Action) -> Self {
        match action {
            crate::events::actions::Action::Merge(_) => Action::Merge,
            crate::events::actions::Action::Finalize(_) => Action::Finalize,
            crate::events::actions::Action::Stale(_) => Action::Stale,
        }
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
pub struct ActionLabels {
    pub action: Action,
}

#[derive(Debug)]
pub struct PrometheusClient {
    registry: Registry,
    successful_commands: Family<CommandLabels, Counter>,
    total_commands: Family<CommandLabels, Counter>,
    succesful_actions: Family<ActionLabels, Counter>,
    total_actions: Family<ActionLabels, Counter>,
    total_interactions: Counter,
}

impl Default for PrometheusClient {
    fn default() -> Self {
        let mut registry = Registry::default();
        let successful_commands = Family::default();
        let total_commands = Family::default();
        let succesful_actions = Family::default();
        let total_actions = Family::default();
        let total_interactions = Counter::default();

        registry.register(
            "successful-commands",
            "Executed succesfully commands initiated by user",
            successful_commands.clone(),
        );

        registry.register(
            "total-commands",
            "Total commands parsed by bot. One command might be executed multiple times, if NEAR RPC fails",
            total_commands.clone(),
        );

        registry.register(
            "successful-actions",
            "Executed succesfully actions initiated by bot",
            succesful_actions.clone(),
        );

        registry.register(
            "total-actions",
            "Total actions executed by bot",
            total_actions.clone(),
        );

        registry.register(
            "total-interactions",
            "Total interactions with bot",
            total_interactions.clone(),
        );

        Self {
            registry,
            successful_commands,
            total_commands,
            succesful_actions,
            total_actions,
            total_interactions,
        }
    }
}

impl PrometheusClient {
    pub fn record(&self, event: &EventType, success: bool) {
        match event {
            EventType::Command { command, .. } => {
                self.successful_commands
                    .get_or_create(&CommandLabels {
                        method: command.into(),
                    })
                    .inc_by(success as u64);
                self.total_commands
                    .get_or_create(&CommandLabels {
                        method: command.into(),
                    })
                    .inc();
            }
            EventType::Action(action) => {
                self.succesful_actions
                    .get_or_create(&ActionLabels {
                        action: action.into(),
                    })
                    .inc_by(success as u64);
                self.total_actions
                    .get_or_create(&ActionLabels {
                        action: action.into(),
                    })
                    .inc();
            }
        }
        self.total_interactions.inc();
    }

    pub fn encode(&self) -> anyhow::Result<String> {
        let mut body = String::new();
        encode(&mut body, &self.registry)?;
        Ok(body)
    }
}
