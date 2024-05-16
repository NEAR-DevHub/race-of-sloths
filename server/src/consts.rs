// Success messages
pub const FINALIZE_MESSAGES: [&str; 3] = [
    "🎉 Hooray! The PR has been finalized. Thank you for your epic contribution! The scoring process is now officially closed. 🏁✨",
    "✅ Great job! The PR is finalized. Your contribution is much appreciated. Scoring is now wrapped up! 🎊",
    "🎊 Woohoo! The PR has been completed. Thanks for your fantastic contribution! The scoring process is now done. 🏆"
];

pub const MERGE_MESSAGES: [&str; 3] = [
    "🚀 Woohoo! The PR has been merged, but it wasn't scored. The scoring process will close automatically in 24 hours! ⏳🕒",
    "🔄 The PR has been merged. Heads up, it wasn't scored. Scoring will close in 24 hours! 🕰️",
    "⚡ The PR is merged! Note: it wasn't scored. The scoring process will end in 24 hours. 🕛"
];

pub const STALE_MESSAGES: [&str; 3] = [
    "🕰️ Uh-oh! The PR has been inactive for two weeks. Marking it as stale. To continue, please restart the bot with the `include` command. ⏮️",
    "⏳ This PR has been inactive for two weeks. It's now marked as stale. Restart the bot with `include` to proceed. 🔄",
    "📅 Two weeks of inactivity! This PR is now stale. Use the `include` command to restart the bot. 🆙"
];

pub const SCORE_MESSAGES: [&str; 3] = [
    "🏆 Awesome! Thanks for submitting your score for the Race of Sloths! 🦥🔥",
    "🥇 Thanks for your score submission in the Race of Sloths! You're helping make this exciting! 🎉",
    "🎖️ Thanks for adding your score to the Race of Sloths! Keep up the great work! 🏅"
];

pub const PAUSE_MESSAGES: [&str; 3] = [
    "⏸️ Time out! We've paused this repository. We won't participate in new PRs, but already scored PRs will be accepted after the merge. 🛠️",
    "🚫 Repository paused. No new PR participation, but scored PRs will be accepted post-merge. 🔨",
    "⏹️ Hold up! We've paused this repo. New PRs are on hold, but scored PRs will be merged. 🔧"
];

pub const UNPAUSE_MESSAGES: [&str; 3] = [
    "▶️ And we're back! We've unpaused this repository. Please start us again to include us in the PRs. 🔄",
    "🔔 The repository is unpaused! Start us again to include us in your PRs. 📢",
    "🟢 We're live again! The repo is unpaused. Include us in your PRs by starting us up. 🏃‍♂️"
];

pub const EXCLUDE_MESSAGES: [&str; 3] = [
    "❌ Oh no! The PR has been excluded. If you want to include it again, please restart the bot with the `include` command. 🆕",
    "🚫 This PR has been excluded. To include it again, restart the bot with the `include` command. 🔄",
    "🛑 PR excluded. To bring it back, restart the bot with the `include` command. 📲"
];

// Score related error messages
pub const SCORE_INVALID_SCORES: [&str; 3] = [
    "⚠️ Oops! Score should be a Fibonacci number: 1, 2, 3, 5, 8, or 13. 📊",
    "🚨 Invalid score! Please use a Fibonacci number: 1, 2, 3, 5, 8, or 13. 🔢",
    "❗ Score error! Only Fibonacci numbers are accepted: 1, 2, 3, 5, 8, or 13. ➕",
];

pub const SCORE_SELF_SCORES: [&str; 3] = [
    "🚫 No self-scoring allowed! Nice try though. 😉",
    "❌ Self-scoring is not permitted. Let's keep it fair! 👍",
    "🔒 You can't score your own PR. Thanks for understanding! 🙏",
];

// Pause related error messages
pub const PAUSE_ALREADY_UNPAUSED_MESSAGES: [&str; 3] = [
    "ℹ️ Heads up! The repository is already unpaused. 📣",
    "🔄 The repo is already unpaused. You're good to go! 💪",
    "📢 Note: The repository is already unpaused. Carry on! ✅",
];

// Include related error messages
pub const INCLUDE_ALREADY_MERGED_MESSAGES: [&str; 3] = [
    "⚠️ Oops! The PR is already merged. It's too late to include us now. Better luck next time! 🚀",
    "🔒 Oh no! This PR is already merged. We're too late to join the party. Maybe next time! 🎉",
    "🛑 Whoops! The PR is already merged. Looks like we missed the boat. Catch you on the next one! ⏭️"
];

// Common error messages
pub const MAINTAINER_ONLY_MESSAGES: [&str; 3] = [
    "👮‍♂️ Hold up! Only maintainers can call this command. Please, ask them nicely, and maybe they'll run it. 🤞",
    "🚫 Access denied! Only maintainers can use this command. Try asking them nicely! 🙏",
    "🔐 This command is for maintainers only. A polite request might get it run for you. 🙂"
];

pub const UNKNOWN_COMMAND_MESSAGES: [&str; 3] = [
    "❓ Hmmm, unknown command. Please check the command and try again. 🕵️‍♂️",
    "🤔 Unknown command detected. Double-check and try again! 🛠️",
    "❗ Command not recognized. Please verify and give it another shot. 🔄",
];
