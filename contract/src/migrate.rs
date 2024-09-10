use super::*;

#[near_bindgen]
impl Contract {
    #[init(ignore_state)]
    #[private]
    pub fn migrate() -> Self {
        let state: Self = env::state_read().unwrap();

        state
    }
}
