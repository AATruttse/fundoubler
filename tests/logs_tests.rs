//! Comprehensive tests for logging: CLI, config, levels, file naming, and content.

use assert_fs::prelude::*;
use assert_fs::TempDir;
use std::fs;
use std::path::Path;
use std::process::Command;

use fundoubler::config::{CliOptions, ConfigFile};
use fundoubler::log;

fn run_fundoubler(args: &[&str]) -> (String, String, std::process::ExitStatus) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_fundoubler"));
    cmd.args(args);
    let output = cmd.output().expect("Failed to execute process");
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    (stdout, stderr, output.status)
}

fn read_log_content(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_default()
}

fn find_log_file(logs_dir: &Path) -> Option<std::path::PathBuf> {
    let entries = fs::read_dir(logs_dir).ok()?;
    for e in entries.flatten() {
        let p = e.path();
        if p.extension().map_or(false, |e| e == "log")
            && p.file_stem()
                .and_then(|s| s.to_str())
                .map_or(false, |s| s.ends_with("fun"))
        {
            return Some(p);
        }
    }
    None
}

// =============================================================================
// Log module unit tests
// =============================================================================

#[test]
fn log_unit_init_level_0_creates_no_file() {
    log::reset();
    let temp = tempfile::tempdir().unwrap();
    log::init(0, temp.path()).unwrap();
    log::log_error("should not appear");
    log::log_info("should not appear");
    assert!(log::current_log_path().is_none());
    let entries: Vec<_> = fs::read_dir(temp.path()).unwrap().collect();
    assert!(
        entries.is_empty(),
        "No log file should be created when level is 0"
    );
}

#[test]
fn log_unit_init_level_1_creates_file() {
    log::reset();
    let temp = tempfile::tempdir().unwrap();
    log::init(1, temp.path()).unwrap();
    assert!(log::current_log_path().is_some());
    let path = log::current_log_path().unwrap();
    assert!(path.exists());
    assert!(path
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .ends_with("fun.log"));
    assert!(path
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .starts_with("20")); // YYYY
}

#[test]
fn log_unit_level_1_only_errors() {
    log::reset();
    let temp = tempfile::tempdir().unwrap();
    log::init(1, temp.path()).unwrap();
    log::log_error("error msg");
    log::log_info("info should not appear");
    log::log_debug("debug should not appear");
    drop(log::current_log_path());
    let path = find_log_file(temp.path()).expect("log file should exist");
    let content = read_log_content(&path);
    assert!(content.contains("error msg"));
    assert!(!content.contains("info should not appear"));
    assert!(!content.contains("debug should not appear"));
    assert!(content.contains("ERROR"));
}

#[test]
fn log_unit_level_2_errors_and_info() {
    log::reset();
    let temp = tempfile::tempdir().unwrap();
    log::init(2, temp.path()).unwrap();
    log::log_error("err");
    log::log_info("inf");
    log::log_debug("dbg should not appear");
    let path = find_log_file(temp.path()).expect("log file should exist");
    let content = read_log_content(&path);
    assert!(content.contains("err"));
    assert!(content.contains("inf"));
    assert!(!content.contains("dbg should not appear"));
}

#[test]
fn log_unit_level_3_all_levels() {
    log::reset();
    let temp = tempfile::tempdir().unwrap();
    log::init(3, temp.path()).unwrap();
    log::log_error("e");
    log::log_info("i");
    log::log_debug("d");
    let path = find_log_file(temp.path()).expect("log file should exist");
    let content = read_log_content(&path);
    assert!(content.contains("e"));
    assert!(content.contains("i"));
    assert!(content.contains("d"));
}

#[test]
fn log_unit_file_naming_format() {
    log::reset();
    let temp = tempfile::tempdir().unwrap();
    log::init(1, temp.path()).unwrap();
    let path = log::current_log_path().unwrap();
    let name = path.file_name().unwrap().to_str().unwrap();
    assert!(name.ends_with("fun.log"), "expected *fun.log, got {}", name);
    let prefix = name.strip_suffix("fun.log").unwrap();
    assert!(!prefix.is_empty() && prefix.chars().all(|c| c.is_ascii_digit()),
        "prefix should be non-empty timestamp digits (e.g. YYYYMMDDHHMMSS), got {}", prefix);
}

#[test]
fn log_unit_creates_logs_dir() {
    log::reset();
    let temp = tempfile::tempdir().unwrap();
    let sub = temp.path().join("sub").join("nested");
    log::init(1, &sub).unwrap();
    assert!(sub.exists());
    assert!(sub.is_dir());
    let path = log::current_log_path().unwrap();
    assert!(path.parent().unwrap() == sub);
}

