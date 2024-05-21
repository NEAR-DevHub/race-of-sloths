use near_sdk::Timestamp;

use super::*;

pub type PRId = String;

#[derive(
    Debug, Clone, BorshDeserialize, BorshSerialize, Serialize, Deserialize, NearSchema, PartialEq,
)]
#[serde(crate = "near_sdk::serde")]
#[borsh(crate = "near_sdk::borsh")]
pub struct Score {
    pub user: GithubHandle,
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
    pub fn average_score(&self) -> u32 {
        if self.votes.is_empty() {
            return 0;
        }

        let total_score: u32 = self.votes.iter().map(|vote| vote.score).sum();
        total_score / self.votes.len() as u32
    }

    pub fn status_message(&self) -> String {
        let mut message = String::from("### 🏆 Race of Sloths Status Update 🏆\n\n");

        if self.excluded {
            message.push_str("Hey there! 🚫 Your PR has been excluded from the Race of Sloths. If you think this is a mistake, please reach out to the maintainers. 🙏\n\n");
            return message;
        }

        if self.executed {
            message.push_str("Hey there! 🎉 Your PR has been executed! Here are the final results. Thanks for being a part of the Race of Sloths! 🙌\n\n");
            if !self.votes.is_empty() {
                message.push_str(&format!(
                    "- **Final Score:** The average score is {}. 🌟\n",
                    self.average_score()
                ));
            } else {
                message.push_str("- **Final Score:** Sorry, your PR wasn't scored. As a result, it's been included with a score of 0. 📉\n");
            }
            return message;
        }

        message.push_str("Hey there! 🎉 Your PR is now part of the Race of Sloths. Thanks for contributing! 🙌\n\n");

        message.push_str("**Current Status:**\n\n");

        if !self.votes.is_empty() {
            message.push_str("- **Scoring:**\n");
            for vote in self.votes.iter() {
                message.push_str(&format!("  - {}: {}\n", vote.user, vote.score));
            }
            if self.votes.len() > 1 {
                message.push_str(&format!("- **Average Score:** {}\n", self.average_score()));
            }
        } else {
            message.push_str("- **Scoring:** No one has scored your PR yet. Maintainers can score using `@race-of-sloths score [1,2,3,5,8,13]`. ⏳\n");
        }

        if self.merged {
            message.push_str("- **Merge Status:** Your PR has been successfully merged! 🎉\n");
        } else {
            message.push_str("- **Merge Status:** Your PR hasn't been tracked as merged yet. Hang tight, it might take a bit of time! ⏳\n");
        }

        message.push_str("\nWe'll keep this status updated as things progress. Thanks again for your awesome contribution! 🌟");

        message
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
    pub author: GithubHandle,
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
        author: GithubHandle,
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

    pub fn add_score(&mut self, user: GithubHandle, score: u32) {
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
        const SCORE_TIMEOUT_IN_SECONDS: Timestamp = 1; // For testing purposes
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

    pub fn pr_id(&self) -> PRId {
        format!("{}/{}/{}", self.organization, self.repo, self.number)
    }
}
