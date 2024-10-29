use near_sdk::Timestamp;

use super::*;

pub type PRId = String;

pub const SCORE_TIMEOUT_IN_SECONDS: Timestamp = 24 * 60 * 60;
pub const SCORE_TIMEOUT_IN_NANOSECONDS: Timestamp = SCORE_TIMEOUT_IN_SECONDS * 1_000_000_000;

#[derive(
    Debug, Clone, BorshDeserialize, BorshSerialize, Serialize, Deserialize, NearSchema, PartialEq,
)]
#[serde(crate = "near_sdk::serde")]
#[borsh(crate = "near_sdk::borsh")]
pub struct Score {
    pub user: GithubHandle,
    pub score: u32,
}

#[derive(Serialize, Debug, Clone, Deserialize, NearSchema, Default)]
#[serde(crate = "near_sdk::serde")]
pub struct PRInfo {
    pub votes: Vec<Score>,
    pub allowed_repo: bool,
    pub paused: bool,
    pub exist: bool,
    pub merged: bool,
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
}

#[derive(
    Debug, Clone, BorshDeserialize, BorshSerialize, Serialize, Deserialize, NearSchema, PartialEq,
)]
#[serde(crate = "near_sdk::serde")]
#[borsh(crate = "near_sdk::borsh")]
pub struct PRv2 {
    pub organization: String,
    pub repo: String,
    pub number: u64,
    pub author: GithubHandle,
    pub score: Vec<Score>,
    pub included_at: Timestamp,
    pub created_at: Option<Timestamp>,
    pub merged_at: Option<Timestamp>,
    pub streak_bonus_rating: u32,
    pub percentage_multiplier: u32,
}

#[derive(Debug, Clone, BorshDeserialize, BorshSerialize, Serialize, Deserialize, NearSchema)]
#[serde(crate = "near_sdk::serde")]
#[borsh(crate = "near_sdk::borsh")]
pub enum VersionedPR {
    V1(PRWithRating),
    V2(PRv2),
}

impl VersionedPR {
    pub fn is_merged(&self) -> bool {
        let data: PRv2 = self.clone().into();

        data.merged_at.is_some()
    }

    pub fn is_ready_to_move(&self, timestamp: Timestamp) -> bool {
        let data: PRv2 = self.clone().into();

        data.is_ready_to_move(timestamp)
    }
}

impl From<VersionedPR> for PRv2 {
    fn from(message: VersionedPR) -> Self {
        match message {
            VersionedPR::V1(x) => PRv2 {
                organization: x.organization,
                repo: x.repo,
                number: x.number,
                author: x.author,
                score: x.score,
                included_at: x.created_at,
                created_at: None,
                merged_at: x.merged_at,
                streak_bonus_rating: x.streak_bonus_rating,
                percentage_multiplier: x.percentage_multiplier,
            },
            VersionedPR::V2(x) => x,
        }
    }
}

#[derive(
    Debug, Clone, BorshDeserialize, BorshSerialize, Serialize, Deserialize, NearSchema, PartialEq,
)]
#[serde(crate = "near_sdk::serde")]
#[borsh(crate = "near_sdk::borsh")]
pub struct PRWithRating {
    pub organization: String,
    pub repo: String,
    pub number: u64,
    pub author: GithubHandle,
    pub score: Vec<Score>,
    pub created_at: Timestamp,
    pub merged_at: Option<Timestamp>,
    pub streak_bonus_rating: u32,
    pub percentage_multiplier: u32,
}

impl PRv2 {
    pub fn new(
        organization: String,
        repo: String,
        number: u64,
        author: GithubHandle,
        included_at: Timestamp,
        created_at: Timestamp,
    ) -> Self {
        Self {
            organization,
            repo,
            number,
            author,
            included_at,
            created_at: Some(created_at),

            score: vec![],
            merged_at: None,
            streak_bonus_rating: 0,
            percentage_multiplier: 0,
        }
    }

    // Returns the old score if the user already had already scored
    pub fn add_score(&mut self, user: GithubHandle, score: u32) -> Option<u32> {
        if let Some(user) = self.score.iter_mut().find(|s| s.user == user) {
            let old_score = user.score;
            user.score = score;
            Some(old_score)
        } else {
            self.score.push(Score { user, score });
            None
        }
    }

    pub fn add_merge_info(&mut self, merged_at: Timestamp) {
        self.merged_at = Some(merged_at);
    }

    pub fn ready_to_move_timestamp(&self) -> Option<Timestamp> {
        self.merged_at.map(|t| t + SCORE_TIMEOUT_IN_NANOSECONDS)
    }

    pub fn is_ready_to_move(&self, timestamp: Timestamp) -> bool {
        self.merged_at.is_some()
            && (timestamp - self.merged_at.unwrap()) > SCORE_TIMEOUT_IN_NANOSECONDS
    }

    pub fn rating(&self) -> u32 {
        let score = self.score().unwrap_or_default() * 10 + self.streak_bonus_rating;
        let percentage = (self.percentage_multiplier + 100) as f64;
        ((score as f64 * percentage / 100.0).ceil()) as u32
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
