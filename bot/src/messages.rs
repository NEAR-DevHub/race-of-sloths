use rand::seq::SliceRandom;
use rand::thread_rng;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use tracing::error;

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum MsgCategory {
    IncludeBasicMessage,
    // TODO: currently unused
    IncludeStreakMessage,
    // TODO: currently unused
    IncludeFirstTimeMessage,
    CorrectNonzeroScoringMessage,
    CorrectZeroScoringMessage,
    CorrectableScoringMessage,
    ExcludeMessages,
    PauseMessage,
    UnpauseMessage,
    MergeWithScoreMessage,
    MergeWithoutScoreMessage,
    FinalMessage,
    StaleMessage,
    ErrorUnknownCommandMessage,
    ErrorRightsViolationMessage,
    ErrorLateIncludeMessage,
    // TODO: currently unused
    ErrorLateScoringMessage,
    ErrorSelfScore,
    ErrorOrgNotInAllowedListMessage,
    UnpauseUnpausedMessage,
}

impl std::fmt::Display for MsgCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Message {
    message: String,
    variables: HashSet<String>,
}

impl Message {
    pub fn new(message: String, variables: HashSet<String>) -> Self {
        Self { message, variables }
    }

    pub fn format(&self, values: HashMap<String, String>) -> String {
        let mut formatted_message = self.message.clone();
        for key in self.variables.iter() {
            if let Some(value) = values.get(key.as_str()) {
                formatted_message = formatted_message.replace(&format!("{{{}}}", key), value);
            } else {
                error!(
                    "The message expects a variable: {}, but it wasn't provided",
                    key
                );
            }
        }
        formatted_message
    }

    fn partial_format(&mut self, values: &HashMap<String, String>) {
        for (key, value) in values {
            self.message = self.message.replace(&format!("{{{key}}}"), value);
            self.variables.remove(key);
        }
    }

    fn with(&self, message: &Message) -> Message {
        let mut new_message = self.clone();
        new_message.message.push_str(&message.message);
        new_message.variables.extend(message.variables.clone());
        new_message
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct MessageLoader {
    pub link: String,
    pub leaderboard_link: String,
    pub form: String,

    pub common_basic_include_message: Vec<Message>,
    pub include_basic_message: Vec<Message>,
    pub include_streak_message: Vec<Message>,
    pub include_first_time_message: Vec<Message>,
    pub correct_nonzero_scoring_message: Vec<Message>,
    pub correct_zero_scoring_message: Vec<Message>,
    pub correctable_scoring_message: Vec<Message>,
    pub exclude_messages: Vec<Message>,
    pub pause_message: Vec<Message>,
    pub unpause_message: Vec<Message>,
    pub merge_with_score_message: Vec<Message>,
    pub merge_without_score_message: Vec<Message>,
    pub final_message: Vec<Message>,
    pub stale_message: Vec<Message>,
    pub error_unknown_command_message: Vec<Message>,
    pub error_rights_violation_message: Vec<Message>,
    pub error_late_include_message: Vec<Message>,
    pub error_late_scoring_message: Vec<Message>,
    pub error_selfscore_message: Vec<Message>,
    pub error_org_not_in_allowed_list_message: Vec<Message>,
    pub unpause_unpaused_message: Vec<Message>,
}

impl MessageLoader {
    pub fn new() -> Self {
        Default::default()
    }

    fn postprocess_messages_with_link(&mut self, bot_name: &str) {
        let values = vec![
            ("link".to_string(), self.link.clone()),
            (
                "leaderboard-link".to_string(),
                self.leaderboard_link.clone(),
            ),
            ("bot-name".to_string(), bot_name.to_string()),
            ("form".to_string(), self.form.clone()),
        ]
        .into_iter()
        .collect::<HashMap<_, _>>();

        let array_of_messages = vec![
            &mut self.common_basic_include_message,
            &mut self.include_basic_message,
            &mut self.include_streak_message,
            &mut self.include_first_time_message,
            &mut self.correct_nonzero_scoring_message,
            &mut self.correct_zero_scoring_message,
            &mut self.correctable_scoring_message,
            &mut self.exclude_messages,
            &mut self.pause_message,
            &mut self.unpause_message,
            &mut self.merge_with_score_message,
            &mut self.merge_without_score_message,
            &mut self.final_message,
            &mut self.stale_message,
            &mut self.error_unknown_command_message,
            &mut self.error_rights_violation_message,
            &mut self.error_late_include_message,
            &mut self.error_late_scoring_message,
            &mut self.error_selfscore_message,
            &mut self.error_org_not_in_allowed_list_message,
            &mut self.unpause_unpaused_message,
        ];
        for messages in array_of_messages {
            for message in messages {
                message.partial_format(&values);
            }
        }
    }

    pub fn load_from_file(file_path: &PathBuf, bot_name: &str) -> anyhow::Result<Self> {
        let file_content = fs::read_to_string(file_path)?;
        let mut result: Self = toml::from_str(&file_content)?;
        result.postprocess_messages_with_link(bot_name);
        tracing::trace!("Loaded messages: {:#?}", result);
        Ok(result)
    }

    pub fn get_message(&self, category: MsgCategory) -> Option<Message> {
        let mut rng = thread_rng();
        let elem = match category {
            MsgCategory::IncludeBasicMessage => &self.include_basic_message,
            MsgCategory::IncludeStreakMessage => &self.include_streak_message,
            MsgCategory::IncludeFirstTimeMessage => &self.include_first_time_message,
            MsgCategory::CorrectNonzeroScoringMessage => &self.correct_nonzero_scoring_message,
            MsgCategory::CorrectZeroScoringMessage => &self.correct_zero_scoring_message,
            MsgCategory::CorrectableScoringMessage => &self.correctable_scoring_message,
            MsgCategory::ExcludeMessages => &self.exclude_messages,
            MsgCategory::PauseMessage => &self.pause_message,
            MsgCategory::UnpauseMessage => &self.unpause_message,
            MsgCategory::MergeWithScoreMessage => &self.merge_with_score_message,
            MsgCategory::MergeWithoutScoreMessage => &self.merge_without_score_message,
            MsgCategory::FinalMessage => &self.final_message,
            MsgCategory::StaleMessage => &self.stale_message,
            MsgCategory::ErrorUnknownCommandMessage => &self.error_unknown_command_message,
            MsgCategory::ErrorRightsViolationMessage => &self.error_rights_violation_message,
            MsgCategory::ErrorLateIncludeMessage => &self.error_late_include_message,
            MsgCategory::ErrorLateScoringMessage => &self.error_late_scoring_message,
            MsgCategory::UnpauseUnpausedMessage => &self.unpause_unpaused_message,
            MsgCategory::ErrorSelfScore => &self.error_selfscore_message,
            MsgCategory::ErrorOrgNotInAllowedListMessage => {
                &self.error_org_not_in_allowed_list_message
            }
        };
        let elem = elem.choose(&mut rng)?;

        let elem = if matches!(
            category,
            MsgCategory::IncludeBasicMessage
                | MsgCategory::IncludeStreakMessage
                | MsgCategory::IncludeFirstTimeMessage
        ) {
            elem.with(self.common_basic_include_message.choose(&mut rng)?)
        } else {
            elem.clone()
        };

        Some(elem)
    }
}
