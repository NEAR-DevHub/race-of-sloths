use near_sdk::{test_utils::VMContextBuilder, testing_env, AccountId, VMContext};
use shared::{PRWithRating, SCORE_TIMEOUT_IN_NANOSECONDS};

use super::*;

pub const WEEK_IN_NANOSECONDS: u64 = 7 * SCORE_TIMEOUT_IN_NANOSECONDS;

pub fn github_handle(id: u8) -> GithubHandle {
    format!("name-{id}")
}

pub fn admin() -> AccountId {
    "admin.near".parse().unwrap()
}

pub fn pr_id_str(pr_id: u64) -> String {
    format!("NEAR-DevHub/devbot/{pr_id}")
}

pub struct ContractExt {
    pub contract: Contract,
    pub context: VMContext,
}

impl ContractExt {
    pub fn new() -> Self {
        let mut context = VMContextBuilder::new().build();
        context.predecessor_account_id = admin();
        testing_env!(context.clone());

        let contract = Contract::new(
            admin(),
            vec![AllowedRepos {
                organization: "NEAR-DevHub".to_owned(),
                repos: vec!["devbot".to_owned()],
            }],
        );

        Self { contract, context }
    }

    pub fn include_sloth_common_repo(&mut self, id: u8, pr_id: u64, started_at: u64) {
        self.include_sloth_with_org("NEAR-DevHub", id, pr_id, started_at);
    }

    pub fn include_sloth_with_org(&mut self, org: &str, id: u8, pr_id: u64, started_at: u64) {
        let handle = github_handle(id);
        self.contract.sloth_include(
            org.to_owned(),
            "devbot".to_string(),
            handle,
            pr_id,
            started_at,
            true,
        );
    }

    pub fn score(&mut self, pr_id: u64, id: u8, score: u32) {
        self.contract
            .sloth_scored(pr_id_str(pr_id), github_handle(id), score);
    }

    pub fn merge(&mut self, pr_id: u64, merged_at: u64) {
        self.contract.sloth_merged(pr_id_str(pr_id), merged_at);
    }

    pub fn exclude(&mut self, pr_id: u64) {
        self.contract.sloth_exclude(pr_id_str(pr_id));
    }

    pub fn finalize(&mut self, pr_id: u64) {
        self.contract.sloth_finalize(pr_id_str(pr_id), None, None)
    }
}

#[test]
fn success_streak_flow() {
    let mut contract = ContractExt::new();

    contract.include_sloth_common_repo(0, 0, 0);
    assert_eq!(contract.contract.unmerged_prs(0, 50).len(), 1);

    contract.score(0, 0, 13);

    contract.merge(0, 10);
    assert_eq!(contract.contract.unmerged_prs(0, 50).len(), 0);
    contract.score(0, 1, 8);

    contract.context.block_timestamp = 11;
    testing_env!(contract.context.clone());
    assert_eq!(contract.contract.unfinalized_prs(0, 50).len(), 0);

    contract.context.block_timestamp = SCORE_TIMEOUT_IN_NANOSECONDS + 11;
    testing_env!(contract.context.clone());
    assert_eq!(contract.contract.unfinalized_prs(0, 50).len(), 1);

    contract.finalize(0);

    assert_eq!(contract.contract.unfinalized_prs(0, 50).len(), 0);
    let user = contract
        .contract
        .user(&github_handle(0), vec!["all-time".to_string()])
        .unwrap();

    assert_eq!(user.period_data[0].1.total_score, 10);
    assert_eq!(user.period_data[0].1.total_rating, 120);
    assert_eq!(user.period_data[0].1.executed_prs, 1);
    assert_eq!(user.period_data[0].1.prs_opened, 1);
    assert_eq!(user.period_data[0].1.prs_merged, 1);
    assert_eq!(user.streaks[0].1.amount, 1);
    assert_eq!(user.streaks[1].1.amount, 1);

    // User received 10 points for weekly streak
    let pr = contract
        .contract
        .executed_prs
        .get(&pr_id_str(0))
        .unwrap()
        .clone();
    let pr: PRWithRating = pr.into();

    assert_eq!(pr.score(), Some(10));
    assert_eq!(pr.rating(), 100 + 10 + 10);
    assert_eq!(pr.streak_bonus_rating, 20);

    // New pr with 10 points for monthly streak

    contract.include_sloth_common_repo(0, 1, 0);
    contract.score(1, 1, 10);
    contract.merge(1, 1);
    contract.finalize(1);

    let pr = contract
        .contract
        .executed_prs
        .get(&pr_id_str(1))
        .unwrap()
        .clone();
    let pr: PRWithRating = pr.into();

    assert_eq!(pr.score(), Some(10));
    assert_eq!(pr.rating(), 100);
    assert_eq!(pr.streak_bonus_rating, 0);

    // Total user rating is 120 + 100

    let user = contract
        .contract
        .user(&github_handle(0), vec!["all-time".to_string()])
        .unwrap();

    assert_eq!(user.period_data[0].1.total_rating, 220);
}

