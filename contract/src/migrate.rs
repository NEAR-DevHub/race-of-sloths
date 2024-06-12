use near_sdk::near;

use super::*;

#[near]
impl Contract {
    #[private]
    #[init(ignore_state)]
    pub fn migrate() -> Self {
        let mut state: Self = env::state_read().unwrap();

        state.accounts.clear();

        state
    }
}
