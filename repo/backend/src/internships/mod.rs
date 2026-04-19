use chrono::{Datelike, Duration, FixedOffset, NaiveDate, NaiveDateTime, TimeZone, Weekday};
use shared::ReportType;

use crate::config::PolicyConfig;

/// Default spec-defined late grace window if policy override missing.
pub const DEFAULT_LATE_GRACE_HOURS: i64 = 72;

/// Compute the server-authoritative due date for a report type in the
/// configured local timezone, then return it as a UTC-naive datetime for
/// storage (DATETIME columns).
///
/// Local-time rules (spec):
///   * DAILY   -> today 23:59:59
///   * WEEKLY  -> Monday 12:00 local of the current ISO week
///                (the upcoming Monday if today is after that Monday's noon)
///   * MONTHLY -> 5th 17:00 local of the current month
///
/// Internals always operate in the `PolicyConfig::local_timezone_offset_minutes`
/// tz; the final UTC conversion is an explicit step so ambiguity is caught.
pub fn compute_due_at(
    report_type: ReportType,
    now_utc: NaiveDateTime,
    policy: &PolicyConfig,
) -> NaiveDateTime {
    let tz = local_offset(policy);
    let now_local = tz.from_utc_datetime(&now_utc).naive_local();
    let local_due = compute_due_local(report_type, now_local);
    // Convert back to UTC naive for DB storage.
    tz.from_local_datetime(&local_due)
        .single()
        .map(|dt| dt.naive_utc())
        .unwrap_or(local_due)
}

/// Test/inspection helper: compute the due date already expressed in local time.
pub fn compute_due_local(report_type: ReportType, now_local: NaiveDateTime) -> NaiveDateTime {
    let date = now_local.date();
    match report_type {
        ReportType::Daily => date.and_hms_opt(23, 59, 59).expect("valid time"),
        ReportType::Weekly => {
            let days_from_monday = date.weekday().num_days_from_monday() as i64;
            let monday = date - Duration::days(days_from_monday);
            monday.and_hms_opt(12, 0, 0).expect("valid time")
        }
        ReportType::Monthly => {
            let fifth = NaiveDate::from_ymd_opt(now_local.year(), now_local.month(), 5)
                .expect("valid date");
            fifth.and_hms_opt(17, 0, 0).expect("valid time")
        }
    }
}

fn local_offset(policy: &PolicyConfig) -> FixedOffset {
    let m = policy
        .local_timezone_offset_minutes
        .clamp(-14 * 60, 14 * 60);
    FixedOffset::east_opt(m * 60).unwrap_or_else(|| FixedOffset::east_opt(0).unwrap())
}

/// Lateness evaluation:
///   Ok(false)            - on time
///   Ok(true)             - late but within grace window (accept + flag)
///   Err(LatenessError::PastGrace) - past grace, reject
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum LatenessError {
    #[error("submission is past the {0}h grace window")]
    PastGrace(i64),
}

pub fn evaluate_lateness(
    submitted_at: NaiveDateTime,
    due_at: NaiveDateTime,
    grace_hours: i64,
) -> Result<bool, LatenessError> {
    let grace = grace_hours.max(0);
    if submitted_at <= due_at {
        Ok(false)
    } else if submitted_at <= due_at + Duration::hours(grace) {
        Ok(true)
    } else {
        Err(LatenessError::PastGrace(grace))
    }
}

/// Returns true iff the submitter is allowed to override the server-computed
/// due date. For weekly/monthly reports, this is NEVER permitted — policy
/// deadlines are server-authoritative. Daily reports also use the server
/// default; any client-provided `due_at` on weekly/monthly is rejected.
pub fn client_can_override_due(_report_type: ReportType) -> bool {
    false
}

