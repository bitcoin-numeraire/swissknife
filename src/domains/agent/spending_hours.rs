use chrono::{DateTime, Datelike, Timelike, Utc};
use swissknife_types::AgentSpendingHours;

use crate::application::errors::DataError;

pub(super) fn validate_hours(hours: &mut AgentSpendingHours) -> Result<(), DataError> {
    if hours.weekdays.is_empty() || hours.weekdays.len() > 7 || hours.weekdays.iter().any(|d| !(1..=7).contains(d)) {
        return Err(DataError::Validation(
            "Select weekdays from 1 (Monday) to 7 (Sunday)".into(),
        ));
    }
    hours.weekdays.sort_unstable();
    if hours.weekdays.windows(2).any(|days| days[0] == days[1]) {
        return Err(DataError::Validation("Select each weekday once".into()));
    }
    if hours.start_minute >= 1440 || hours.end_minute > 1440 || hours.start_minute == hours.end_minute {
        return Err(DataError::Validation(
            "Choose a UTC start from 0–1439 and a different end from 0–1440".into(),
        ));
    }
    Ok(())
}

pub(crate) fn authorize_hours(hours: &AgentSpendingHours, now: DateTime<Utc>) -> Result<(), DataError> {
    let minute = (now.hour() * 60 + now.minute()) as u16;
    let day = now.weekday();
    let starts_today = hours.weekdays.contains(&(day.number_from_monday() as u8));
    let allowed = if hours.start_minute < hours.end_minute {
        starts_today && (hours.start_minute..hours.end_minute).contains(&minute)
    } else {
        (starts_today && minute >= hours.start_minute)
            || (hours.weekdays.contains(&(day.pred().number_from_monday() as u8)) && minute < hours.end_minute)
    };
    if allowed {
        Ok(())
    } else {
        Err(DataError::Validation(
            "Delegated payments are outside this account’s UTC spending hours".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    #[test]
    fn sunday_window_crosses_iso_week_and_end_is_exclusive() {
        let hours = AgentSpendingHours {
            weekdays: vec![7],
            start_minute: 22 * 60,
            end_minute: 2 * 60,
        };
        for (day, hour, minute, allow) in [
            (29, 21, 59, false),
            (29, 22, 0, true),
            (30, 1, 59, true),
            (30, 2, 0, false),
            (30, 22, 0, false),
        ] {
            assert_eq!(
                authorize_hours(&hours, Utc.with_ymd_and_hms(2026, 11, day, hour, minute, 0).unwrap()).is_ok(),
                allow
            );
        }
    }
}
