use super::*;

#[near_bindgen]
impl Contract {
    #[private]
    #[init(ignore_state)]
    pub fn migrate() -> Self {
        let mut state: Self = env::state_read().unwrap();
        // Maximally stupid migration: clear all data, instead of migrating it.
        state.streaks.clear();
        state
    }
}
