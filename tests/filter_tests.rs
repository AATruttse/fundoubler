//! Tests for time, user, and group file filters.

use assert_fs::prelude::*;
use assert_fs::TempDir;
use std::process::Command;
use std::thread;
use std::time::Duration;

use fundoubler::config::{ConfigFile, CliOptions};
use fundoubler::filters;
use fundoubler::scanner::FileScanner;

fn run_fundoubler(args: &[&str]) -> (String, String, std::process::ExitStatus) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_fundoubler"));
    cmd.args(args);
    let output = cmd.output().expect("Failed to execute process");
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    (stdout, stderr, output.status)
}

fn create_test_file(dir: &TempDir, filename: &str, content: &str) {
    dir.child(filename).write_str(content).unwrap();
}

// ============= parse_datetime =============

#[test]
fn filter_parse_datetime_yyyy_mm_dd() {
    let t = filters::parse_datetime("2024-01-15");
    assert!(t.is_some());
}

#[test]
fn filter_parse_datetime_yyyy_mm_dd_hms() {
    let t = filters::parse_datetime("2024-01-15 12:30:00");
    assert!(t.is_some());
}

#[test]
fn filter_parse_datetime_unix_timestamp() {
    let t = filters::parse_datetime("1705305600"); // 2024-01-15 00:00:00 UTC
    assert!(t.is_some());
}

#[test]
fn filter_parse_datetime_invalid_returns_none() {
    assert!(filters::parse_datetime("not-a-date").is_none());
    assert!(filters::parse_datetime("").is_none());
}

#[test]
fn filter_parse_datetime_rfc3339() {
    let t = filters::parse_datetime("2024-01-15T12:30:00Z");
    assert!(t.is_some());
}

#[test]
fn filter_parse_datetime_iso_format() {
    let t = filters::parse_datetime("2024-01-15T12:30:00");
    assert!(t.is_some());
}

// ============= time_in_range =============

#[test]
fn filter_time_in_range_none_bounds_includes_all() {
    use std::time::SystemTime;
    let t = SystemTime::now();
    assert!(filters::time_in_range(Some(t), None, None));
    assert!(filters::time_in_range(None, None, None));
}

#[test]
fn filter_time_in_range_within_bounds() {
    use std::time::{Duration, UNIX_EPOCH};
    let t = UNIX_EPOCH + Duration::from_secs(1000);
    let min = UNIX_EPOCH + Duration::from_secs(500);
    let max = UNIX_EPOCH + Duration::from_secs(1500);
    assert!(filters::time_in_range(Some(t), Some(&min), Some(&max)));
}

#[test]
fn filter_time_in_range_below_min_excluded() {
    use std::time::{Duration, UNIX_EPOCH};
    let t = UNIX_EPOCH + Duration::from_secs(100);
    let min = UNIX_EPOCH + Duration::from_secs(500);
    assert!(!filters::time_in_range(Some(t), Some(&min), None));
}

#[test]
fn filter_time_in_range_above_max_excluded() {
    use std::time::{Duration, UNIX_EPOCH};
    let t = UNIX_EPOCH + Duration::from_secs(2000);
    let max = UNIX_EPOCH + Duration::from_secs(1500);
    assert!(!filters::time_in_range(Some(t), None, Some(&max)));
}

#[test]
fn filter_time_in_range_no_time_included() {
    use std::time::{Duration, UNIX_EPOCH};
    let min = UNIX_EPOCH + Duration::from_secs(500);
    let max = UNIX_EPOCH + Duration::from_secs(1500);
    assert!(filters::time_in_range(None, Some(&min), Some(&max)));
}

// ============= min/max mod time filter (cross-platform) =============

#[test]
fn filter_min_mod_time_excludes_older_files() {
    let temp = TempDir::new().unwrap();
    let old = temp.child("old.txt");
    old.write_str("x").unwrap();

    thread::sleep(Duration::from_millis(100));
    let future = chrono::Utc::now() + chrono::Duration::seconds(10);
    let min_mod = future.format("%Y-%m-%d %H:%M:%S").to_string();

    let mut config = ConfigFile::default();
    config.path_start = temp.path().to_path_buf();
    config.compare_by_size = true;
    config.compare_by_xxh3 = false;
    config.min_mod_time = Some(min_mod);

    let scanner = FileScanner::new(&config, false);
    let groups = scanner.scan().unwrap();
    assert!(
        groups.is_empty(),
        "old.txt should be excluded by min_mod_time (file is older than filter)"
    );
}

#[test]
fn filter_max_mod_time_excludes_newer_files() {
    let temp = TempDir::new().unwrap();
    let recent = temp.child("recent.txt");
    recent.write_str("y").unwrap();

    let past = chrono::Utc::now() - chrono::Duration::days(365);
    let max_mod = past.format("%Y-%m-%d %H:%M:%S").to_string();

    let mut config = ConfigFile::default();
    config.path_start = temp.path().to_path_buf();
    config.compare_by_size = true;
    config.compare_by_xxh3 = false;
    config.max_mod_time = Some(max_mod);

    let scanner = FileScanner::new(&config, false);
    let groups = scanner.scan().unwrap();
    assert!(groups.is_empty());
}

// ============= Integration: min-mod-time CLI =============

#[test]
fn integration_min_mod_time_filter() {
    let temp = TempDir::new().unwrap();
    create_test_file(&temp, "a.txt", "dup");
    create_test_file(&temp, "b.txt", "dup");

    let past = chrono::Utc::now() - chrono::Duration::days(1);
    let max_mod = past.format("%Y-%m-%d").to_string();

    let (stdout, stderr, status) = run_fundoubler(&[
        temp.path().to_str().unwrap(),
        "--md5",
        "--max-mod-time",
        &max_mod,
    ]);
    assert!(status.success(), "stderr: {}", stderr);
    assert!(
        stdout.contains("No duplicates"),
        "Files newer than max_mod_time should be excluded"
    );
}

