use super::*;

#[near_bindgen]
impl Contract {
    #[init(ignore_state)]
    #[private]
    pub fn migrate() -> Self {
        let mut state: Self = env::state_read().unwrap();

        let users = state.users.len();
        for user_id in 0..users {
            let period_data = state.period_data(user_id, &"102024".to_string());
            if let Some(period_data) = period_data {
                state.sloths_per_period.insert(
                    (user_id, "rosctober2024".to_string()),
                    VersionedUserPeriodData::V2(period_data),
                );
            }
        }

        state
    }
}
