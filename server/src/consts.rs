// Success messages
pub const FINALIZE_MESSAGE: &str = "🎉 Hooray! The PR has been finalized. Thank you for your epic contribution! The scoring process is now officially closed. 🏁✨";
pub const MERGE_MESSAGE: &str = "🚀 Woohoo! The PR has been merged, but it wasn't scored. The scoring process will close automatically in 24 hours! ⏳🕒";
pub const STALE_MESSAGE: &str = "🕰️ Uh-oh! The PR has been inactive for two weeks. Marking it as stale. To continue, please restart the bot with the `include` command. ⏮️";
pub const SCORE_MESSAGE: &str =
    "🏆 Awesome! Thanks for submitting your score for the Race of Sloths! 🦥🔥";
pub const PAUSE_MESSAGE: &str = "⏸️ Time out! We've paused this repository. We won't participate in new PRs, but already scored PRs will be accepted after the merge. 🛠️";
pub const UNPAUSE_MESSAGE: &str = "▶️ And we're back! We've unpaused this repository. Please start us again to include us in the PRs. 🔄";
pub const EXCLUDE_MESSAGE: &str = "❌ Oh no! The PR has been excluded. If you want to include it again, please restart the bot with the `include` command. 🆕";

// Score related error messages
pub const SCORE_INVALID_SCORE: &str =
    "⚠️ Oops! Score should be a Fibonacci number: 1, 2, 3, 5, 8, or 13. 📊";
pub const SCORE_SELF_SCORE: &str = "🚫 No self-scoring allowed! Nice try though. 😉";

// Pause related error messages
pub const PAUSE_ALREADY_UNPAUSED: &str = "ℹ️ Heads up! The repository is already unpaused. 📣";

// Common error messages
pub const MAINTAINER_ONLY: &str = "👮‍♂️ Hold up! Only maintainers can call this command. Please, ask them nicely, and maybe they'll run it. 🤞";
pub const UNKNOWN_COMMAND: &str =
    "❓ Hmmm, unknown command. Please check the command and try again. 🕵️‍♂️";
