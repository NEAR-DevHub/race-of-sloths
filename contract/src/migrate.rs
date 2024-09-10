use self::storage::StorageKey;

use super::*;

#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
#[borsh(crate = "near_sdk::borsh")]
pub struct OldState {
    sloth: AccountId,
    account_ids: LookupMap<GithubHandle, UserId>,
    users: Vector<VersionedAccount>,
    sloths_per_period: LookupMap<(UserId, TimePeriodString), VersionedUserPeriodData>,
    // We need to think about removing PRs that are stale for a long time
    #[allow(deprecated)]
    prs: UnorderedMap<PRId, VersionedPR>,
    #[allow(deprecated)]
    executed_prs: UnorderedMap<PRId, VersionedPR>,
    excluded_prs: LookupSet<PRId>,

    // Configured streaksd
    streaks: Vector<VersionedStreak>,
    user_streaks: LookupMap<(UserId, StreakId), VersionedStreakUserData>,

    // Repo allowlist
    #[allow(deprecated)]
    repos: UnorderedMap<(GithubHandle, GithubHandle), VersionedRepository>,
}

#[near_bindgen]
impl Contract {
    #[init(ignore_state)]
    #[private]
    pub fn migrate() -> Self {
        let mut state: OldState = env::state_read().unwrap();
        let mut repos = IterableMap::new(StorageKey::ReposNew);
        let mut prs = IterableMap::new(StorageKey::PRs);
        let mut executed_prs = IterableMap::new(StorageKey::MergedPRs);

        for (key, value) in state.repos.drain() {
            repos.insert(key, value);
        }

        for (key, value) in state.prs.drain() {
            prs.insert(key, value);
        }

        for (key, value) in state.executed_prs.drain() {
            executed_prs.insert(key, value);
        }

        Self {
            sloth: state.sloth,
            account_ids: state.account_ids,
            users: state.users,
            sloths_per_period: state.sloths_per_period,
            prs,
            executed_prs,
            excluded_prs: state.excluded_prs,
            streaks: state.streaks,
            user_streaks: state.user_streaks,
            repos,
        }
    }
}