// Helper kept for diagnostics: "what's the next Monday noon after `now`?"
pub fn next_weekday_noon(now_local: NaiveDateTime, wd: Weekday) -> NaiveDateTime {
    let today = now_local.date();
    let today_idx = today.weekday().num_days_from_monday() as i64;
    let target_idx = wd.num_days_from_monday() as i64;
    let delta = (target_idx - today_idx).rem_euclid(7);
    let target = today + Duration::days(delta);
    target.and_hms_opt(12, 0, 0).expect("valid time")
}

// Compatibility wrapper for existing call sites during migration. New code
// should use `compute_due_at`; this version applies no offset.
pub fn default_due_at(report_type: ReportType, now_utc: NaiveDateTime) -> NaiveDateTime {
    compute_due_at(report_type, now_utc, &PolicyConfig::default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn weekly_is_monday_noon_local() {
        // Wednesday 10:00 local -> Monday of the same week 12:00 local.
        let wed = NaiveDate::from_ymd_opt(2026, 3, 4)
            .unwrap()
            .and_hms_opt(10, 0, 0)
            .unwrap();
        let due = compute_due_local(ReportType::Weekly, wed);
        let mon = NaiveDate::from_ymd_opt(2026, 3, 2)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();
        assert_eq!(due, mon);
    }

    #[test]
    fn monthly_is_fifth_5pm_local() {
        let now = NaiveDate::from_ymd_opt(2026, 4, 18)
            .unwrap()
            .and_hms_opt(9, 0, 0)
            .unwrap();
        let due = compute_due_local(ReportType::Monthly, now);
        let expected = NaiveDate::from_ymd_opt(2026, 4, 5)
            .unwrap()
            .and_hms_opt(17, 0, 0)
            .unwrap();
        assert_eq!(due, expected);
    }

    #[test]
    fn daily_is_end_of_day() {
        let now = NaiveDate::from_ymd_opt(2026, 4, 18)
            .unwrap()
            .and_hms_opt(9, 0, 0)
            .unwrap();
        let due = compute_due_local(ReportType::Daily, now);
        let expected = NaiveDate::from_ymd_opt(2026, 4, 18)
            .unwrap()
            .and_hms_opt(23, 59, 59)
            .unwrap();
        assert_eq!(due, expected);
    }

    #[test]
    fn grace_window_boundaries() {
        let due = NaiveDate::from_ymd_opt(2026, 4, 6)
            .unwrap()
            .and_hms_opt(17, 0, 0)
            .unwrap();
        assert_eq!(evaluate_lateness(due, due, 72), Ok(false));
        assert_eq!(
            evaluate_lateness(due + Duration::hours(72), due, 72),
            Ok(true)
        );
        assert_eq!(
            evaluate_lateness(due + Duration::hours(73), due, 72),
            Err(LatenessError::PastGrace(72))
        );
    }

    #[test]
    fn tz_offset_shifts_due_date_correctly() {
        // US Pacific UTC-8 (-480 min). At 2026-04-18 07:00 UTC, local time
        // is 2026-04-17 23:00 — the Monday *noon local* for that week is
        // 2026-04-13 12:00 local which is 2026-04-13 20:00 UTC.
        let pol = PolicyConfig {
            local_timezone_offset_minutes: -480,
            late_grace_hours: 72,
        };
        let now_utc = NaiveDate::from_ymd_opt(2026, 4, 18)
            .unwrap()
            .and_hms_opt(7, 0, 0)
            .unwrap();
        let due_utc = compute_due_at(ReportType::Weekly, now_utc, &pol);
        let expected = NaiveDate::from_ymd_opt(2026, 4, 13)
            .unwrap()
            .and_hms_opt(20, 0, 0)
            .unwrap();
        assert_eq!(due_utc, expected);
    }

    #[test]
    fn client_cannot_override_weekly_or_monthly() {
        assert!(!client_can_override_due(ReportType::Weekly));
        assert!(!client_can_override_due(ReportType::Monthly));
        assert!(!client_can_override_due(ReportType::Daily));
    }
}
