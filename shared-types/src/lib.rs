use near_sdk::{
    borsh::{BorshDeserialize, BorshSerialize},
    serde::{Deserialize, Serialize},
    AccountId, NearSchema, Timestamp,
};

pub type MonthYearString = String;

#[derive(
    Debug, Clone, BorshDeserialize, BorshSerialize, Serialize, Deserialize, NearSchema, PartialEq,
)]
#[serde(crate = "near_sdk::serde")]
#[borsh(crate = "near_sdk::borsh")]
pub struct Score {
    pub user: String,
    pub score: u32,
}

#[derive(Serialize, Deserialize, NearSchema)]
#[serde(crate = "near_sdk::serde")]
pub struct PRInfo {
    pub comment_id: u64,
    pub votes: Vec<Score>,
    pub allowed_org: bool,
    pub allowed_repo: bool,
    pub exist: bool,
    pub merged: bool,
    pub scored: bool,
    pub executed: bool,
    pub excluded: bool,
}

impl PRInfo {
    pub fn status_message(&self) -> String {
        let mut message = String::from("### 🏆 Race of Sloths Status Update 🏆\n\n");

        if self.excluded {
            message.push_str("Hey there! 🚫 Your PR has been excluded from the Race of Sloths. If you think this is a mistake, please reach out to the maintainers. 🙏\n\n");
            return message;
        }

        message.push_str(
            "Hey there! 🎉 Your PR is now part of the Race of Sloths. Thanks for contributing! 🙌\n\n",
        );

        message.push_str("**Current Status:**\n\n");

        if !self.votes.is_empty() {
            message.push_str("- **Scoring:**\n");
            for vote in self.votes.iter() {
                message.push_str(&format!("  - {}: {}\n", vote.user, vote.score));
            }
        } else {
            message.push_str(
                "- **Scoring:** No one has scored your PR yet. Maintainers can score using `@race-of-sloths score [1,2,3,5,8,13]`.\n",
            );
        }

        if self.merged {
            message.push_str("- **Merge Status:** Your PR has been successfully merged! 🎉\n");
        } else {
            message.push_str("- **Merge Status:** Your PR hasn't been tracked as merged yet. Hang tight, it might take a bit of time! ⏳\n");
        }

        if self.executed {
            message.push_str("- **Execution Status:** The PR has been executed. Great job! 🚀\n");
        } else {
            message.push_str(
                "- **Execution Status:** The PR hasn't been executed yet. Stay tuned! 👀\n",
            );
        }

        message.push_str("\nWe'll keep this status updated as things progress. Thanks again for your awesome contribution! 🌟");

        message
    }
}

#[derive(Debug, Serialize, Deserialize, NearSchema)]
#[serde(crate = "near_sdk::serde")]
pub struct UserWithMonthScore {
    pub user: UserData,
    pub score: u32,
    pub month: MonthYearString,
}

// We need to carefully think what we want to store in the contract storage
#[derive(Debug, Clone, BorshDeserialize, BorshSerialize, Serialize, Deserialize, NearSchema)]
#[serde(crate = "near_sdk::serde")]
#[borsh(crate = "near_sdk::borsh")]
pub struct UserData {
    pub handle: String,
    pub total_prs_merged: u32,
    pub total_prs_opened: u32,
    pub total_score: u32,
    // Created for the future, but we would need to think more about it
    pub account_id: Option<AccountId>,
}

impl UserData {
    pub fn new(handle: String) -> Self {
        Self {
            handle,
            total_prs_merged: 0,
            total_score: 0,
            total_prs_opened: 0,
            account_id: None,
        }
    }

    pub fn add_score(&mut self, score: u32) {
        self.total_prs_merged += 1;
        self.total_score += score;
    }

    pub fn add_opened_pr(&mut self) {
        self.total_prs_opened += 1;
    }
}

#[derive(
    Debug, Clone, BorshDeserialize, BorshSerialize, Serialize, Deserialize, NearSchema, PartialEq,
)]
#[serde(crate = "near_sdk::serde")]
#[borsh(crate = "near_sdk::borsh")]
pub struct PR {
    pub organization: String,
    pub repo: String,
    pub number: u64,
    pub author: String,
    pub score: Vec<Score>,
    pub created_at: Timestamp,
    pub merged_at: Option<Timestamp>,
    pub comment_id: u64,
}

impl PR {
    pub fn new(
        organization: String,
        repo: String,
        number: u64,
        author: String,
        created_at: Timestamp,
        comment_id: u64,
    ) -> Self {
        Self {
            organization,
            repo,
            number,
            author,
            created_at,
            comment_id,

            score: vec![],
            merged_at: None,
        }
    }

    pub fn add_score(&mut self, user: String, score: u32) {
        if let Some(user) = self.score.iter_mut().find(|s| s.user == user) {
            user.score = score;
        } else {
            self.score.push(Score { user, score });
        }
    }

    pub fn add_merge_info(&mut self, merged_at: Timestamp) {
        self.merged_at = Some(merged_at);
    }

    pub fn is_ready_to_move(&self, timestamp: Timestamp) -> bool {
        // const SCORE_TIMEOUT_IN_SECONDS: Timestamp = 24 * 60 * 60;
        const SCORE_TIMEOUT_IN_SECONDS: Timestamp = 60;
        const SCORE_TIMEOUT_IN_NANOSECONDS: Timestamp = SCORE_TIMEOUT_IN_SECONDS * 1_000_000_000;

        self.merged_at.is_some()
            && (timestamp - self.merged_at.unwrap()) > SCORE_TIMEOUT_IN_NANOSECONDS
    }

    pub fn score(&self) -> Option<u32> {
        self.score
            .iter()
            .map(|s| s.score)
            .sum::<u32>()
            .checked_div(self.score.len() as u32)
    }

    pub fn full_id(&self) -> String {
        format!("{}/{}/{}", self.organization, self.repo, self.number)
    }
}
