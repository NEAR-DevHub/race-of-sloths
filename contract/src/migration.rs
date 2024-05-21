use self::storage::StorageKey;

use super::*;

// We need to carefully think what we want to store in the contract storage
#[derive(Debug, Clone, BorshDeserialize, BorshSerialize)]
#[borsh(crate = "near_sdk::borsh")]
pub struct UserData {
    pub handle: String,
    pub total_prs_merged: u32,
    pub total_prs_opened: u32,
    // Created for the future, but we would need to think more about it
    pub account_id: Option<AccountId>,
}

#[derive(BorshDeserialize, BorshSerialize)]
#[borsh(crate = "near_sdk::borsh")]
pub struct OldState {
    sloth: AccountId,
    #[allow(deprecated)]
    sloths: UnorderedMap<String, UserData>,
    #[allow(deprecated)]
    sloths_per_month: UnorderedMap<(String, TimePeriodString), u32>,
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
        let mut old_state = env::state_read::<OldState>().unwrap();
        old_state.sloths_per_month.clear();

        let sloths_per_period = UnorderedMap::new(StorageKey::SlothsPerPeriod);
        let mut accounts = UnorderedMap::new(StorageKey::Accounts);
        old_state.sloths.into_iter().for_each(|(key, value)| {
            accounts.insert(
                key.clone(),
                Account {
                    account_id: value.account_id.clone(),
                },
            );
        });
        old_state.sloths.clear();

        let mut contract = Self {
            sloth: old_state.sloth,
            sloths_per_period,
            organizations: old_state.organizations,
            prs: old_state.prs,
            executed_prs: old_state.executed_prs,
            excluded_prs: old_state.excluded_prs,
            accounts,
            streaks: Vector::new(StorageKey::Streaks),
            user_streaks: UnorderedMap::new(StorageKey::UserStreaks),
        };

        for value in contract.executed_prs.values().cloned().collect::<Vec<_>>() {
            contract.apply_to_periods(&value.author, value.created_at, |period| {
                period.prs_opened += 1;
            });
            contract.apply_to_periods(&value.author, value.merged_at.unwrap(), |period| {
                period.prs_merged += 1;
                period.total_score += value.score().unwrap();
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
