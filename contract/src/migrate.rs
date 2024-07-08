use near_sdk::near_bindgen;

use super::*;

#[near_bindgen]
impl Contract {
    #[init(ignore_state)]
    #[private]
    pub fn migrate(allowed_repos: Vec<AllowedRepos>) -> Self {
        let mut contract: Contract = near_sdk::env::state_read().unwrap();

        contract.organizations.clear();
        for org in allowed_repos {
            for repo in org.repos {
                contract.include_repo(org.organization.clone(), repo)
            }
        }

        contract
    }
}
