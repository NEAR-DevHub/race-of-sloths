#[allow(deprecated)]
use near_sdk::store::UnorderedMap;
use near_sdk::{
    borsh::{BorshDeserialize, BorshSerialize},
    require,
    store::{LookupSet, Vector},
    Timestamp,
};
use near_sdk::{env, near_bindgen, AccountId, PanicOnDefault};
use shared_types::{
    GithubHandle, IntoEnumIterator, PRId, Streak, StreakId, StreakType, StreakUserData, TimePeriod,
    TimePeriodString, UserPeriodData, VersionedPR, VersionedStreak, VersionedStreakUserData,
    VersionedUserPeriodData, PR,
};
use types::{Account, Organization, VersionedAccount, VersionedOrganization};

pub mod storage;
pub mod types;
pub mod views;

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
#[borsh(crate = "near_sdk::borsh")]
pub struct Contract {
    sloth: AccountId,
    #[allow(deprecated)]
    accounts: UnorderedMap<GithubHandle, VersionedAccount>,
    #[allow(deprecated)]
    sloths_per_period: UnorderedMap<(GithubHandle, TimePeriodString), VersionedUserPeriodData>,
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
    #[allow(deprecated)]
    user_streaks: UnorderedMap<(GithubHandle, StreakId), VersionedStreakUserData>,
}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new(sloth: AccountId) -> Self {
        let mut contract = Self {
            sloth,
            #[allow(deprecated)]
            accounts: UnorderedMap::new(storage::StorageKey::Accounts),
            #[allow(deprecated)]
            sloths_per_period: UnorderedMap::new(storage::StorageKey::SlothsPerPeriod),
            #[allow(deprecated)]
            organizations: UnorderedMap::new(storage::StorageKey::Organizations),
            #[allow(deprecated)]
            prs: UnorderedMap::new(storage::StorageKey::PRs),
            #[allow(deprecated)]
            executed_prs: UnorderedMap::new(storage::StorageKey::MergedPRs),
            excluded_prs: LookupSet::new(storage::StorageKey::ExcludedPRs),
            streaks: Vector::new(storage::StorageKey::Streaks),
            #[allow(deprecated)]
            user_streaks: UnorderedMap::new(storage::StorageKey::UserStreaks),
        };

        contract.allow_organization("NEAR-DevHub".to_owned());
        contract.allow_organization("akorchyn".to_owned());

        contract.create_streak(
            "Weekly PR".to_owned(),
            TimePeriod::Week,
            vec![StreakType::PRsOpened(1)],
        );
        contract.create_streak(
            "Monthly PR with score higher 8".to_owned(),
            TimePeriod::Month,
            vec![StreakType::LargestScore(8)],
        );

        contract
    }

    pub fn create_streak(
        &mut self,
        name: String,
        time_period: TimePeriod,
        streak_criterias: Vec<StreakType>,
    ) {
        self.assert_sloth();
        let id = self.streaks.len();
        let streak = Streak::new(id, name, time_period, streak_criterias);
        self.streaks.push(VersionedStreak::V1(streak));
    }

    pub fn deactivate_streak(&mut self, id: u32) {
        self.assert_sloth();

        let streak = self.streaks.get_mut(id);
        if streak.is_none() {
            env::panic_str("Streak doesn't exist")
        }

        let streak = streak.unwrap();
        match streak {
            VersionedStreak::V1(streak) => {
                streak.is_active = false;
            }
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
        self.get_or_create_account(&user);

        let pr_id = format!("{organization}/{repo}/{pr_number}");

        if self.excluded_prs.contains(&pr_id) {
            if !override_exclude {
                env::panic_str("Excluded PR cannot be included without override flag")
            }
            self.excluded_prs.remove(&pr_id);
        }

        // Check if PR already exists
        let pr = self.prs.get(&pr_id).or(self.executed_prs.get(&pr_id));
        if pr.is_some() {
            env::panic_str("PR already exists: {pr_id}")
        }

        // Create user if it doesn't exist
        let pr = PR::new(organization, repo, pr_number, user, started_at, comment_id);

        self.apply_to_periods(&pr.author, started_at, |data| data.pr_opened());
        self.prs.insert(pr_id, VersionedPR::V1(pr));
    }

    pub fn sloth_scored(&mut self, pr_id: String, user: String, score: u32) {
        self.assert_sloth();

        let pr = match self.prs.get_mut(&pr_id) {
            Some(VersionedPR::V1(pr)) => pr,
            None => env::panic_str("PR is not started or already executed"),
        };

        pr.add_score(user, score);
    }

    pub fn sloth_merged(&mut self, pr_id: String, merged_at: Timestamp) {
        self.assert_sloth();

        let mut pr = match self.prs.get(&pr_id).cloned() {
            Some(VersionedPR::V1(pr)) => pr,
            None => env::panic_str("PR is not started or already executed"),
        };
        pr.add_merge_info(merged_at);

        self.apply_to_periods(&pr.author, merged_at, |data| data.pr_merged());
        self.prs.insert(pr_id.clone(), VersionedPR::V1(pr));
    }

    pub fn sloth_exclude(&mut self, pr_id: String) {
        self.assert_sloth();
        let pr = match self.prs.get(&pr_id).cloned() {
            Some(VersionedPR::V1(pr)) => pr,
            None => env::panic_str("PR is not started or already executed"),
        };
        if pr.merged_at.is_some() {
            env::panic_str("Merged PR cannot be excluded")
        }

        self.apply_to_periods(&pr.author, pr.created_at, |data| {
            data.pr_closed();
        });

        self.prs.remove(&pr_id);
        self.excluded_prs.insert(pr_id);
    }

    pub fn allow_organization(&mut self, organization: String) {
        self.assert_sloth();

        if self.organizations.get(&organization).is_some() {
            env::panic_str("Organization already allowlisted")
        }

        let org = Organization::new_all(organization);
        self.organizations
            .insert(org.name.clone(), VersionedOrganization::V1(org));
    }

    pub fn exclude_repo(&mut self, organization: String, repo: String) {
        self.assert_sloth();

        let org = match self.organizations.get_mut(&organization) {
            Some(VersionedOrganization::V1(org)) => org,
            _ => env::panic_str("Organization is not in the list"),
        };

        org.exclude(&repo);
    }

    pub fn include_repo(&mut self, organization: String, repo: String) {
        self.assert_sloth();

        match self.organizations.get_mut(&organization) {
            Some(VersionedOrganization::V1(org)) => {
                org.include(&repo);
            }
            None => {
                let org = Organization::new_only(organization, vec![repo].into_iter().collect());
                self.organizations
                    .insert(org.name.clone(), VersionedOrganization::V1(org));
            }
        }
    }

    pub fn sloth_stale(&mut self, pr_id: String) {
        self.assert_sloth();

        let pr = match self.prs.get(&pr_id).cloned() {
            Some(VersionedPR::V1(pr)) => pr,
            None => env::panic_str("PR is not started or already executed"),
        };
        require!(pr.merged_at.is_none(), "Merged PR cannot be stale");

        self.apply_to_periods(&pr.author, pr.created_at, |data| data.pr_closed());
        self.prs.remove(&pr_id);
    }

    pub fn sloth_finalize(&mut self, pr_id: String) {
        self.assert_sloth();

        let pr = match self.prs.get(&pr_id).cloned() {
            Some(VersionedPR::V1(pr)) => pr,
            None => env::panic_str("PR is not started or already executed"),
        };

        if !pr.is_ready_to_move(env::block_timestamp()) {
            env::panic_str("PR is not ready to be finalized")
        }

        // Reward with zero score if PR wasn't scored to track the number of merged PRs
        let score = pr.score().unwrap_or_default();

        self.apply_to_periods(&pr.author, pr.merged_at.unwrap(), |data| {
            data.pr_executed(score)
        });

        let full_id = pr.pr_id();
        self.prs.remove(&full_id);
        self.executed_prs.insert(full_id, VersionedPR::V1(pr));
    }
}

