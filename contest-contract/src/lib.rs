use near_sdk::{
    borsh::{BorshDeserialize, BorshSerialize},
    env, near_bindgen, AccountId, Gas, PanicOnDefault, Promise, PromiseError,
};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaChaRng;
use shared::{GithubHandle, TimePeriod, TimePeriodString, User};

pub mod ext;

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
#[borsh(crate = "near_sdk::borsh")]
pub struct Contract {
    pub account: AccountId,
}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new(account: AccountId) -> Self {
        Self { account }
    }

    #[private]
    pub fn contest_results(
        &mut self,
        participants: Vec<GithubHandle>,
        winners: usize,
        time_period: Option<TimePeriodString>,
    ) -> Promise {
        let time_period =
            time_period.unwrap_or_else(|| TimePeriod::Month.time_string(env::block_timestamp()));

        if participants.len() < winners {
            env::panic_str("Number of winners is greater than the number of participants");
        }

        let promise = ext::external_trait::ext(self.account.clone())
            .users_by_name(participants, vec![time_period]);

        promise.then(
            Self::ext(env::current_account_id())
                .with_static_gas(Gas::from_tgas(20))
                .contest_results_private(winners),
        )
    }

    #[private]
    pub fn contest_results_private(
        &mut self,
        winners: usize,
        #[callback_result] users: Result<Vec<User>, PromiseError>,
    ) {
        let mut users = match users {
            Ok(users) => users,
            Err(e) => {
                env::panic_str(&format!("Failed to receive users list: {:?}", e));
            }
        };
        env::log_str(&format!("Received {} users", users.len()));

        // We will make random choice with chance depending on the user's rating
        // The higher the rating, the higher the chance to win

        let mut total_rating: u32 = users
            .iter()
            .map(|u| u.period_data.first().unwrap().1.total_rating)
            .sum();
        let rand = env::random_seed_array();
        let mut rand = ChaChaRng::from_seed(rand);

        for winner_index in 0..winners {
            let mut rating = rand.gen_range(0..total_rating);
            let winner = users.iter().enumerate().find_map(|(i, user)| {
                let user_rating = user.period_data.first().unwrap().1.total_rating;
                if rating < user_rating {
                    Some((i, user.clone()))
                } else {
                    rating -= user_rating;
                    None
                }
            });
            match winner {
                Some((i, user)) => {
                    total_rating -= user.period_data.first().unwrap().1.total_rating;
                    users.remove(i);
                    env::log_str(&format!("The {} winner is {}", winner_index + 1, user.name));
                }
                None => {
                    env::panic_str("Failed to select winner");
                }
            };
        }
    }
}
