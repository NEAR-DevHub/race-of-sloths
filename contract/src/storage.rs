use near_sdk::{
    borsh::{BorshDeserialize, BorshSerialize},
    BorshStorageKey,
};

#[derive(BorshSerialize, BorshDeserialize, BorshStorageKey)]
#[borsh(crate = "near_sdk::borsh")]
pub enum StorageKey {
    // Unused
    _1,
    // Unused
    _2,
    Organizations,
    PRs,
    MergedPRs,
    ExcludedPRs,
    Streaks,
    UserStreaks,
    Accounts,
    SlothsPerPeriod,
}