impl Contract {
    pub fn calculate_streak(&mut self, user: &String) {
        let current_time = env::block_timestamp();
        for streak in self.streaks.iter() {
            let streak: Streak = streak.clone().into();

            if !streak.is_active {
                continue;
            }

            let key = (user.clone(), streak.id);
            let mut streak_data: StreakUserData = self
                .user_streaks
                .get(&key)
                .cloned()
                .unwrap_or_else(|| VersionedStreakUserData::V1(Default::default()))
                .into();
            let current_time_string = streak.time_period.time_string(current_time);

            // Check if user accomplished the streak for current period
            let achieved = self
                .sloths_per_period
                .get(&(user.clone(), current_time_string.clone()))
                .map(|s| streak.is_streak_achieved(s))
                .unwrap_or_default();

            let streak = self.verify_previous_streak(&streak, &streak_data, user, current_time);

            match (
                streak_data.latest_time_string == current_time_string,
                achieved,
            ) {
                // We haven't counted current period yet
                (false, true) => {
                    streak_data.amount += streak + 1;
                    streak_data.latest_time_string = current_time_string;
                }
                // We have counted current period, but now user is losing the streak
                (true, false) => {
                    // We have update the streak data previously with success, so we need to revert it
                    streak_data.amount = streak - 1;
                }
                // If both are false, then user hasn't achieved the streak and we don't need to do anything
                // If both are true, then user has already achieved the streak and we don't need to do anything
                _ => {}
            }
            self.user_streaks
                .insert(key, VersionedStreakUserData::V1(streak_data));
        }
    }

    fn verify_previous_streak(
        &self,
        streak: &Streak,
        streak_data: &StreakUserData,
        user: &String,
        timestamp: Timestamp,
    ) -> u32 {
        for i in 0u32..std::cmp::min(5, streak_data.amount) {
            let previous_time = if let Some(a) = streak.time_period.previous_period(timestamp) {
                a
            } else {
                // Shouldn't happen, but if it does, we can't verify the streak
                return i;
            };

            let previous_time_string = streak.time_period.time_string(previous_time);

            let previous_data = self
                .sloths_per_period
                .get(&(user.to_owned(), previous_time_string.clone()))
                .map(|s| streak.is_streak_achieved(s))
                .unwrap_or_default();
            if !previous_data {
                return i;
            }
        }
        // We will check at max 5 previous periods
        streak_data.amount
    }

    pub fn apply_to_periods(
        &mut self,
        author: &String,
        timestamp: Timestamp,
        func: impl Fn(&mut VersionedUserPeriodData),
    ) {
        for period in TimePeriod::iter() {
            if period == TimePeriod::Day {
                continue;
            }

            let key = period.time_string(timestamp);
            let entry = self
                .sloths_per_period
                .entry((author.to_owned(), key.clone()))
                .or_insert(VersionedUserPeriodData::V1(Default::default()));
            func(entry);
        }
        self.calculate_streak(author);
    }

    pub fn get_or_create_account(&mut self, account_id: &str) -> Account {
        self.accounts
            .entry(account_id.to_owned())
            .or_insert(VersionedAccount::V1(Default::default()))
            .clone()
            .into()
    }

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
}
