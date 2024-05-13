use self::storage::StorageKey;

use super::*;

#[derive(BorshDeserialize, BorshSerialize)]
#[borsh(crate = "near_sdk::borsh")]
pub struct OldState {
    sloth: AccountId,
    #[allow(deprecated)]
    sloths: UnorderedMap<String, UserData>,
    #[allow(deprecated)]
    sloths_per_month: UnorderedMap<(String, MonthYearString), u32>,
    #[allow(deprecated)]
    organizations: UnorderedMap<String, Organization>,
    // We need to think about removing PRs that are stale for a long time
    #[allow(deprecated)]
    prs: UnorderedMap<String, PR>,
    #[allow(deprecated)]
    executed_prs: UnorderedMap<String, PR>,
}

#[near_bindgen]
impl Contract {
    #[init(ignore_state)]
    pub fn migrate() -> Self {
        let old_state = env::state_read::<OldState>().unwrap();

        Self {
            sloth: old_state.sloth,
            sloths: old_state.sloths,
            sloths_per_month: old_state.sloths_per_month,
            organizations: old_state.organizations,
            prs: old_state.prs,
            executed_prs: old_state.executed_prs,
            excluded_prs: LookupSet::new(StorageKey::ExcludedPRs),
        }
    }
}
