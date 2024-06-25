use near_sdk::{
    serde::{Deserialize, Serialize},
    NearSchema,
};

use super::*;

use shared::SCORE_TIMEOUT_IN_NANOSECONDS;

const WEEK_TIMEOUT_IN_NANOSECONDS: u64 = SCORE_TIMEOUT_IN_NANOSECONDS * 7;

#[near_bindgen]
impl Contract {
    #[init(ignore_state)]
    pub fn new_mocked(
        sloth: AccountId,
        allowed_repos: Vec<AllowedRepos>,
        mocked_data: Vec<MockUser>,
    ) -> Self {
        let mut contract = Self::new(sloth, allowed_repos);

        let current_timestamp = env::block_timestamp() - 2 * SCORE_TIMEOUT_IN_NANOSECONDS;
        let mut counter = 0;
        for user in mocked_data {
            let (user_id, name) = contract.get_or_create_account(&user.username);

            for i in (0..user.weekly_prs).rev() {
                let pr_id = format!("race-of-sloths/mock/{}", counter);
                contract.mock_pr_full_cycle(
                    user_id,
                    name.github_handle.clone(),
                    counter,
                    &pr_id,
                    current_timestamp - WEEK_TIMEOUT_IN_NANOSECONDS * i,
                );

                counter += 1;
            }
        }

        contract
    }
}

impl Contract {
    fn mock_pr_full_cycle(
        &mut self,
        user_id: UserId,
        name: GithubHandle,
        pr_number: u64,
        pr_id: &str,
        timestamp: Timestamp,
    ) {
        let pr_with_rating = PRWithRating::new(
            "race-of-sloths".to_owned(),
            "mock".to_owned(),
            pr_number,
            name.to_string(),
            timestamp,
        );
        // Simulate PR opening
        self.prs
            .insert(pr_id.to_string(), VersionedPR::V1(pr_with_rating));
        self.apply_to_periods(timestamp, user_id, |data: &mut VersionedUserPeriodData| {
            data.pr_opened()
        });

        // Simulate scoring
        let score = 10; // Example fixed score
        self.sloth_scored(pr_id.to_string(), "reviewer".to_string(), score);

        // Simulate merging
        let merged_at = timestamp + 1000000000; // Example: Merge 1,000 seconds later
        self.sloth_merged(pr_id.to_string(), merged_at);

        self.sloth_finalize(
            pr_id.to_string(),
            Some(merged_at + SCORE_TIMEOUT_IN_NANOSECONDS + 1),
        );
    }
}

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, NearSchema)]
#[serde(crate = "near_sdk::serde")]
#[borsh(crate = "near_sdk::borsh")]
pub struct MockUser {
    username: String,
    weekly_prs: u64,
}