#[test]
fn log_unit_reinit_replaces_state() {
    log::reset();
    let temp = tempfile::tempdir().unwrap();
    log::init(2, temp.path()).unwrap();
    log::log_info("first");
    log::init(1, temp.path()).unwrap();
    log::log_info("second info - should not appear at level 1");
    log::log_error("second err");
    // Reinit creates a new log file (new timestamp); read the current one we're writing to
    let path = log::current_log_path().expect("log file should exist after reinit");
    let content = read_log_content(&path);
    assert!(
        content.contains("second err"),
        "reinit to level 1 should still log errors"
    );
    assert!(
        !content.contains("second info"),
        "reinit to level 1 should not log info"
    );
}

#[test]
fn log_unit_reset_clears_state() {
    log::reset();
    let temp = tempfile::tempdir().unwrap();
    log::init(2, temp.path()).unwrap();
    log::log_info("before reset");
    log::reset();
    log::log_info("after reset");
    let path = find_log_file(temp.path()).unwrap();
    let content = read_log_content(&path);
    assert!(content.contains("before reset"));
    assert!(!content.contains("after reset"));
}

// =============================================================================
// Config / CLI parsing
// =============================================================================

#[test]
fn config_cli_log_level_parsing() {
    use clap::Parser;

    let args = ["fundoubler", "-l"];
    let cli = CliOptions::parse_from(args);
    assert_eq!(cli.log_level, 1);

    let args = ["fundoubler", "-ll"];
    let cli = CliOptions::parse_from(args);
    assert_eq!(cli.log_level, 2);

    let args = ["fundoubler", "-l", "-l", "-l"];
    let cli = CliOptions::parse_from(args);
    assert_eq!(cli.log_level, 3);

    let args = ["fundoubler", "--log-level"];
    let cli = CliOptions::parse_from(args);
    assert_eq!(cli.log_level, 1);

    let args = ["fundoubler"];
    let cli = CliOptions::parse_from(args);
    assert_eq!(cli.log_level, 0);
}

#[test]
fn config_cli_logs_dir_parsing() {
    use clap::Parser;

    let args = ["fundoubler", "--logs-dir", "/var/log/fundoubler"];
    let cli = CliOptions::parse_from(args);
    assert_eq!(
        cli.logs_dir.as_deref(),
        Some(std::path::Path::new("/var/log/fundoubler"))
    );

    let args = ["fundoubler"];
    let cli = CliOptions::parse_from(args);
    assert!(cli.logs_dir.is_none());
}

#[test]
fn config_from_cli_log_options() {
    use clap::Parser;

    let args = [
        "fundoubler",
        "-ll",
        "--logs-dir",
        "/tmp/mylogs",
        "/scan/path",
    ];
    let cli = CliOptions::parse_from(args);
    let config = ConfigFile::from_cli(&cli).unwrap();
    assert_eq!(config.log_level, 2);
    assert_eq!(config.logs_dir, std::path::PathBuf::from("/tmp/mylogs"));
}

#[test]
fn config_file_log_level_and_logs_dir() {
    use clap::Parser;

    let temp = tempfile::tempdir().unwrap();
    let config_path = temp.path().join("cfg.toml");
    let custom_logs = temp.path().join("custom_logs");
    let custom_logs_str = custom_logs.to_str().unwrap().replace('\\', "/");
    fs::write(
        &config_path,
        format!(
            r#"
path_start = "."
compare_by_size = true
compare_by_xxh3 = true
log_level = 3
logs_dir = "{}"
"#,
            custom_logs_str
        ),
    )
    .unwrap();

    let args = ["fundoubler", "--config", config_path.to_str().unwrap()];
    let cli = CliOptions::parse_from(args);
    let config = ConfigFile::from_cli(&cli).unwrap();
    assert_eq!(config.log_level, 3);
    assert!(
        config.logs_dir.ends_with("custom_logs")
            || config.logs_dir.to_string_lossy().contains("custom_logs")
    );
}

// =============================================================================
// Integration tests (run binary)
// =============================================================================

