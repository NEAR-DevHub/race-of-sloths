use std::collections::HashSet;

use near_sdk::{
    near_bindgen,
    serde::{Deserialize, Serialize},
    NearSchema,
};

use self::storage::StorageKey;

use super::*;

#[derive(Debug, Clone, BorshDeserialize, BorshSerialize, Serialize, Deserialize, NearSchema)]
#[serde(crate = "near_sdk::serde")]
#[borsh(crate = "near_sdk::borsh")]
enum PermissionModel {
    // Represents the list of repositories that are allowed
    Allowlist(HashSet<String>),
    // Represents `all, except` list of repositories
    Blocklist(HashSet<String>),
}

#[derive(Debug, Clone, BorshDeserialize, BorshSerialize, Serialize, Deserialize, NearSchema)]
#[serde(crate = "near_sdk::serde")]
#[borsh(crate = "near_sdk::borsh")]
pub enum VersionedOrganization {
    V1(Organization),
}

#[derive(Debug, Clone, BorshDeserialize, BorshSerialize, Serialize, Deserialize, NearSchema)]
#[serde(crate = "near_sdk::serde")]
#[borsh(crate = "near_sdk::borsh")]
pub struct Organization {
    pub name: String,
    all: PermissionModel,
}

#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
#[borsh(crate = "near_sdk::borsh")]
pub struct OldContract {
    sloth: AccountId,
    account_ids: LookupMap<GithubHandle, UserId>,
    users: Vector<VersionedAccount>,
    sloths_per_period: LookupMap<(UserId, TimePeriodString), VersionedUserPeriodData>,
    #[allow(deprecated)]
    organizations: UnorderedMap<GithubHandle, VersionedOrganization>,
    // We need to think about removing PRs that are stale for a long time
    #[allow(deprecated)]
    prs: UnorderedMap<PRId, VersionedPR>,
    #[allow(deprecated)]
    executed_prs: UnorderedMap<PRId, VersionedPR>,
    excluded_prs: LookupSet<PRId>,

    // Configured streaks
    streaks: Vector<VersionedStreak>,
    user_streaks: LookupMap<(UserId, StreakId), VersionedStreakUserData>,
}

#[near_bindgen]
impl Contract {
    #[init(ignore_state)]
    #[private]
    pub fn migrate() -> Self {
        let mut contract: OldContract = near_sdk::env::state_read().unwrap();

        let mut new_contract = Self {
            sloth: contract.sloth,
            account_ids: contract.account_ids,
            users: contract.users,
            sloths_per_period: contract.sloths_per_period,
            prs: contract.prs,
            executed_prs: contract.executed_prs,
            excluded_prs: contract.excluded_prs,
            streaks: contract.streaks,
            user_streaks: contract.user_streaks,
            #[allow(deprecated)]
            repos: UnorderedMap::new(StorageKey::Repos),
        };

        for (org, data) in contract.organizations.drain() {
            let VersionedOrganization::V1(new_org) = data;

            if let PermissionModel::Allowlist(repos) = new_org.all {
                for repo in repos {
                    new_contract.repos.insert(
                        (org.clone(), repo),
                        VersionedRepository::V1(Repository { paused: false }),
                    );
                }
            }
        }

        new_contract
    }
}
