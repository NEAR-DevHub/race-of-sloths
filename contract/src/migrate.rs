use super::*;

#[near_bindgen]
impl Contract {
    #[init(ignore_state)]
    #[private]
    pub fn migrate() -> Self {
        let mut state: Contract = env::state_read().unwrap();

        let prs = state
            .prs
            .values()
            .cloned()
            .map(Into::<PRv2>::into)
            .collect::<Vec<_>>();

        for pr in prs {
            let user_id = *state.account_ids.get(&pr.author).unwrap();

            state.apply_to_periods(pr.included_at, user_id, |data| {
                if let Some(score) = pr.score() {
                    data.pr_scored(0, score);
                }
            })
        }

        state
    }
}