#[test]
fn integration_log_level_1_creates_log_on_success() {
    log::reset();
    let temp = TempDir::new().unwrap();
    temp.child("a.txt").write_str("x").unwrap();
    temp.child("b.txt").write_str("x").unwrap();
    let logs_dir = temp.path().join("logs");
    fs::create_dir_all(&logs_dir).unwrap();

    let (_, stderr, status) = run_fundoubler(&[
        temp.path().to_str().unwrap(),
        "--md5",
        "-l",
        "--logs-dir",
        logs_dir.to_str().unwrap(),
    ]);

    assert!(status.success(), "stderr: {}", stderr);
    let log_file = find_log_file(&logs_dir).expect("log file should exist");
    // Level 1 = errors only; success path logs info (level 2+), so content may be empty
    assert!(log_file.exists(), "log file should be created");
}

#[test]
fn integration_log_level_2_info_messages() {
    log::reset();
    let temp = TempDir::new().unwrap();
    temp.child("a.txt").write_str("dup").unwrap();
    temp.child("b.txt").write_str("dup").unwrap();
    let logs_dir = temp.path().join("logs");

    let (stdout, stderr, status) = run_fundoubler(&[
        temp.path().to_str().unwrap(),
        "--md5",
        "-ll",
        "--logs-dir",
        logs_dir.to_str().unwrap(),
    ]);

    assert!(status.success(), "stderr: {}", stderr);
    assert!(stdout.contains("duplicates"));
    let log_file = find_log_file(&logs_dir).expect("log file should exist");
    let content = read_log_content(&log_file);
    assert!(content.contains("INFO"));
    assert!(
        content.contains("Logging initialized")
            || content.contains("scan")
            || content.contains("complete")
    );
}

#[test]
fn integration_log_level_3_debug_messages() {
    log::reset();
    let temp = TempDir::new().unwrap();
    temp.child("f.txt").write_str("x").unwrap();
    let logs_dir = temp.path().join("logs");

    run_fundoubler(&[
        temp.path().to_str().unwrap(),
        "--md5",
        "-lll",
        "--logs-dir",
        logs_dir.to_str().unwrap(),
    ]);

    let log_file = find_log_file(&logs_dir).expect("log file should exist");
    let content = read_log_content(&log_file);
    assert!(
        content.contains("DEBUG") || content.contains("Scanning") || content.contains("Collected")
    );
}

#[test]
fn integration_error_logged_on_config_failure() {
    log::reset();
    let temp = TempDir::new().unwrap();
    let config_path = temp.path().join("bad.toml");
    fs::write(&config_path, "invalid toml {{{").unwrap();
    let logs_dir = temp.path().join("logs");

    let (_, _stderr, status) = run_fundoubler(&[
        "--config",
        config_path.to_str().unwrap(),
        "-l",
        "--logs-dir",
        logs_dir.to_str().unwrap(),
    ]);

    assert!(!status.success());
    let log_file = find_log_file(&logs_dir).expect("log file should exist");
    let content = read_log_content(&log_file);
    assert!(content.contains("ERROR"));
    assert!(
        content.contains("Config") || content.contains("config") || content.contains("Invalid")
    );
}

#[test]
fn integration_default_logs_dir() {
    log::reset();
    let temp = TempDir::new().unwrap();
    temp.child("a.txt").write_str("x").unwrap();
    temp.child("b.txt").write_str("x").unwrap();

    // Run in temp dir so default ./logs is temp/logs
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_fundoubler"));
    cmd.current_dir(temp.path()).arg(".").arg("--md5").arg("-l");
    let output = cmd.output().unwrap();
    assert!(output.status.success());

    let logs_dir = temp.path().join("logs");
    assert!(logs_dir.exists(), "default ./logs should be created");
    let log_file = find_log_file(&logs_dir);
    assert!(
        log_file.is_some(),
        "log file should exist in default logs dir"
    );
}

#[test]
fn integration_log_via_config_file() {
    log::reset();
    let temp = TempDir::new().unwrap();
    temp.child("a.txt").write_str("x").unwrap();
    temp.child("b.txt").write_str("x").unwrap();
    let config_path = temp.path().join("cfg.toml");
    let logs_dir = temp.path().join("mylogs");
    let logs_str = logs_dir.to_str().unwrap().replace('\\', "/");
    fs::write(
        &config_path,
        format!(
            r#"
path_start = "."
compare_by_size = true
compare_by_xxh3 = true
log_level = 2
logs_dir = "{}"
"#,
            logs_str
        ),
    )
    .unwrap();

    let (_, stderr, status) = run_fundoubler(&[
        "--config",
        config_path.to_str().unwrap(),
        temp.path().to_str().unwrap(),
    ]);

    assert!(status.success(), "stderr: {}", stderr);
    assert!(logs_dir.exists());
    let log_file = find_log_file(&logs_dir).expect("log file from config");
    assert!(!read_log_content(&log_file).is_empty());
}
