use super::*;

#[near_bindgen]
impl Contract {
    #[init(ignore_state)]
    pub fn migrate() -> Self {
        env::state_read::<Contract>().expect("Old state doesn't exist")
    }
}