#[test]
fn streak_calculation() {
    let mut contract = ContractExt::new();

    let mut current_time = 0;
    for i in 0..12 {
        contract.context.block_timestamp = current_time;
        testing_env!(contract.context.clone());
        contract.include_sloth_common_repo(0, i, current_time);
        contract.score(i, 1, 10);
        contract.merge(i, current_time + 1);
        current_time += SCORE_TIMEOUT_IN_NANOSECONDS * 7 + 1;
        contract.context.block_timestamp = current_time;
        testing_env!(contract.context.clone());
        contract.finalize(i);
    }

    let user = contract
        .contract
        .user(&github_handle(0), vec!["all-time".to_string()])
        .unwrap();
    // 12 weeks streak with opened PR
    assert_eq!(user.streaks[0].1.amount, 12);
    // 3 months streak with 8+ scopre
    assert_eq!(user.streaks[1].1.amount, 3);

    assert_eq!(user.period_data[0].1.total_score, 12 * 10);
    assert_eq!(user.period_data[0].1.executed_prs, 12);
}

#[test]
fn streak_in_a_nutshell() {
    let mut contract = ContractExt::new();

    contract.include_sloth_common_repo(0, 0, 0);
    let user = contract
        .contract
        .user(&github_handle(0), vec!["all-time".to_string()])
        .unwrap();

    assert_eq!(user.streaks[0].1.amount, 1);

    contract.merge(0, 10);
    let user = contract
        .contract
        .user(&github_handle(0), vec!["all-time".to_string()])
        .unwrap();
    assert_eq!(user.streaks[0].1.amount, 1);
    assert_eq!(user.period_data[0].1.total_rating, 0);

    contract.context.block_timestamp = SCORE_TIMEOUT_IN_NANOSECONDS + 11;
    testing_env!(contract.context.clone());
    contract.finalize(0);
    let user = contract
        .contract
        .user(&github_handle(0), vec!["all-time".to_string()])
        .unwrap();
    assert_eq!(user.period_data[0].1.total_score, 1);
    assert_eq!(user.period_data[0].1.total_rating, 10 + 10);
}

#[test]
fn user_had_a_streak_then_lost_then_again_get_it() {
    let mut contract = ContractExt::new();

    contract.include_sloth_common_repo(0, 0, 0);
    let user = contract
        .contract
        .user(&github_handle(0), vec!["all-time".to_string()])
        .unwrap();

    assert_eq!(user.streaks[0].1.amount, 1);
    assert_eq!(user.period_data[0].1.total_rating, 0);

    contract.exclude(0);

    let user = contract
        .contract
        .user(&github_handle(0), vec!["all-time".to_string()])
        .unwrap();

    assert_eq!(user.streaks[0].1.amount, 0);
    assert_eq!(user.streaks[0].1.best, 1);
    assert_eq!(user.period_data[0].1.total_rating, 0);

    contract.include_sloth_common_repo(0, 1, 0);

    let user = contract
        .contract
        .user(&github_handle(0), vec!["all-time".to_string()])
        .unwrap();

    assert_eq!(user.streaks[0].1.amount, 1);
}

