use near_sdk::{
    borsh::{BorshDeserialize, BorshSerialize},
    BorshStorageKey,
};

#[derive(BorshSerialize, BorshDeserialize, BorshStorageKey)]
#[borsh(crate = "near_sdk::borsh")]
pub enum StorageKey {
    Users,
    SlothsPerPeriod,
    ReposNew,
    _RESERVED1,
    _RESERVED2,
    ExcludedPRs,
    Streaks,
    UserStreaks,
    AccountIds,
    _RESERVED3,
    MergedPRs,
    PRs,
}
