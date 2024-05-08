use near_sdk::{
    near_bindgen,
    serde::{Deserialize, Serialize},
    NearSchema,
};

use super::*;

#[derive(Serialize, Deserialize, NearSchema)]
#[serde(crate = "near_sdk::serde")]
pub struct PRInfo {
    allowed: bool,
    finished: bool,
    exist: bool,
}

#[near_bindgen]
impl Contract {
    pub fn check_info(&self, organization: String, repo: String, issue_id: u64) -> PRInfo {
        let pr_id = format!("{}/{}/{}", organization, repo, issue_id);
        let pr = self.prs.get(&pr_id);
        PRInfo {
            allowed: self
                .organizations
                .get(&organization)
                .map(|org| org.is_allowed(&repo))
                .unwrap_or_default(),
            exist: pr.is_some(),
            finished: pr.map(|pr| pr.accounted).unwrap_or_default(),
        }
    }
}
