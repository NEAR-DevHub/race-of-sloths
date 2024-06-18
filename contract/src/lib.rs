#[allow(deprecated)]
use near_sdk::store::UnorderedMap;
use near_sdk::{
    borsh::{BorshDeserialize, BorshSerialize},
    require,
    store::{LookupSet, Vector},
    Timestamp,
};
use near_sdk::{env, near_bindgen, AccountId, PanicOnDefault};
use shared::{
    AccountWithPermanentPercentageBonus, Event, GithubHandle, IntoEnumIterator, PRId, PRWithRating,
    Streak, StreakId, StreakReward, StreakType, StreakUserData, TimePeriod, TimePeriodString,
    VersionedAccount, VersionedPR, VersionedStreak, VersionedStreakUserData,
    VersionedUserPeriodData,
};
use types::{Organization, VersionedOrganization};

pub mod events;
pub mod migrate;
pub mod storage;
#[cfg(test)]
mod tests;
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
            vec![
                StreakReward::FlatReward(10),
                StreakReward::FlatReward(15),
                StreakReward::FlatReward(20),
                StreakReward::FlatReward(25),
                StreakReward::PermanentPercentageBonus(5),
                StreakReward::FlatReward(30),
                StreakReward::FlatReward(35),
                StreakReward::FlatReward(40),
                StreakReward::FlatReward(45),
                StreakReward::PermanentPercentageBonus(10),
                StreakReward::FlatReward(50),
                StreakReward::FlatReward(55),
                StreakReward::FlatReward(60),
                StreakReward::FlatReward(65),
                StreakReward::FlatReward(70),
                StreakReward::FlatReward(75),
                StreakReward::FlatReward(80),
                StreakReward::FlatReward(85),
                StreakReward::FlatReward(90),
                StreakReward::PermanentPercentageBonus(15),
                StreakReward::FlatReward(100),
            ],
        );
        contract.create_streak(
            "Monthly PR with score higher 8".to_owned(),
            TimePeriod::Month,
            vec![StreakType::LargestScore(8)],
            vec![
                StreakReward::FlatReward(10),
                StreakReward::FlatReward(20),
                StreakReward::FlatReward(40),
                StreakReward::FlatReward(60),
                StreakReward::PermanentPercentageBonus(5),
                StreakReward::FlatReward(80),
                StreakReward::FlatReward(100),
                StreakReward::FlatReward(120),
                StreakReward::FlatReward(140),
                StreakReward::PermanentPercentageBonus(10),
                StreakReward::FlatReward(160),
                StreakReward::FlatReward(200),
            ],
        );

        contract
    }

    pub fn create_streak(
        &mut self,
        name: String,
        time_period: TimePeriod,
        streak_criterias: Vec<StreakType>,
        streak_rewards: Vec<StreakReward>,
    ) {
        self.assert_sloth();
        let id = self.streaks.len();
        let streak = Streak::new(id, name, time_period, streak_criterias, streak_rewards);
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

        let pr = PRWithRating::new(organization, repo, pr_number, user, started_at);

        self.apply_to_periods(started_at, &pr, |data| data.pr_opened());
        self.prs.insert(pr_id, VersionedPR::V1(pr));
    }

    pub fn sloth_scored(&mut self, pr_id: String, user: String, score: u32) {
        self.assert_sloth();

        let mut pr: PRWithRating = match self.prs.get(&pr_id).cloned() {
            Some(x) => x.into(),
            None => env::panic_str("PR is not started or already executed"),
        };

        pr.add_score(user, score);
        self.prs.insert(pr_id.clone(), VersionedPR::V1(pr));
    }

    pub fn sloth_merged(&mut self, pr_id: String, merged_at: Timestamp) {
        self.assert_sloth();

        let mut pr: PRWithRating = match self.prs.get(&pr_id).cloned() {
            Some(pr) => pr.into(),
            None => env::panic_str("PR is not started or already executed"),
        };
        pr.add_merge_info(merged_at);

        self.apply_to_periods(merged_at, &pr, |data| data.pr_merged());
        self.prs.insert(pr_id, VersionedPR::V1(pr));
    }

    pub fn sloth_exclude(&mut self, pr_id: String) {
        self.assert_sloth();
        let pr: PRWithRating = match self.prs.get(&pr_id).cloned() {
            Some(pr) => pr.into(),
            None => env::panic_str("PR is not started or already executed"),
        };
        if pr.merged_at.is_some() {
            env::panic_str("Merged PR cannot be excluded")
        }

        self.apply_to_periods(pr.created_at, &pr, |data| {
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

        let pr: PRWithRating = match self.prs.get(&pr_id).cloned() {
            Some(pr) => pr.into(),
            None => env::panic_str("PR is not started or already executed"),
        };
        require!(pr.merged_at.is_none(), "Merged PR cannot be stale");

        self.apply_to_periods(pr.created_at, &pr, |data| data.pr_closed());
        self.prs.remove(&pr_id);
    }

    pub fn sloth_finalize(&mut self, pr_id: String) {
        self.assert_sloth();

        let mut pr: PRWithRating = match self.prs.get(&pr_id).cloned() {
            Some(pr) => pr.into(),
            None => env::panic_str("PR is not started or already executed"),
        };

        if !pr.is_ready_to_move(env::block_timestamp()) {
            env::panic_str("PR is not ready to be finalized")
        }

        let mut user = self.get_or_create_account(&pr.author);

        let score = pr.score().unwrap_or_default();
        let mut bonus_points = 0;
        for streak in self.streaks.iter().filter(|s| s.is_active()).cloned() {
            let streak: Streak = streak.into();
            let streak_data: StreakUserData = self
                .user_streaks
                .get(&(pr.author.to_owned(), streak.id))
                .cloned()
                .unwrap_or_else(|| VersionedStreakUserData::V1(Default::default()))
                .into();

            bonus_points += user.use_flat_bonus(streak.id, streak_data.amount);
        }

        let full_id = pr.pr_id();
        pr.streak_bonus_rating = bonus_points;
        pr.percentage_multiplier = user.lifetime_percentage_bonus();
        let rating = pr.rating();

        self.accounts
            .insert(pr.author.clone(), VersionedAccount::V1(user));
        self.apply_to_periods(pr.merged_at.unwrap(), &pr, |data| {
            data.pr_executed(score, rating)
        });

        self.prs.remove(&full_id);
        self.executed_prs.insert(full_id, VersionedPR::V1(pr));
    }
}

impl Contract {
    pub fn calculate_streak(&mut self, user: &str) {
        let current_time = env::block_timestamp();
        for streak in self.streaks.into_iter().cloned().collect::<Vec<_>>() {
            let streak: Streak = streak.into();

            if !streak.is_active {
                continue;
            }
            let current_time_string = streak.time_period.time_string(current_time);

            // Check if user accomplished the streak for current period
            let achieved = self
                .sloths_per_period
                .get(&(user.to_owned(), current_time_string.clone()))
                .map(|s| streak.is_streak_achieved(s))
                .unwrap_or_default();

            let key = (user.to_owned(), streak.id);
            let mut streak_data: StreakUserData = self
                .user_streaks
                .get(&key)
                .cloned()
                .unwrap_or_else(|| VersionedStreakUserData::V1(Default::default()))
                .into();

            if streak_data.latest_time_string == current_time_string && achieved {
                // Already achieved
                continue;
            }

            let current_streak = streak_data.amount;
            let prev_time_string = streak
                .time_period
                .previous_period(current_time)
                .map(|a| streak.time_period.time_string(a))
                .unwrap_or_default();

            let older_streak = if streak_data.latest_time_string == prev_time_string {
                streak_data.amount
            } else if streak_data.latest_time_string == current_time_string {
                // Lost the streak
                streak_data.amount - 1
            } else {
                0
            };

            streak_data.amount = older_streak + achieved as u32;
            streak_data.best = streak_data.best.max(streak_data.amount);
            streak_data.latest_time_string = if achieved {
                current_time_string
            } else {
                prev_time_string
            };

            if streak_data.amount > current_streak {
                self.reward_streak(user, &streak, streak_data.amount);
                events::log_event(Event::StreakIncreased {
                    streak_id: streak.id,
                    new_streak: streak_data.amount,
                    largest_streak: streak_data.best,
                    name: streak.name.clone(),
                });
            }

            self.user_streaks
                .insert(key, VersionedStreakUserData::V1(streak_data));
        }
    }

    pub fn reward_streak(&mut self, user: &str, streak: &Streak, achieved: u32) {
        let reward = match streak.get_streak_reward(achieved) {
            Some(reward) => reward,
            None => return,
        };

        let mut account = self.get_or_create_account(user);
        match reward {
            StreakReward::FlatReward(amount) => {
                account.add_flat_bonus(streak.id, amount, achieved);
            }
            StreakReward::PermanentPercentageBonus(amount) => {
                account.add_streak_percent(streak.id, amount);
            }
        }
        self.accounts
            .insert(user.to_owned(), VersionedAccount::V1(account));
    }

    pub fn apply_to_periods(
        &mut self,
        timestamp: Timestamp,
        pr: &PRWithRating,
        func: impl Fn(&mut VersionedUserPeriodData),
    ) {
        for period in TimePeriod::iter() {
            if period == TimePeriod::Day {
                continue;
            }

            let key = period.time_string(timestamp);
            let entry = self
                .sloths_per_period
                .entry((pr.author.to_owned(), key.clone()))
                .or_insert(VersionedUserPeriodData::V1(Default::default()));
            func(entry);
        }

        self.calculate_streak(&pr.author);
    }

    pub fn get_or_create_account(
        &mut self,
        account_id: &str,
    ) -> AccountWithPermanentPercentageBonus {
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