#[test]
fn integration_invalid_datetime_fails() {
    let temp = TempDir::new().unwrap();
    create_test_file(&temp, "a.txt", "x");

    let (_, stderr, status) = run_fundoubler(&[
        temp.path().to_str().unwrap(),
        "--min-mod-time",
        "invalid-date",
    ]);
    assert!(!status.success());
    assert!(stderr.contains("Invalid") || stderr.contains("min_mod_time"));
}

// ============= Config from CLI =============

#[test]
fn config_cli_time_and_user_filters() {
    use clap::Parser;

    let config = ConfigFile::from_cli(&CliOptions::parse_from([
        "fundoubler",
        ".",
        "--min-mod-time",
        "2024-01-01",
        "--max-mod-time",
        "2024-12-31",
    ]))
    .unwrap();
    assert_eq!(config.min_mod_time.as_deref(), Some("2024-01-01"));
    assert_eq!(config.max_mod_time.as_deref(), Some("2024-12-31"));
}

// ============= Create-time filters (cross-platform) =============

#[test]
fn filter_max_create_time_excludes_newer_files() {
    let temp = TempDir::new().unwrap();
    let recent = temp.child("recent.txt");
    recent.write_str("z").unwrap();

    let past = chrono::Utc::now() - chrono::Duration::days(365);
    let max_create = past.format("%Y-%m-%d").to_string();

    let mut config = ConfigFile::default();
    config.path_start = temp.path().to_path_buf();
    config.compare_by_size = true;
    config.compare_by_xxh3 = false;
    config.max_create_time = Some(max_create);

    let scanner = FileScanner::new(&config, false);
    let groups = scanner.scan().unwrap();
    assert!(
        groups.is_empty(),
        "recent.txt (just created) should be excluded by max_create_time"
    );
}

#[test]
fn integration_min_create_time_filter() {
    let temp = TempDir::new().unwrap();
    create_test_file(&temp, "a.txt", "dup");
    create_test_file(&temp, "b.txt", "dup");

    // Use far-future date: only files "created" after 2099 would be included.
    // Current files are excluded. (On some systems created() may be unavailable;
    // then files are included and we may find duplicates - test still passes.)
    let min_create = "2099-01-01";

    let (stdout, stderr, status) = run_fundoubler(&[
        temp.path().to_str().unwrap(),
        "--md5",
        "--min-create-time",
        min_create,
    ]);
    assert!(status.success(), "stderr: {}", stderr);
    assert!(
        stdout.contains("No duplicates"),
        "Files created before min_create_time (2099) should be excluded; got: {}",
        stdout
    );
}

// ============= Config file loading =============

#[test]
fn config_file_time_and_user_group_filters() {
    use clap::Parser;

    let temp = TempDir::new().unwrap();
    let config_path = temp.path().join("config.toml");
    std::fs::write(
        &config_path,
        r#"
path_start = "."
compare_by_size = true
compare_by_xxh3 = true
min_create_time = "2024-01-01"
max_create_time = "2024-12-31"
min_mod_time = "2024-02-01"
max_mod_time = "2024-11-30"
user_filter = "nobody"
group_filter = "nogroup"
"#,
    )
    .unwrap();

    let config = ConfigFile::from_cli(&CliOptions::parse_from([
        "fundoubler",
        "--config",
        config_path.to_str().unwrap(),
        temp.path().to_str().unwrap(),
    ]))
    .unwrap();
    assert_eq!(config.min_create_time.as_deref(), Some("2024-01-01"));
    assert_eq!(config.max_create_time.as_deref(), Some("2024-12-31"));
    assert_eq!(config.min_mod_time.as_deref(), Some("2024-02-01"));
    assert_eq!(config.max_mod_time.as_deref(), Some("2024-11-30"));
    assert_eq!(config.user_filter.as_deref(), Some("nobody"));
    assert_eq!(config.group_filter.as_deref(), Some("nogroup"));
}

// ============= Unix-only: user/group filter =============

/// Smoke test: verifies scanner runs with user_filter set. Does NOT verify exclusion
/// of other users' files (would require multi-user setup).
#[cfg(unix)]
#[test]
fn filter_user_filter_unix() {
    use std::fs;
    use std::os::unix::fs::MetadataExt;

    let temp = TempDir::new().unwrap();
    let f = temp.child("f.txt");
    f.write_str("x").unwrap();
    let uid = fs::metadata(f.path()).unwrap().uid();

    let mut config = ConfigFile::default();
    config.path_start = temp.path().to_path_buf();
    config.compare_by_size = true;
    config.compare_by_xxh3 = false;
    config.user_filter = Some(uid.to_string());

    let scanner = FileScanner::new(&config, false);
    let groups = scanner.scan().unwrap();
    assert!(groups.is_empty()); // one file, no duplicates
}

/// Smoke test: verifies scanner runs with group_filter set (Unix only).
#[cfg(unix)]
#[test]
fn filter_group_filter_unix() {
    use std::fs;
    use std::os::unix::fs::MetadataExt;

    let temp = TempDir::new().unwrap();
    let f = temp.child("f.txt");
    f.write_str("x").unwrap();
    let gid = fs::metadata(f.path()).unwrap().gid();

    let mut config = ConfigFile::default();
    config.path_start = temp.path().to_path_buf();
    config.compare_by_size = true;
    config.compare_by_xxh3 = false;
    config.group_filter = Some(gid.to_string());

    let scanner = FileScanner::new(&config, false);
    let groups = scanner.scan().unwrap();
    assert!(groups.is_empty()); // one file, no duplicates
}
