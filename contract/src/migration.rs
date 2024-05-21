use near_sdk::{
    env::log_str,
    serde::{Deserialize, Serialize},
    NearSchema,
};

use self::storage::StorageKey;

use super::*;

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

#[derive(BorshDeserialize, BorshSerialize)]
#[borsh(crate = "near_sdk::borsh")]
pub struct OldState {
    sloth: AccountId,
    #[allow(deprecated)]
    sloths: UnorderedMap<String, UserData>,
    #[allow(deprecated)]
    sloths_per_month: UnorderedMap<(String, String), u32>,
    #[allow(deprecated)]
    organizations: UnorderedMap<String, Organization>,
    // We need to think about removing PRs that are stale for a long time
    #[allow(deprecated)]
    prs: UnorderedMap<String, PR>,
    #[allow(deprecated)]
    executed_prs: UnorderedMap<String, PR>,
    excluded_prs: LookupSet<String>,
}

#[near_bindgen]
impl Contract {
    #[init(ignore_state)]
    pub fn migrate() -> Self {
        let mut old_state = env::state_read::<OldState>().expect("Old state doesn't exist");
        old_state.sloths_per_month.clear();

        #[allow(deprecated)]
        let sloths_per_period = UnorderedMap::new(StorageKey::SlothsPerPeriod);

        #[allow(deprecated)]
        let accounts = UnorderedMap::new(StorageKey::Accounts);

        let mut contract = Self {
            sloth: old_state.sloth,
            sloths_per_period,
            organizations: old_state.organizations,
            prs: old_state.prs,
            executed_prs: old_state.executed_prs,
            excluded_prs: old_state.excluded_prs,
            accounts,
            streaks: Vector::new(StorageKey::Streaks),
            #[allow(deprecated)]
            user_streaks: UnorderedMap::new(StorageKey::UserStreaks),
        };

        for (key, _) in old_state.sloths.drain() {
            log_str(&format!("Migrating user: {}", key));
            contract.accounts.insert(key, Default::default());
        }

        contract.streaks.push(Streak::new(
            0,
            TimePeriod::Week,
            vec![StreakType::PRsOpened(1)],
        ));
        contract.streaks.push(Streak::new(
            1,
            TimePeriod::Month,
            vec![StreakType::LargestScore(8)],
        ));

        for value in contract.executed_prs.values().cloned().collect::<Vec<_>>() {
            contract.apply_to_periods(&value.author, value.created_at, |period| {
                period.prs_opened += 1;
            });
            contract.apply_to_periods(&value.author, value.merged_at.unwrap(), |period| {
                period.prs_merged += 1;
                period.total_score += value.score().unwrap_or_default();
                period.executed_prs += 1;
            });
        }

        for value in contract.executed_prs.values().cloned().collect::<Vec<_>>() {
            contract.apply_to_periods(&value.author, value.created_at, |period| {
                period.prs_opened += 1;
            });
            if let Some(merged_at) = value.merged_at {
                contract.apply_to_periods(&value.author, merged_at, |period| {
                    period.prs_merged += 1;
                });
            }
        }

        for user in old_state.sloths.values() {
            contract.calculate_streak(&user.handle);
        }

        contract
    }
}