#[test]
fn streak_crashed_in_middle() {
    let mut contract = ContractExt::new();

    let mut current_time = 0;
    for i in 0..8 {
        contract.context.block_timestamp = current_time;
        testing_env!(contract.context.clone());
        contract.include_sloth_common_repo(0, i, current_time);
        contract.score(i, 1, 10);
        contract.merge(i, current_time + 1);
        current_time += SCORE_TIMEOUT_IN_NANOSECONDS * 7 + 1;
        contract.context.block_timestamp = current_time;
        testing_env!(contract.context.clone());
        contract.finalize(i);
    }

    let user = contract
        .contract
        .user(&github_handle(0), vec!["all-time".to_string()])
        .unwrap();
    assert_eq!(user.streaks[0].1.amount, 8);
    assert_eq!(user.streaks[1].1.amount, 2);
    assert_eq!(user.period_data[0].1.total_score, 8 * 10);
    assert_eq!(user.period_data[0].1.executed_prs, 8);

    // 5 weeks skipped to crush both streaks
    current_time += SCORE_TIMEOUT_IN_NANOSECONDS * 7 * 5 + 1;

    // we update streak reactively, it means that we will not update streaks until the next PR
    // somehow we need to update streaks if user stops doing something

    for i in 8..12 {
        contract.context.block_timestamp = current_time;
        testing_env!(contract.context.clone());
        contract.include_sloth_common_repo(0, i, current_time);
        contract.score(i, 1, 10);
        contract.merge(i, current_time + 1);
        current_time += SCORE_TIMEOUT_IN_NANOSECONDS * 7 + 1;
        contract.context.block_timestamp = current_time;
        testing_env!(contract.context.clone());
        contract.finalize(i);
    }

    let user = contract
        .contract
        .user(&github_handle(0), vec!["all-time".to_string()])
        .unwrap();
    assert_eq!(user.streaks[0].1.amount, 4);
    assert_eq!(user.streaks[0].1.best, 8);
    assert_eq!(user.streaks[1].1.amount, 1);
    assert_eq!(user.streaks[1].1.best, 2);

    assert_eq!(user.period_data[0].1.total_score, 12 * 10);
    assert_eq!(user.period_data[0].1.executed_prs, 12);
}

#[test]
#[should_panic(expected = "PR is not started or already executed")]
fn exclude_pr() {
    let mut contract = ContractExt::new();

    contract.include_sloth_common_repo(0, 0, 0);

    contract.exclude(0);

    contract.score(0, 1, 13);
}

#[test]
#[should_panic(expected = "Repo is not allowlisted")]
fn notallowlisted_org() {
    let mut contract = ContractExt::new();

    contract.include_sloth_with_org("blahh", 0, 0, 0);

    contract.score(0, 1, 13);
}

#[test]
#[should_panic(expected = "PR is not started or already executed")]
fn not_started_pr() {
    let mut contract = ContractExt::new();

    contract.score(0, 1, 13);
}

#[test]
fn cannot_double_streak_reward() {
    let mut contract = ContractExt::new();

    contract.include_sloth_common_repo(0, 0, 0);
    contract.exclude(0);

    contract.include_sloth_common_repo(0, 1, 0);
    contract.exclude(1);

    contract.include_sloth_common_repo(0, 2, 0);
    contract.score(2, 1, 5);
    contract.merge(2, 1);

    contract.context.block_timestamp = SCORE_TIMEOUT_IN_NANOSECONDS + 2;
    testing_env!(contract.context.clone());
    contract.finalize(2);

    let pr = contract
        .contract
        .executed_prs
        .get(&pr_id_str(2))
        .unwrap()
        .clone();
    let pr: PRWithRating = pr.into();

    assert_eq!(pr.streak_bonus_rating, 10);
    assert_eq!(pr.rating(), 50 + 10);

    let user: AccountWithPermanentPercentageBonus =
        contract.contract.users.get(0).unwrap().clone().into();
    assert_eq!(user.flat_bonus.len(), 0);
}

