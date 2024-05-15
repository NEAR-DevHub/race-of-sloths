#[allow(deprecated)]
use near_sdk::store::UnorderedMap;
use near_sdk::{
    borsh::{BorshDeserialize, BorshSerialize},
    store::LookupSet,
    Timestamp,
};
use near_sdk::{env, near_bindgen, AccountId, PanicOnDefault};
use shared_types::{MonthYearString, UserData, PR};
use types::{timestamp_to_month_string, Organization};

pub mod migration;
pub mod storage;
pub mod types;
pub mod views;

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
#[borsh(crate = "near_sdk::borsh")]
pub struct Contract {
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
    excluded_prs: LookupSet<String>,
}

#[near_bindgen]
impl Contract {
    #[init]
    #[init(ignore_state)]
    pub fn new(sloth: AccountId) -> Self {
        Self {
            sloth,
            #[allow(deprecated)]
            sloths: UnorderedMap::new(storage::StorageKey::Sloths),
            #[allow(deprecated)]
            sloths_per_month: UnorderedMap::new(storage::StorageKey::SlothsPerMonth),
            #[allow(deprecated)]
            organizations: UnorderedMap::new(storage::StorageKey::Organizations),
            #[allow(deprecated)]
            prs: UnorderedMap::new(storage::StorageKey::PRs),
            #[allow(deprecated)]
            executed_prs: UnorderedMap::new(storage::StorageKey::MergedPRs),
            excluded_prs: LookupSet::new(storage::StorageKey::ExcludedPRs),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn sloth_include(
        &mut self,
        organization: String,
        repo: String,
        user: String,
        pr_number: u64,
        started_at: Timestamp,
        override_exclude: bool,
        comment_id: u64,
    ) {
        self.assert_sloth();
        self.assert_organization_allowed(&organization, &repo);

        let pr_id = format!("{organization}/{repo}/{pr_number}");

        if self.excluded_prs.contains(&pr_id) {
            if !override_exclude {
                env::panic_str("Excluded PR cannot be included without override flag")
            }
            self.excluded_prs.remove(&pr_id);
        }

        // Check if PR already exists
        let pr = self.prs.get(&pr_id);
        let executed_pr = self.executed_prs.get(&pr_id);
        if pr.is_some() || executed_pr.is_some() {
            env::panic_str("PR already exists: {pr_id}")
        }

        // Create user if it doesn't exist
        let mut user_data = self.get_user(user.clone());
        user_data.add_opened_pr();
        self.sloths.insert(user_data.handle.clone(), user_data);

        let pr = PR::new(organization, repo, pr_number, user, started_at, comment_id);
        self.prs.insert(pr_id, pr);
    }

    pub fn sloth_scored(&mut self, pr_id: String, user: String, score: u32) {
        self.assert_sloth();

        let pr = self.prs.get_mut(&pr_id);
        if pr.is_none() {
            env::panic_str("PR is not started or already executed")
        }

        let pr = pr.unwrap();
        pr.add_score(user, score);
    }

    pub fn sloth_merged(&mut self, pr_id: String, merged_at: Timestamp) {
        self.assert_sloth();

        let pr = self.prs.get_mut(&pr_id);
        if pr.is_none() {
            env::panic_str("PR is not started or already executed")
        }

        let pr = pr.unwrap();
        pr.add_merge_info(merged_at);
    }

    pub fn sloth_exclude(&mut self, pr_id: String) {
        self.assert_sloth();

        self.prs.remove(&pr_id);
        self.excluded_prs.insert(pr_id);
    }

    pub fn allow_organization(&mut self, organization: String) {
        self.assert_sloth();

        if self.organizations.get(&organization).is_some() {
            env::panic_str("Organization already allowlisted")
        }

        let org = Organization::new_all(organization);
        self.organizations.insert(org.name.clone(), org);
    }

    pub fn exclude_repo(&mut self, organization: String, repo: String) {
        self.assert_sloth();

        let org = self.organizations.get_mut(&organization);
        if org.is_none() {
            env::panic_str("Organization is not in the list")
        }

        let org = org.unwrap();
        org.exclude(&repo);
    }

    pub fn include_repo(&mut self, organization: String, repo: String) {
        self.assert_sloth();

        let org = self.organizations.get_mut(&organization);
        if org.is_none() {
            let org = Organization::new_only(organization, vec![repo].into_iter().collect());
            self.organizations.insert(org.name.clone(), org);
            return;
        }

        let org = org.unwrap();
        org.include(&repo);
    }

    pub fn sloth_stale(&mut self, pr_id: String) {
        self.assert_sloth();

        let pr = self.prs.get(&pr_id);
        if pr.is_none() {
            env::panic_str("PR is not started or already executed")
        }
        let pr = pr.unwrap();

        self.sloths.get_mut(&pr.author).map(|user| {
            user.total_prs_opened -= 1;
            user
        });
        self.prs.remove(&pr_id);
    }

    pub fn sloth_finalize(&mut self, pr_id: String) {
        self.assert_sloth();

        let pr = self.prs.get(&pr_id).cloned();
        let pr = if let Some(pr) = pr {
            pr
        } else {
            env::panic_str("PR is not started or already executed")
        };

        if !pr.is_ready_to_move(env::block_timestamp()) {
            env::panic_str("PR is not ready to be finalized")
        }

        // Reward with zero score if PR wasn't scored to track the number of merged PRs
        let score = pr.score().unwrap_or_default();
        let mut user = self.get_user(pr.author.clone());
        let user_name = user.handle.clone();

        user.add_score(score);
        self.sloths.insert(user.handle.clone(), user);

        *self
            .sloths_per_month
            .entry((user_name, timestamp_to_month_string(pr.merged_at.unwrap())))
            .or_default() += score;

        let full_id = pr.full_id();
        self.prs.remove(&full_id);
        self.executed_prs.insert(full_id, pr);
    }
}

impl Contract {
    pub fn assert_sloth(&self) {
        if env::predecessor_account_id() != self.sloth {
            env::panic_str("Only sloth can call this method")
        }
    }

    pub fn assert_organization_allowed(&self, organization: &str, repo: &str) {
        let org = self.organizations.get(organization);
        if let Some(org) = org {
            if !org.is_allowed(repo) {
                env::panic_str("The repository is not allowlisted for the organization")
            }
        } else {
            env::panic_str("Organization is not allowlisted")
        }
    }

    pub fn get_user(&self, user_handle: String) -> UserData {
        self.sloths
            .get(&user_handle)
            .cloned()
            .unwrap_or_else(|| UserData::new(user_handle))
    }
}
