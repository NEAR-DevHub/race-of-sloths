use chrono::{DateTime, Datelike, Days, Months, NaiveDate, TimeZone, Utc};
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
    Eq,
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
    pub fn from_streak_type(type_: &str) -> Option<Self> {
        match type_.to_lowercase().as_str() {
            "daily" => Some(Self::Day),
            "weekly" => Some(Self::Week),
            "monthly" => Some(Self::Month),
            "quarterly" => Some(Self::Quarter),
            "yearly" => Some(Self::Year),
            "all-time" => Some(Self::AllTime),
            _ => None,
        }
    }

    pub fn time_string(&self, timestamp: Timestamp) -> TimePeriodString {
        match self {
            Self::Day => timestamp_to_day_string(timestamp),
            Self::Week => timestamp_to_week_string(timestamp),
            Self::Month => timestamp_to_month_string(timestamp),
            Self::Quarter => timestamp_to_quarter_string(timestamp),
            Self::Year => DateTime::from_timestamp_nanos(timestamp as i64)
                .year()
                .to_string(),
            Self::AllTime => "all-time".to_string(),
        }
    }

    pub fn previous_period(&self, timestamp: Timestamp) -> Option<Timestamp> {
        let timestamp = DateTime::from_timestamp_nanos(timestamp as i64);
        let result = match self {
            Self::Day => timestamp
                .checked_sub_days(Days::new(1))?
                .timestamp_nanos_opt()? as Timestamp,
            Self::Week => timestamp
                .checked_sub_days(Days::new(7))?
                .timestamp_nanos_opt()? as Timestamp,
            Self::Month => timestamp
                .checked_sub_months(Months::new(1))?
                .timestamp_nanos_opt()? as Timestamp,
            Self::Quarter => timestamp
                .checked_sub_months(Months::new(3))?
                .timestamp_nanos_opt()? as Timestamp,
            Self::Year => timestamp
                .checked_sub_months(Months::new(12))?
                .timestamp_nanos_opt()? as Timestamp,
            Self::AllTime => return None,
        };

        Some(result)
    }

    pub fn end_period(&self, timestamp: Timestamp) -> Option<chrono::DateTime<Utc>> {
        let date_time = DateTime::<Utc>::from_timestamp_nanos(timestamp as i64).date_naive();

        match self {
            Self::Day => (date_time + chrono::Duration::days(1))
                .and_hms_opt(0, 0, 0)
                .map(|d| d.and_utc()),
            Self::Week => {
                let iso_week = date_time.iso_week();
                NaiveDate::from_isoywd_opt(
                    iso_week.year(),
                    iso_week.week() + 1,
                    chrono::Weekday::Mon,
                )
                .and_then(|d| d.and_hms_opt(0, 0, 0).map(|d| d.and_utc()))
            }
            Self::Month => if date_time.month() == 12 {
                Utc.with_ymd_and_hms(date_time.year() + 1, 1, 1, 0, 0, 0)
            } else {
                Utc.with_ymd_and_hms(date_time.year(), date_time.month() + 1, 1, 0, 0, 0)
            }
            .earliest(),
            Self::Quarter => {
                let current_quarter = (date_time.month() - 1) / 3 + 1;
                if current_quarter == 4 {
                    Utc.with_ymd_and_hms(date_time.year() + 1, 1, 1, 0, 0, 0)
                } else {
                    Utc.with_ymd_and_hms(date_time.year(), current_quarter * 3 + 1, 1, 0, 0, 0)
                }
                .earliest()
            }
            Self::Year => Utc
                .with_ymd_and_hms(date_time.year() + 1, 1, 1, 0, 0, 0)
                .earliest(),
            Self::AllTime => None,
        }
    }

    pub fn start_period(&self, timestamp: Timestamp) -> Option<chrono::DateTime<Utc>> {
        let date_time = DateTime::<Utc>::from_timestamp_nanos(timestamp as i64).date_naive();

        match self {
            Self::Day => date_time.and_hms_opt(0, 0, 0).map(|d| d.and_utc()),
            Self::Week => {
                let iso_week = date_time.iso_week();
                NaiveDate::from_isoywd_opt(iso_week.year(), iso_week.week(), chrono::Weekday::Mon)
                    .and_then(|d| d.and_hms_opt(0, 0, 0).map(|d| d.and_utc()))
            }
            Self::Month => Utc
                .with_ymd_and_hms(date_time.year(), date_time.month(), 1, 0, 0, 0)
                .earliest(),
            Self::Quarter => {
                let current_quarter = (date_time.month() - 1) / 3 + 1;
                Utc.with_ymd_and_hms(date_time.year(), current_quarter * 3 - 2, 1, 0, 0, 0)
                    .earliest()
            }
            Self::Year => Utc
                .with_ymd_and_hms(date_time.year(), 1, 1, 0, 0, 0)
                .earliest(),
            Self::AllTime => None,
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    #[test]
    fn test_from_streak_type() {
        assert_eq!(TimePeriod::from_streak_type("daily"), Some(TimePeriod::Day));
        assert_eq!(
            TimePeriod::from_streak_type("weekly"),
            Some(TimePeriod::Week)
        );
        assert_eq!(
            TimePeriod::from_streak_type("monthly"),
            Some(TimePeriod::Month)
        );
        assert_eq!(
            TimePeriod::from_streak_type("quarterly"),
            Some(TimePeriod::Quarter)
        );
        assert_eq!(
            TimePeriod::from_streak_type("yearly"),
            Some(TimePeriod::Year)
        );
        assert_eq!(
            TimePeriod::from_streak_type("all-time"),
            Some(TimePeriod::AllTime)
        );
        assert_eq!(TimePeriod::from_streak_type("unknown"), None);
    }

    #[test]
    fn test_time_string() {
        let timestamp = Utc
            .with_ymd_and_hms(2023, 6, 20, 0, 0, 0)
            .unwrap()
            .timestamp_nanos_opt()
            .unwrap() as Timestamp;

        assert_eq!(TimePeriod::Day.time_string(timestamp), "20062023");
        assert_eq!(TimePeriod::Week.time_string(timestamp), "2023W25");
        assert_eq!(TimePeriod::Month.time_string(timestamp), "062023");
        assert_eq!(TimePeriod::Quarter.time_string(timestamp), "2023Q2");
        assert_eq!(TimePeriod::Year.time_string(timestamp), "2023");
        assert_eq!(
            TimePeriod::AllTime.time_string(timestamp),
            "all-time".to_string()
        );
    }

    #[test]
    fn test_previous_period() {
        let timestamp = Utc
            .with_ymd_and_hms(2023, 6, 20, 0, 0, 0)
            .unwrap()
            .timestamp_nanos_opt()
            .unwrap() as Timestamp;

        let previous_day = Utc
            .with_ymd_and_hms(2023, 6, 19, 0, 0, 0)
            .unwrap()
            .timestamp_nanos_opt()
            .unwrap() as Timestamp;
        assert_eq!(
            TimePeriod::Day.previous_period(timestamp),
            Some(previous_day)
        );

        let previous_week = Utc
            .with_ymd_and_hms(2023, 6, 13, 0, 0, 0)
            .unwrap()
            .timestamp_nanos_opt()
            .unwrap() as Timestamp;
        assert_eq!(
            TimePeriod::Week.previous_period(timestamp),
            Some(previous_week)
        );

        let previous_month = Utc
            .with_ymd_and_hms(2023, 5, 20, 0, 0, 0)
            .unwrap()
            .timestamp_nanos_opt()
            .unwrap() as Timestamp;
        assert_eq!(
            TimePeriod::Month.previous_period(timestamp),
            Some(previous_month)
        );

        let previous_quarter = Utc
            .with_ymd_and_hms(2023, 3, 20, 0, 0, 0)
            .unwrap()
            .timestamp_nanos_opt()
            .unwrap() as Timestamp;
        assert_eq!(
            TimePeriod::Quarter.previous_period(timestamp),
            Some(previous_quarter)
        );

        let previous_year = Utc
            .with_ymd_and_hms(2022, 6, 20, 0, 0, 0)
            .unwrap()
            .timestamp_nanos_opt()
            .unwrap() as Timestamp;
        assert_eq!(
            TimePeriod::Year.previous_period(timestamp),
            Some(previous_year)
        );

        assert_eq!(TimePeriod::AllTime.previous_period(timestamp), None);
    }

    #[test]
    fn test_timestamp_to_day_string() {
        let timestamp = Utc
            .with_ymd_and_hms(2023, 6, 20, 0, 0, 0)
            .unwrap()
            .timestamp_nanos_opt()
            .unwrap() as Timestamp;
        assert_eq!(timestamp_to_day_string(timestamp), "20062023");
    }

    #[test]
    fn test_timestamp_to_week_string() {
        let timestamp = Utc
            .with_ymd_and_hms(2023, 6, 20, 0, 0, 0)
            .unwrap()
            .timestamp_nanos_opt()
            .unwrap() as Timestamp;
        assert_eq!(timestamp_to_week_string(timestamp), "2023W25");
    }

    #[test]
    fn test_timestamp_to_quarter_string() {
        let timestamp = Utc
            .with_ymd_and_hms(2023, 6, 20, 0, 0, 0)
            .unwrap()
            .timestamp_nanos_opt()
            .unwrap() as Timestamp;
        assert_eq!(timestamp_to_quarter_string(timestamp), "2023Q2");
    }

    #[test]
    fn test_timestamp_to_month_string() {
        let timestamp = Utc
            .with_ymd_and_hms(2023, 6, 20, 0, 0, 0)
            .unwrap()
            .timestamp_nanos_opt()
            .unwrap() as Timestamp;
        assert_eq!(timestamp_to_month_string(timestamp), "062023");
    }

    #[test]
    fn test_end_period() {
        let timestamp = Utc
            .with_ymd_and_hms(2023, 5, 16, 5, 13, 0)
            .unwrap()
            .timestamp_nanos_opt()
            .unwrap() as Timestamp;

        let end_day = Utc.with_ymd_and_hms(2023, 5, 17, 0, 0, 0).unwrap();
        assert_eq!(TimePeriod::Day.end_period(timestamp), Some(end_day));

        let end_week = Utc.with_ymd_and_hms(2023, 5, 22, 0, 0, 0).unwrap();
        assert_eq!(TimePeriod::Week.end_period(timestamp), Some(end_week));

        let end_month = Utc.with_ymd_and_hms(2023, 6, 1, 0, 0, 0).unwrap();
        assert_eq!(TimePeriod::Month.end_period(timestamp), Some(end_month));

        let end_quarter = Utc.with_ymd_and_hms(2023, 7, 1, 0, 0, 0).unwrap();
        assert_eq!(TimePeriod::Quarter.end_period(timestamp), Some(end_quarter));

        let end_year = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
        assert_eq!(TimePeriod::Year.end_period(timestamp), Some(end_year));
    }

    #[test]
    fn test_start_period() {
        let timestamp = Utc
            .with_ymd_and_hms(2023, 5, 16, 5, 13, 0)
            .unwrap()
            .timestamp_nanos_opt()
            .unwrap() as Timestamp;

        let start_day = Utc.with_ymd_and_hms(2023, 5, 16, 0, 0, 0).unwrap();
        assert_eq!(TimePeriod::Day.start_period(timestamp), Some(start_day));

        let start_week = Utc.with_ymd_and_hms(2023, 5, 15, 0, 0, 0).unwrap();
        assert_eq!(TimePeriod::Week.start_period(timestamp), Some(start_week));

        let start_month = Utc.with_ymd_and_hms(2023, 5, 1, 0, 0, 0).unwrap();
        assert_eq!(TimePeriod::Month.start_period(timestamp), Some(start_month));

        let start_quarter = Utc.with_ymd_and_hms(2023, 4, 1, 0, 0, 0).unwrap();
        assert_eq!(
            TimePeriod::Quarter.start_period(timestamp),
            Some(start_quarter)
        );

        let start_year = Utc.with_ymd_and_hms(2023, 1, 1, 0, 0, 0).unwrap();
        assert_eq!(TimePeriod::Year.start_period(timestamp), Some(start_year));
    }
}
