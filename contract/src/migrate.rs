use super::*;

#[near_bindgen]
impl Contract {
    #[init(ignore_state)]
    #[private]
    pub fn migrate() -> Self {
        let mut state: Contract = env::state_read().unwrap();
        let current_timestamp = env::block_timestamp();

        for user_id in 0..state.users.len() {
            for streak_id in 0..2 {
                let streak: Streak = state.streaks[streak_id].clone().into();
                let Some(mut streak_data): Option<StreakUserData> = state
                    .user_streaks
                    .get(&(user_id, streak_id))
                    .map(|streak| streak.clone().into())
                else {
                    continue;
                };

                let mut time = streak
                    .time_period
                    .previous_period(current_timestamp)
                    .unwrap();
                let mut i = 0;
                let recalculated_streak = loop {
                    let period = streak.time_period.time_string(time);
                    let achieved = check_period_data(streak_id, user_id, period, &state);

                    if achieved {
                        i += 1;
                        time = if let Some(time) = streak.time_period.previous_period(time) {
                            time
                        } else {
                            break i;
                        };
                    } else {
                        break i;
                    }
                };

                let current_period = streak.time_period.time_string(current_timestamp);
                let is_current_period_achieved =
                    check_period_data(streak_id, user_id, current_period.clone(), &state);
                let recalculated_streak = if is_current_period_achieved {
                    recalculated_streak + 1
                } else {
                    recalculated_streak
                };

                if recalculated_streak > streak_data.amount {
                    streak_data.amount = recalculated_streak;
                    streak_data.best = recalculated_streak.max(streak_data.best);
                    streak_data.latest_time_string = if is_current_period_achieved {
                        current_period
                    } else {
                        streak_data.latest_time_string
                    };
                    state.user_streaks.insert(
                        (user_id, streak_id),
                        VersionedStreakUserData::V1(streak_data),
                    );

                    state.reward_streak(user_id, &streak, recalculated_streak);
                }
            }
        }
        state
    }
}

fn check_period_data(streak_id: u32, user_id: u32, period: String, state: &Contract) -> bool {
    let period_data = state.sloths_per_period.get(&(user_id, period)).cloned();
    if let Some(period_data) = period_data {
        let streak: Streak = state.streaks[streak_id].clone().into();
        return streak.is_streak_achieved(&period_data);
    }
    false
}
