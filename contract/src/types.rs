use std::collections::HashMap;

use chrono::{DateTime, Datelike};
use near_sdk::{
    borsh::{BorshDeserialize, BorshSerialize},
    env,
    serde::{Deserialize, Serialize},
    AccountId, NearSchema,
};

type MonthYearCode = [u8; 6];

// We need to carefully think what we want to store in the contract storage
#[derive(Debug, Clone, BorshDeserialize, BorshSerialize, Serialize, Deserialize, NearSchema)]
#[serde(crate = "near_sdk::serde")]
#[borsh(crate = "near_sdk::borsh")]
pub struct UserData {
    pub handle: String,
    pub score: HashMap<MonthYearCode, u64>,
    pub total_prs_merged: u64,
    pub total_score: u64,
    // Created for the future, but we would need to think more about it
    pub account_id: Option<AccountId>,
}

impl UserData {
    pub fn new(handle: String) -> Self {
        Self {
            handle,
            score: HashMap::new(),
            total_prs_merged: 0,
            total_score: 0,
            account_id: None,
        }
    }

    pub fn add_score(&mut self, score: u64) {
        let timestamp = env::block_timestamp();
        let month_year_code = timestamp_to_code(timestamp);

        self.score.insert(month_year_code, score);
        self.total_prs_merged += 1;
        self.total_score += score;
    }
}

fn timestamp_to_code(timestamp: u64) -> MonthYearCode {
    let date = DateTime::from_timestamp_nanos(timestamp as i64);

    let month = date.month();
    let year = date.year();
    let code = format!("{:02}{:04}", month, year);
    let code = code.as_bytes();
    let mut month_year_code = [0u8; 6];
    month_year_code.copy_from_slice(code);
    month_year_code
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timestamp_to_code() {
        let timestamp = 1625097600000000000; // 2021-07-01
        let code = timestamp_to_code(timestamp);
        assert_eq!(code, [b'0', b'7', b'2', b'0', b'2', b'1']);
    }
}
