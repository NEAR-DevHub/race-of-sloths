use near_sdk::ext_contract;
use shared::{GithubHandle, TimePeriodString, User};

#[ext_contract(external_trait)]
pub trait RaceContract {
    fn users_by_name(&self, users: Vec<GithubHandle>, periods: Vec<TimePeriodString>) -> Vec<User>;
}
