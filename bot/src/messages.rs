use rand::seq::SliceRandom;
use rand::thread_rng;
use serde::{Deserialize, Serialize};
use shared::github::PrMetadata;
use shared::{PRInfo, TimePeriod, User};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::ops::Add;
use std::path::PathBuf;
use tracing::error;

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum MsgCategory {
    IncludeBasicMessage,
    IncludeCommonMessage,
    CorrectNonzeroScoringMessage,
    CorrectZeroScoringMessage,
    CorrectableScoringMessage,
    ExcludeMessages,
    PauseMessage,
    UnpauseMessage,
    MergeWithoutScoreMessage,
    FinalMessageCommon,
    FinalMessagesWeeklyStreak,
    FinalMessagesMonthlyStreak,
    FinalMessagesFirstLifetimeBonus,
    FinalMessagesLifetimeBonus,
    FinalMessagesFeedbackForm,
    StaleMessage,
    ErrorUnknownCommandMessage,
    ErrorRightsViolationMessage,
    ErrorLateIncludeMessage,
    ErrorPausePausedMessage,
    ErrorUnpauseUnpausedMessage,
    ErrorPausedMessage,
    ErrorLateScoringMessage,
    ErrorSelfScore,
    ErrorOrgNotInAllowedListMessage,

    FirstTimeContribution,
    FirstWeekContribution,
    FirstMonthContribution,
    Contribution3,
    Contribution4,
    Contribution5,
    Contribution6,
    Contribution7,
    Contribution8,
}

impl std::fmt::Display for MsgCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Messages {
    message: Vec<String>,
    variables: HashSet<String>,
}

impl Messages {
    pub fn new(message: Vec<String>, variables: HashSet<String>) -> Self {
        Self { message, variables }
    }

    pub fn format(&self, values: HashMap<String, String>) -> anyhow::Result<String> {
        let mut formatted_message = self
            .message
            .choose(&mut thread_rng())
            .ok_or_else(|| anyhow::anyhow!("Failed to choose randomly an message"))?
            .clone();
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
        Ok(formatted_message)
    }

