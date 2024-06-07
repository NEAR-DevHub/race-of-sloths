use chrono::{DateTime, Datelike, Days, Months};
use near_sdk::Timestamp;
use strum::EnumIter;

use super::*;

pub use strum::IntoEnumIterator;

pub type TimePeriodString = String;

#[derive(
    BorshDeserialize,
    BorshSerialize,
    Serialize,
    Deserialize,
    NearSchema,
    Debug,
    PartialEq,
    Clone,
    Copy,
    EnumIter,
)]
#[serde(crate = "near_sdk::serde")]
#[borsh(crate = "near_sdk::borsh")]
pub enum TimePeriod {
    Day,
    Week,
    Month,
    Quarter,
    Year,
    AllTime,
}

impl TimePeriod {
    pub fn from_time_period_string(period: &str) -> Option<Self> {
        match period {
            a if a.contains('W') => Some(TimePeriod::Week),
            a if a.contains('Q') => Some(TimePeriod::Quarter),
            a if a.len() == 8 => Some(TimePeriod::Day),
            a if a.len() == 6 => Some(TimePeriod::Month),
            "all-time" => Some(TimePeriod::AllTime),
            _ => None,
        }
    }

    pub fn time_string(&self, timestamp: Timestamp) -> TimePeriodString {
        match self {
            TimePeriod::Day => timestamp_to_day_string(timestamp),
            TimePeriod::Week => timestamp_to_week_string(timestamp),
            TimePeriod::Month => timestamp_to_month_string(timestamp),
            TimePeriod::Quarter => timestamp_to_quarter_string(timestamp),
            TimePeriod::Year => DateTime::from_timestamp_nanos(timestamp as i64)
                .year()
                .to_string(),
            TimePeriod::AllTime => "all-time".to_string(),
        }
    }

    pub fn previous_period(&self, timestamp: Timestamp) -> Option<Timestamp> {
        let timestamp = DateTime::from_timestamp_nanos(timestamp as i64);
        let result = match self {
            TimePeriod::Day => timestamp
                .checked_sub_days(Days::new(1))?
                .timestamp_nanos_opt()? as Timestamp,
            TimePeriod::Week => timestamp
                .checked_sub_days(Days::new(7))?
                .timestamp_nanos_opt()? as Timestamp,
            TimePeriod::Month => timestamp
                .checked_sub_months(Months::new(1))?
                .timestamp_nanos_opt()? as Timestamp,
            TimePeriod::Quarter => timestamp
                .checked_sub_months(Months::new(3))?
                .timestamp_nanos_opt()? as Timestamp,
            TimePeriod::Year => timestamp
                .checked_sub_months(Months::new(12))?
                .timestamp_nanos_opt()? as Timestamp,
            TimePeriod::AllTime => return None,
        };

        Some(result)
    }
}

// Helper function to convert timestamp to quarter string
fn timestamp_to_day_string(timestamp: Timestamp) -> TimePeriodString {
    let date = DateTime::from_timestamp_nanos(timestamp as i64);
    format!("{:02}{:02}{:04}", date.day(), date.month(), date.year())
}

fn timestamp_to_week_string(timestamp: Timestamp) -> TimePeriodString {
    let date = DateTime::from_timestamp_nanos(timestamp as i64);
    format!("{}W{}", date.year(), date.iso_week().week())
}

fn timestamp_to_quarter_string(timestamp: Timestamp) -> TimePeriodString {
    let datetime = DateTime::from_timestamp_nanos(timestamp as i64);
    let quarter = datetime.month0() / 3 + 1;
    format!("{}Q{}", datetime.year(), quarter)
}

fn timestamp_to_month_string(timestamp: u64) -> TimePeriodString {
    let date = DateTime::from_timestamp_nanos(timestamp as i64);
    format!("{:02}{:04}", date.month(), date.year())
}
