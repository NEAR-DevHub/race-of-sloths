use near_sdk::{
    borsh::{BorshDeserialize, BorshSerialize},
    BorshStorageKey,
};

#[derive(BorshSerialize, BorshDeserialize, BorshStorageKey)]
#[borsh(crate = "near_sdk::borsh")]
pub enum StorageKey {
    Users,
    SlothsPerPeriod,
    _Reserved,
    PRs,
    MergedPRs,
    ExcludedPRs,
    Streaks,
    UserStreaks,
    AccountIds,
    Repos,
}
