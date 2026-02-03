//! File filters: time ranges, user, group.
//! Time filters work on Windows and Linux. User/group filters work on Unix only.

use std::path::Path;
use std::time::SystemTime;

use chrono::{DateTime, Local, NaiveDate, NaiveDateTime, TimeZone, Utc};

/// Parse a datetime string to SystemTime. Tries multiple formats:
/// - Unix timestamp (seconds since epoch)
/// - YYYY-MM-DD
/// - YYYY-MM-DD HH:MM:SS
/// - YYYY-MM-DDTHH:MM:SS
pub fn parse_datetime(s: &str) -> Option<SystemTime> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    // Try Unix timestamp first
    if let Ok(secs) = s.parse::<i64>() {
        if let Some(dt) = Utc.timestamp_opt(secs, 0).single() {
            return Some(SystemTime::from(dt));
        }
    }
    // Try YYYY-MM-DD
    if let Ok(d) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        let ndt = d.and_hms_opt(0, 0, 0)?;
        let dt: DateTime<Local> = Local.from_local_datetime(&ndt).single()?;
        return Some(SystemTime::from(dt));
    }
    // Try YYYY-MM-DD HH:MM:SS
    if let Ok(ndt) = NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S") {
        let dt: DateTime<Local> = Local.from_local_datetime(&ndt).single()?;
        return Some(SystemTime::from(dt));
    }
    // Try YYYY-MM-DDTHH:MM:SS
    if let Ok(ndt) = NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S") {
        let dt: DateTime<Local> = Local.from_local_datetime(&ndt).single()?;
        return Some(SystemTime::from(dt));
    }
    // Try RFC 3339
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Some(SystemTime::from(dt));
    }
    None
}

/// Check if time is within [min, max] range (inclusive). None means no bound.
pub fn time_in_range(t: Option<SystemTime>, min: Option<&SystemTime>, max: Option<&SystemTime>) -> bool {
    let Some(t) = t else { return true }; // No time info: include file
    if let Some(m) = min {
        if t < *m {
            return false;
        }
    }
    if let Some(m) = max {
        if t > *m {
            return false;
        }
    }
    true
}

#[cfg(unix)]
/// Check if file matches user filter (Unix only). Returns true if no filter or match.
pub fn matches_user_filter(_path: &Path, uid: u32, user_filter: Option<&str>) -> bool {
    let Some(filter) = user_filter else { return true };
    let filter = filter.trim();
    if filter.is_empty() {
        return true;
    }
    // Try numeric uid first
    if let Ok(fuid) = filter.parse::<u32>() {
        return uid == fuid;
    }
    // Look up by name
    if let Some(user) = users::get_user_by_uid(uid) {
        return user.name().to_string_lossy() == filter;
    }
    false
}

#[cfg(unix)]
/// Check if file matches group filter (Unix only). Returns true if no filter or match.
pub fn matches_group_filter(_path: &Path, gid: u32, group_filter: Option<&str>) -> bool {
    let Some(filter) = group_filter else { return true };
    let filter = filter.trim();
    if filter.is_empty() {
        return true;
    }
    if let Ok(fgid) = filter.parse::<u32>() {
        return gid == fgid;
    }
    if let Some(group) = users::get_group_by_gid(gid) {
        return group.name().to_string_lossy() == filter;
    }
    false
}

#[cfg(not(unix))]
/// On Windows, user filter is not supported; all files pass.
pub fn matches_user_filter(_path: &Path, _uid: u32, _user_filter: Option<&str>) -> bool {
    true
}

#[cfg(not(unix))]
/// On Windows, group filter is not supported; all files pass.
pub fn matches_group_filter(_path: &Path, _gid: u32, _group_filter: Option<&str>) -> bool {
    true
}
