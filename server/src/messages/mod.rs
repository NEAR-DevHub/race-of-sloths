use crate::api::near::PRInfo;

impl PRInfo {
    pub fn status_message(&self) -> String {
        let mut message = String::from("### 🏆 Race of Sloths Status Update 🏆\n\n");

        if self.excluded {
            message.push_str("Hey there! 🚫 Your PR has been excluded from the Race of Sloths. If you think this is a mistake, please reach out to the maintainers. 🙏\n\n");
            return message;
        }

        message.push_str(
            "Hey there! 🎉 Your PR is now part of the Race of Sloths. Thanks for contributing! 🙌\n\n",
        );

        message.push_str("**Current Status:**\n\n");

        if !self.votes.is_empty() {
            message.push_str("- **Scoring:**\n");
            for vote in self.votes.iter() {
                message.push_str(&format!("  - {}: {}\n", vote.user, vote.score));
            }
        } else {
            message.push_str(
                "- **Scoring:** No one has scored your PR yet. Maintainers can score using `@race-of-sloths score [1,2,3,5,8,13]`.\n",
            );
        }

        if self.merged {
            message.push_str("- **Merge Status:** Your PR has been successfully merged! 🎉\n");
        } else {
            message.push_str("- **Merge Status:** Your PR hasn't been tracked as merged yet. Hang tight, it might take a bit of time! ⏳\n");
        }

        if self.executed {
            message.push_str("- **Execution Status:** The PR has been executed. Great job! 🚀\n");
        } else {
            message.push_str(
                "- **Execution Status:** The PR hasn't been executed yet. Stay tuned! 👀\n",
            );
        }

        message.push_str("\nWe'll keep this status updated as things progress. Thanks again for your awesome contribution! 🌟");

        message
    }
}