    fn partial_format(&mut self, values: &HashMap<String, String>) {
        for message in self.message.iter_mut() {
            for (key, value) in values {
                *message = message.replace(&format!("{{{key}}}"), value);
                self.variables.remove(key);
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MessageLoader {
    pub link: String,
    pub leaderboard_link: String,
    pub form: String,
    pub picture_api_link: String,

    // Messages
    pub include_basic_messages: Messages,
    pub include_common_messages: Messages,
    pub correct_nonzero_scoring_messages: Messages,
    pub correct_zero_scoring_messages: Messages,
    pub correctable_scoring_messages: Messages,
    pub exclude_messages: Messages,
    pub pause_messages: Messages,
    pub unpause_messages: Messages,
    pub merge_without_score_messages: Messages,
    pub final_messages_common: Messages,
    pub final_messages_weekly_streak: Messages,
    pub final_messages_monthly_streak: Messages,
    pub final_messages_first_lifetime_bonus: Messages,
    pub final_messages_lifetime_bonus: Messages,
    pub final_messages_feedback_form: Messages,
    pub stale_messages: Messages,

    // Errors
    pub error_unknown_command_messages: Messages,
    pub error_rights_violation_messages: Messages,
    pub error_late_include_messages: Messages,
    pub error_late_scoring_messages: Messages,
    pub error_pause_paused_messages: Messages,
    pub error_unpause_unpaused_messages: Messages,
    pub error_paused_messages: Messages,
    pub error_selfscore_messages: Messages,
    pub error_org_not_in_allowed_list_messages: Messages,

    // Message by amount of contributions
    pub first_time_contribution: Messages,
    pub first_week_contribution: Messages,
    pub first_month_contribution: Messages,
    pub contribution_3: Messages,
    pub contribution_4: Messages,
    pub contribution_5: Messages,
    pub contribution_6: Messages,
    pub contribution_7: Messages,
    pub contribution_8: Messages,
}

impl MessageLoader {
    pub fn load_from_file(file_path: &PathBuf, bot_name: &str) -> anyhow::Result<Self> {
        let file_content = fs::read_to_string(file_path)?;
        let mut result: Self = toml::from_str(&file_content)?;
        result.postprocess_messages_with_link(bot_name);
        tracing::trace!("Loaded messages: {:#?}", result);
        Ok(result)
    }

    fn postprocess_messages_with_link(&mut self, bot_name: &str) {
        let values = vec![
            ("link".to_string(), self.link.clone()),
            (
                "leaderboard_link".to_string(),
                self.leaderboard_link.clone(),
            ),
            ("bot_name".to_string(), bot_name.to_string()),
            ("form".to_string(), self.form.clone()),
            (
                "picture_api_link".to_string(),
                self.picture_api_link.clone(),
            ),
        ]
        .into_iter()
        .collect::<HashMap<_, _>>();

        let array_of_messages = vec![
            &mut self.include_basic_messages,
            &mut self.include_common_messages,
            &mut self.correct_nonzero_scoring_messages,
            &mut self.correct_zero_scoring_messages,
            &mut self.correctable_scoring_messages,
            &mut self.exclude_messages,
            &mut self.pause_messages,
            &mut self.unpause_messages,
            &mut self.merge_without_score_messages,
            &mut self.final_messages_common,
            &mut self.final_messages_first_lifetime_bonus,
            &mut self.final_messages_lifetime_bonus,
            &mut self.final_messages_monthly_streak,
            &mut self.final_messages_weekly_streak,
            &mut self.final_messages_feedback_form,
            &mut self.stale_messages,
            &mut self.error_unknown_command_messages,
            &mut self.error_rights_violation_messages,
            &mut self.error_late_include_messages,
            &mut self.error_late_scoring_messages,
            &mut self.error_pause_paused_messages,
            &mut self.error_unpause_unpaused_messages,
            &mut self.error_paused_messages,
            &mut self.error_selfscore_messages,
            &mut self.error_org_not_in_allowed_list_messages,
            &mut self.first_time_contribution,
            &mut self.first_week_contribution,
            &mut self.first_month_contribution,
            &mut self.contribution_3,
            &mut self.contribution_4,
            &mut self.contribution_5,
            &mut self.contribution_6,
            &mut self.contribution_7,
            &mut self.contribution_8,
        ];
        for message in array_of_messages {
            message.partial_format(&values);
        }
    }

    pub fn get_message(&self, category: MsgCategory) -> Messages {
        let elem = match category {
            MsgCategory::IncludeBasicMessage => &self.include_basic_messages,
            MsgCategory::IncludeCommonMessage => &self.include_common_messages,
            MsgCategory::CorrectNonzeroScoringMessage => &self.correct_nonzero_scoring_messages,
            MsgCategory::CorrectZeroScoringMessage => &self.correct_zero_scoring_messages,
            MsgCategory::CorrectableScoringMessage => &self.correctable_scoring_messages,
            MsgCategory::ExcludeMessages => &self.exclude_messages,
            MsgCategory::PauseMessage => &self.pause_messages,
            MsgCategory::UnpauseMessage => &self.unpause_messages,
            MsgCategory::MergeWithoutScoreMessage => &self.merge_without_score_messages,
            MsgCategory::FinalMessageCommon => &self.final_messages_common,
            MsgCategory::FinalMessagesWeeklyStreak => &self.final_messages_weekly_streak,
            MsgCategory::FinalMessagesMonthlyStreak => &self.final_messages_monthly_streak,
            MsgCategory::FinalMessagesFirstLifetimeBonus => {
                &self.final_messages_first_lifetime_bonus
            }
            MsgCategory::FinalMessagesFeedbackForm => &self.final_messages_feedback_form,
            MsgCategory::FinalMessagesLifetimeBonus => &self.final_messages_lifetime_bonus,
            MsgCategory::StaleMessage => &self.stale_messages,
            MsgCategory::ErrorUnknownCommandMessage => &self.error_unknown_command_messages,
            MsgCategory::ErrorRightsViolationMessage => &self.error_rights_violation_messages,
            MsgCategory::ErrorLateIncludeMessage => &self.error_late_include_messages,
            MsgCategory::ErrorLateScoringMessage => &self.error_late_scoring_messages,
            MsgCategory::ErrorSelfScore => &self.error_selfscore_messages,
            MsgCategory::ErrorOrgNotInAllowedListMessage => {
                &self.error_org_not_in_allowed_list_messages
            }
            MsgCategory::ErrorPausePausedMessage => &self.error_pause_paused_messages,
            MsgCategory::ErrorUnpauseUnpausedMessage => &self.error_unpause_unpaused_messages,
            MsgCategory::ErrorPausedMessage => &self.error_paused_messages,

            MsgCategory::FirstTimeContribution => &self.first_time_contribution,
            MsgCategory::FirstWeekContribution => &self.first_week_contribution,
            MsgCategory::FirstMonthContribution => &self.first_month_contribution,
            MsgCategory::Contribution3 => &self.contribution_3,
            MsgCategory::Contribution4 => &self.contribution_4,
            MsgCategory::Contribution5 => &self.contribution_5,
            MsgCategory::Contribution6 => &self.contribution_6,
            MsgCategory::Contribution7 => &self.contribution_7,
            MsgCategory::Contribution8 => &self.contribution_8,
        };
        elem.clone()
    }

    pub fn include_message_text(
        &self,
        bot_name: &str,
        check_info: &PRInfo,
        pr: &PrMetadata,
        user: &User,
    ) -> String {
        let user_specific_message = self.user_specific_message(user);
        let message = self
            .get_message(MsgCategory::IncludeBasicMessage)
            .format(
                [
                    ("pr_author_username".to_string(), user.name.clone()),
                    ("user_specific_message".to_string(), user_specific_message),
                ]
                .into_iter()
                .collect(),
            )
            .unwrap_or_default();
        let status_message = self.status_message(bot_name, check_info, pr);
        let message = self.update_pr_status_message(message, status_message);
        let common = self
            .get_message(MsgCategory::IncludeCommonMessage)
            .format(
                [("pr_author_username".to_string(), user.name.clone())]
                    .into_iter()
                    .collect(),
            )
            .unwrap_or_default();
        message + &common
    }

    pub fn status_message(&self, bot_name: &str, check_info: &PRInfo, pr: &PrMetadata) -> String {
        let mut message = String::new();

        let status = if check_info.excluded {
            "excluded"
        } else if !check_info.exist {
            "stale" // PR was removed for inactivity
        } else if check_info.executed {
            "executed"
        } else if check_info.votes.is_empty() {
            "waiting for scoring"
        } else if !check_info.merged {
            "waiting for merge"
        } else {
            "waiting for finalization"
        };

        message.push_str(&format!(
            "\n<details><summary>Current status: <i>{status}</i></summary>\n",
        ));

        if status == "waiting for scoring" {
            message.push_str(&format!("\nWe're waiting for maintainer to score this pull request with `@{bot_name} score [0,1,2,3,5,8,13]` command"));
        }

        if status == "stale" {
            message.push_str(&format!("\nThis pull request was removed from the race, but you can include it again with `@{bot_name} include` command"));
        }

        if status == "waiting for finalization" {
            message.push_str(&format!("\nThe pull request is merged, you have 24 hours to finalize your scoring. The scoring ends {}", pr.merged.unwrap().add(chrono::Duration::days(1)).format("%c")));
        }

        if !check_info.votes.is_empty() {
            message.push_str("\n| Reviewer | Score |\n");
            message.push_str("|--------|--------|\n");

            for vote in &check_info.votes {
                message.push_str(&format!("| @{}  | {} |\n", vote.user, vote.score));
            }
            let final_score = check_info.average_score();
            message.push_str(&format!("\n**The average score is {}**\n", final_score));
        }

        if status == "executed" {
            message.push_str(&format!(
                "\n@{} check out your results on the [Race of Sloths Leaderboard!]({}) and in the [profile]({}/profile/{})\n",
                pr.author.login, self.leaderboard_link, self.link, pr.author.login
            ));
        }

        message.push_str("\n</details>");

        message
    }

    fn user_specific_message(&self, user: &User) -> String {
        let timestamp = chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default() as u64;
        let current_period = TimePeriod::Month.time_string(timestamp);

        let monthly_statistics = user
            .get_period(&current_period)
            .cloned()
            .unwrap_or_default();

        let all_time_period = TimePeriod::AllTime.time_string(timestamp);
        let all_time_statistics = user
            .get_period(&all_time_period)
            .cloned()
            .unwrap_or_default();

        let weekly_period = TimePeriod::Week.time_string(timestamp);
        let weekly_statistics = user.get_period(&weekly_period).cloned().unwrap_or_default();

        let message_type = if all_time_statistics.prs_opened == 1 {
            MsgCategory::FirstTimeContribution
        } else if monthly_statistics.prs_opened == 1 {
            MsgCategory::FirstMonthContribution
        } else if weekly_statistics.prs_opened == 1 {
            MsgCategory::FirstWeekContribution
        } else if monthly_statistics.prs_opened == 3 {
            MsgCategory::Contribution3
        } else if monthly_statistics.prs_opened == 4 {
            MsgCategory::Contribution4
        } else if monthly_statistics.prs_opened == 5 {
            MsgCategory::Contribution5
        } else if monthly_statistics.prs_opened == 6 {
            MsgCategory::Contribution6
        } else if monthly_statistics.prs_opened == 7 {
            MsgCategory::Contribution7
        } else if monthly_statistics.prs_opened == 8 {
            MsgCategory::Contribution8
        } else {
            return String::new();
        };

        match self.get_message(message_type).format(
            [("pr_author_username".to_string(), user.name.clone())]
                .into_iter()
                .collect(),
        ) {
            Ok(message) => message,
            Err(err) => {
                error!(
                    "Failed to format user-specific message for {}: {err}",
                    user.name,
                );
                String::new()
            }
        }
    }

    pub fn update_pr_status_message(&self, old_text: String, status: String) -> String {
        let place = old_text.find("<details><summary>Current status:");

        if let Some(i) = place {
            let end_details = old_text[i..].find("</details>");
            if let Some(j) = end_details {
                old_text[..i].to_string() + &status + &old_text[i + j + 10..]
            } else {
                tracing::error!("Failed to find the end of the details tag");
                old_text[..i].to_string() + &status
            }
        } else {
            old_text + &status
        }
    }

    pub fn final_message(
        &self,
        user_name: &str,
        total_rating: u32,
        score: u32,
        weekly: u32,
        monthly: u32,
        percent_reward: u32,
        total_percent: u32,
        pr_number_this_week: u32,
    ) -> anyhow::Result<String> {
        let rating = rating_breakthrough(total_rating, score, weekly, monthly, total_percent);
        let final_common = self.get_message(MsgCategory::FinalMessageCommon).format(
            [
                ("score".to_string(), score.to_string()),
                ("rating".to_string(), rating),
            ]
            .into_iter()
            .collect(),
        )?;

        let optional_message = if percent_reward > 0 && total_percent > 5 {
            let rank: &str = match total_percent {
                a if a >= 25 => "Rust",
                a if a >= 20 => "Platinum",
                a if a >= 15 => "Gold",
                a if a >= 10 => "Silver",
                a => {
                    tracing::error!(
                        "Expected total_lifetime_bonus as one of predefined values, but got: {a}. Recovering to Bronze",
                    );
                    "Bronze"
                }
            };
            self.get_message(MsgCategory::FinalMessagesLifetimeBonus)
                .format(
                    [
                        (
                            "total_lifetime_percent".to_string(),
                            total_percent.to_string(),
                        ),
                        ("lifetime_percent".to_string(), percent_reward.to_string()),
                        ("pr_author_username".to_string(), user_name.to_string()),
                        ("rank_name".to_string(), rank.to_string()),
                    ]
                    .into_iter()
                    .collect(),
                )?
        } else if percent_reward > 0 {
            self.get_message(MsgCategory::FinalMessagesFirstLifetimeBonus)
                .format(
                    [("pr_author_username".to_string(), user_name.to_string())]
                        .into_iter()
                        .collect(),
                )?
        } else if monthly > 0 {
            self.get_message(MsgCategory::FinalMessagesMonthlyStreak)
                .format(
                    [("pr_author_username".to_string(), user_name.to_string())]
                        .into_iter()
                        .collect(),
                )?
        } else if weekly > 0 {
            self.get_message(MsgCategory::FinalMessagesWeeklyStreak)
                .format(
                    [("pr_author_username".to_string(), user_name.to_string())]
                        .into_iter()
                        .collect(),
                )?
        } else if pr_number_this_week % 3 == 0 {
            self.get_message(MsgCategory::FinalMessagesFeedbackForm)
                .format([].into_iter().collect())?
        } else {
            String::new()
        };

        Ok(format!("{}\n\n{}", final_common, optional_message))
    }
}

fn rating_breakthrough(
    total_rating: u32,
    score: u32,
    weekly: u32,
    monthly: u32,
    percent: u32,
) -> String {
    let mut result = total_rating.to_string();
    if weekly == 0 && monthly == 0 && percent == 0 {
        return result;
    }

    result.push_str(&format!(" ({} base", score * 10));
    if weekly > 0 {
        result.push_str(&format!(" + {} weekly bonus", weekly));
    }

    if monthly > 0 {
        result.push_str(&format!(" + {} monthly bonus", monthly));
    }

    if percent > 0 {
        result.push_str(&format!(" + {}% lifetime bonus", percent));
    }

    result.push(')');

    result
}

#[cfg(test)]
mod tests {
    use shared::{
        github::{PrMetadata, User},
        Score,
    };

    use super::MessageLoader;

    fn load_message_loader() -> MessageLoader {
        let file = include_str!("../../Messages.toml");
        let mut result: MessageLoader = toml::from_str(file).unwrap();
        result.postprocess_messages_with_link("bot");
        result
    }

    #[test]
    fn test_update_message_with_existing_status() {
        let old_text =
            "Welcome to the race!\n<details><summary>Current status: <i>waiting for scoring</i></summary>";
        let new_status = "#### Current status: executed\n>New status info";
        let expected = "Welcome to the race!\n#### Current status: executed\n>New status info";

        let message_loader = load_message_loader();
        let updated_text =
            message_loader.update_pr_status_message(old_text.to_string(), new_status.to_string());

        assert_eq!(updated_text, expected);
    }

    fn period_data(amount_prs: u32) -> shared::UserPeriodData {
        shared::UserPeriodData {
            total_score: 0,
            executed_prs: 0,
            largest_score: 0,
            prs_opened: amount_prs,
            prs_merged: 0,
            total_rating: 0,
            largest_rating_per_pr: 0,
        }
    }

    #[test]
    fn include() {
        let message_loader = load_message_loader();

        let user = shared::User {
            name: "user".to_string(),
            id: 1,
            percentage_bonus: 5,
            period_data: vec![("all-time".to_string(), period_data(1))],
            streaks: vec![],
        };

        let mut pr_info = shared::PRInfo {
            votes: vec![],
            allowed_org: true,
            allowed_repo: true,
            merged: false,
            executed: false,
            excluded: false,
            exist: true,
        };
        let pr = PrMetadata {
            owner: "a".to_string(),
            repo: "a".to_string(),
            author: User::new(
                "a".to_string(),
                octocrab::models::AuthorAssociation::Contributor,
            ),
            started: chrono::Utc::now(),
            merged: None,
            number: 0,
            updated_at: chrono::Utc::now(),
            full_id: "a/a/0".to_string(),
            body: "".to_string(),
            closed: false,
        };

        let text1 = message_loader.include_message_text("bot", &pr_info, &pr, &user);
        let status_message_init = message_loader.status_message("bot", &pr_info, &pr);
        println!("{}", text1);
        assert!(text1.contains(&status_message_init));

        pr_info.votes.push(Score {
            user: "b".to_string(),
            score: 5,
        });

        let new_status_message = message_loader.status_message("bot", &pr_info, &pr);
        assert_ne!(status_message_init, new_status_message);

        let text2 =
            message_loader.update_pr_status_message(text1.clone(), new_status_message.clone());
        assert_ne!(text1, text2);
        assert!(text2.contains(&new_status_message));
        assert!(!text2.contains(&status_message_init));

        pr_info.executed = true;
        let new_status_message = message_loader.status_message("bot", &pr_info, &pr);
        assert_ne!(status_message_init, new_status_message);

        let text3 =
            message_loader.update_pr_status_message(text1.clone(), new_status_message.clone());
        assert_ne!(text3, text2);
        println!("{}", text3);
        assert!(text3.contains(&new_status_message));
    }

    #[test]
    fn rating_breakthrough_full() {
        let total_rating = 100;
        let score = 5;
        let weekly = 10;
        let monthly = 20;
        let percent = 5;

        let result = super::rating_breakthrough(total_rating, score, weekly, monthly, percent);
        assert_eq!(
            result,
            "100 (50 base + 10 weekly bonus + 20 monthly bonus + 5% lifetime bonus)"
        );
    }

    #[test]
    fn rating_breakthrough_none() {
        let result = super::rating_breakthrough(100, 10, 0, 0, 0);
        assert_eq!(result, "100");
    }

    #[test]
    fn rating_breakthrough_partial() {
        let result = super::rating_breakthrough(100, 5, 0, 5, 0);
        assert_eq!(result, "100 (50 base + 5 monthly bonus)");
    }
}
