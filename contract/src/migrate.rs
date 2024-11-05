use super::*;

#[near_bindgen]
impl Contract {
    #[init(ignore_state)]
    #[private]
    pub fn migrate(list: Vec<String>) -> Self {
        let mut state: Self = env::state_read().unwrap();

        for repo in list {
            let mut split = repo.split('/');
            let owner = split.next().unwrap();
            let name = split.next().unwrap();

            state.repos.remove(&(owner.to_string(), name.to_string()));
        }

        state
    }
}