#[test]
fn cannot_double_percent_reward() {
    let mut contract = ContractExt::new();

    let mut start = 0;
    for i in 0..4 {
        contract.include_sloth_common_repo(0, i, start);
        contract.merge(i, start + 1);

        start += WEEK_IN_NANOSECONDS + 1;
        contract.context.block_timestamp = start;
        testing_env!(contract.context.clone());
        contract.finalize(i);
    }

    let user: AccountWithPermanentPercentageBonus =
        contract.contract.users.get(0).unwrap().clone().into();

    assert_eq!(user.lifetime_percentage_bonus(), 0);

    contract.include_sloth_common_repo(0, 5, start);
    contract.score(5, 1, 5);
    contract.merge(5, start + 1);

    contract.context.block_timestamp = start + WEEK_IN_NANOSECONDS;
    testing_env!(contract.context.clone());
    contract.finalize(5);

    let user: AccountWithPermanentPercentageBonus =
        contract.contract.users.get(0).unwrap().clone().into();

    assert_eq!(user.permanent_percentage_bonus.len(), 1);
    assert_eq!(user.lifetime_percentage_bonus(), 5);

    let pr = contract
        .contract
        .executed_prs
        .get(&pr_id_str(5))
        .unwrap()
        .clone();

    let pr: PRWithRating = pr.into();

    assert_eq!(pr.rating(), 53);

    let total_user = contract
        .contract
        .user(&github_handle(0), vec!["all-time".to_string()])
        .unwrap();

    let weekly_streaks = 10 + 15 + 20 + 25;
    let autoscore = 10 + 10 + 10 + 10;
    let total_rating = weekly_streaks + autoscore + 53;

    assert_eq!(total_user.period_data[0].1.total_rating, total_rating)
}

#[test]
fn monthly_streak_awarded_imediatly() {
    let mut contract = ContractExt::new();

    contract.include_sloth_common_repo(0, 0, 0);
    contract.score(0, 1, 8);
    contract.merge(0, 1);

    contract.context.block_timestamp = SCORE_TIMEOUT_IN_NANOSECONDS + 2;
    testing_env!(contract.context.clone());
    contract.finalize(0);

    let pr = contract
        .contract
        .executed_prs
        .get(&pr_id_str(0))
        .unwrap()
        .clone();

    let pr: PRWithRating = pr.into();

    assert_eq!(pr.streak_bonus_rating, 10 + 10);
}

#[test]
fn non_scored_pr_gets_autoscored() {
    let mut contract = ContractExt::new();

    contract.include_sloth_common_repo(0, 0, 0);
    contract.merge(0, 1);

    contract.context.block_timestamp = SCORE_TIMEOUT_IN_NANOSECONDS + 2;
    testing_env!(contract.context.clone());
    contract.finalize(0);

    let pr = contract
        .contract
        .executed_prs
        .get(&pr_id_str(0))
        .unwrap()
        .clone();

    let pr: PRWithRating = pr.into();

    assert_eq!(pr.score(), Some(1));
}

#[test]
fn active_non_scored_pr_gets_autoscored() {
    let mut contract = ContractExt::new();

    contract.include_sloth_common_repo(0, 0, 0);
    contract.merge(0, 1);

    contract.context.block_timestamp = SCORE_TIMEOUT_IN_NANOSECONDS + 2;
    testing_env!(contract.context.clone());
    contract
        .contract
        .sloth_finalize(pr_id_str(0), Some((true, "admin".to_string())), None);

    let pr = contract
        .contract
        .executed_prs
        .get(&pr_id_str(0))
        .unwrap()
        .clone();

    let pr: PRWithRating = pr.into();

    assert_eq!(pr.score(), Some(2));
}
