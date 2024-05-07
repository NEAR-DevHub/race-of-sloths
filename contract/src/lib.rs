use near_sdk::borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::store::{UnorderedMap, Vector};
use near_sdk::{env, near_bindgen, AccountId, PanicOnDefault};
use types::UserData;

pub mod storage;
pub mod types;
pub mod views;

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
#[borsh(crate = "near_sdk::borsh")]
pub struct Contract {
    sloth: AccountId,
    racer_to_index: UnorderedMap<String, u32>,
    racers: Vector<UserData>,
}

#[near_bindgen]
impl Contract {
    pub fn new(sloth: AccountId) -> Contract {
        Self {
            sloth,
            racer_to_index: UnorderedMap::new(storage::StorageKey::RacerToIndex),
            racers: Vector::new(storage::StorageKey::Racers),
        }
    }

    pub fn pr_merged(&mut self, handle: String, score: u64) {
        self.assert_sloth();

        let account_id = self.racer_to_index.get(&handle).copied();
        if let Some(index) = account_id {
            let user_data = self.racers.get_mut(index).expect("User data not found");
            user_data.add_score(score);
        } else {
            let mut user_data = UserData::new(handle.clone());
            user_data.add_score(score);
            self.racer_to_index.insert(handle, self.racers.len());
            self.racers.push(user_data);
        }
    }
}

impl Contract {
    pub fn assert_sloth(&self) {
        if env::predecessor_account_id() != self.sloth {
            env::panic_str("Only sloth can call this method")
        }
    }
}
