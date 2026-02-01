//! Comprehensive tests for delete log and restore functionality.
//! Covers del_log module, config/CLI, and integration scenarios.

use assert_fs::prelude::*;
use assert_fs::TempDir;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::Command;
use std::thread;
use std::time::Duration;

use clap::Parser;
use fundoubler::config::{CliOptions, ConfigFile};
use fundoubler::del_log;

// --- Helpers ---

fn run_fundoubler(args: &[&str]) -> (String, String, std::process::ExitStatus) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_fundoubler"));
    cmd.args(args);
    let output = cmd.output().expect("Failed to execute process");
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    (stdout, stderr, output.status)
}

fn run_fundoubler_with_stdin(args: &[&str], stdin_input: &str) -> (String, String, std::process::ExitStatus) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_fundoubler"));
    cmd.args(args).stdin(std::process::Stdio::piped()).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped());
    let mut child = cmd.spawn().expect("Failed to spawn");
    {
        let mut stdin = child.stdin.take().unwrap();
        stdin.write_all(stdin_input.as_bytes()).unwrap();
    }
    let output = child.wait_with_output().expect("Failed to wait");
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    (stdout, stderr, output.status)
}

fn create_test_file(dir: &TempDir, filename: &str, content: &str) {
    dir.child(filename).write_str(content).unwrap();
}

// =============================================================================
// del_log MODULE - Unit tests
// =============================================================================

#[test]
fn del_log_unit_del_logs_dir_constructs_correct_path() {
    let temp = TempDir::new().unwrap();
    let logs_dir = temp.path().join("logs");
    let dir = del_log::del_logs_dir(&logs_dir);
    assert!(dir.ends_with("del_logs"));
    assert_eq!(dir.parent().unwrap(), logs_dir);
}

#[test]
fn del_log_unit_create_del_log_creates_directory_and_file() {
    let temp = TempDir::new().unwrap();
    let logs_dir = temp.path().join("logs");
    let (path, mut file) = del_log::create_del_log(&logs_dir).unwrap();

    assert!(path.exists());
    assert!(path.parent().unwrap().exists());
    assert!(path.parent().unwrap().file_name().unwrap() == "del_logs");

    del_log::write_record(&mut file, Path::new("/a/deleted.txt"), Path::new("/a/kept.txt")).unwrap();
    drop(file);

    let content = fs::read_to_string(&path).unwrap();
    assert!(content.contains("deleted:/a/deleted.txt"));
    assert!(content.contains("source:/a/kept.txt"));
}

#[test]
fn del_log_unit_create_del_log_file_naming_format() {
    let temp = TempDir::new().unwrap();
    let logs_dir = temp.path().join("logs");
    let (path, _) = del_log::create_del_log(&logs_dir).unwrap();

    let name = path.file_name().unwrap().to_str().unwrap();
    assert!(name.ends_with("fundel.log"));
    assert!(name.len() >= 23); // YYYYMMDDHHMMSS (14) + "fundel.log" (9)
    assert!(name.chars().take(14).all(|c| c.is_ascii_digit()));
}

#[test]
fn del_log_unit_write_record_format() {
    let temp = TempDir::new().unwrap();
    let logs_dir = temp.path().join("logs");
    let (path, mut file) = del_log::create_del_log(&logs_dir).unwrap();

    del_log::write_record(&mut file, Path::new("C:\\del\\a.txt"), Path::new("C:\\keep\\a.txt")).unwrap();
    del_log::write_record(&mut file, Path::new("/tmp/b.txt"), Path::new("/tmp/c.txt")).unwrap();
    drop(file);

    let content = fs::read_to_string(&path).unwrap();
    assert!(content.starts_with("deleted:"));
    assert!(content.contains("source:"));
    let lines: Vec<&str> = content.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 4);
}

#[test]
fn del_log_unit_parse_del_log_empty_file() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("empty.log");
    fs::write(&path, "").unwrap();

    let records = del_log::parse_del_log(&path).unwrap();
    assert!(records.is_empty());
}

#[test]
fn del_log_unit_parse_del_log_single_record() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("one.log");
    fs::write(
        &path,
        "deleted:/path/to/deleted.txt\nsource:/path/to/kept.txt\n",
    )
    .unwrap();

    let records = del_log::parse_del_log(&path).unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].0, Path::new("/path/to/deleted.txt"));
    assert_eq!(records[0].1, Path::new("/path/to/kept.txt"));
}

#[test]
fn del_log_unit_parse_del_log_multiple_records() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("many.log");
    fs::write(
        &path,
        "deleted:/a.txt\nsource:/b.txt\ndeleted:/c.txt\nsource:/d.txt\n",
    )
    .unwrap();

    let records = del_log::parse_del_log(&path).unwrap();
    assert_eq!(records.len(), 2);
    assert_eq!(records[0].0, Path::new("/a.txt"));
    assert_eq!(records[0].1, Path::new("/b.txt"));
    assert_eq!(records[1].0, Path::new("/c.txt"));
    assert_eq!(records[1].1, Path::new("/d.txt"));
}

#[test]
fn del_log_unit_parse_del_log_skips_empty_lines() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("blank.log");
    fs::write(
        &path,
        "\ndeleted:/a.txt\n\nsource:/b.txt\n\n",
    )
    .unwrap();

    let records = del_log::parse_del_log(&path).unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].0, Path::new("/a.txt"));
    assert_eq!(records[0].1, Path::new("/b.txt"));
}

#[test]
fn del_log_unit_parse_del_log_orphan_source_ignored() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("orphan.log");
    fs::write(
        &path,
        "source:/orphan.txt\ndeleted:/a.txt\nsource:/b.txt\n",
    )
    .unwrap();

    let records = del_log::parse_del_log(&path).unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].0, Path::new("/a.txt"));
    assert_eq!(records[0].1, Path::new("/b.txt"));
}

#[test]
fn del_log_unit_parse_del_log_double_deleted_second_pair_used() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("double.log");
    fs::write(
        &path,
        "deleted:/a.txt\ndeleted:/b.txt\nsource:/c.txt\n",
    )
    .unwrap();

    let records = del_log::parse_del_log(&path).unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].0, Path::new("/b.txt"));
    assert_eq!(records[0].1, Path::new("/c.txt"));
}

#[test]
fn del_log_unit_parse_del_log_trims_whitespace() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("trim.log");
    fs::write(
        &path,
        "deleted:  /a.txt  \nsource:  /b.txt  \n",
    )
    .unwrap();

    let records = del_log::parse_del_log(&path).unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].0, Path::new("/a.txt"));
    assert_eq!(records[0].1, Path::new("/b.txt"));
}

#[test]
fn del_log_unit_find_latest_del_log_nonexistent_dir_returns_none() {
    let temp = TempDir::new().unwrap();
    let logs_dir = temp.path().join("missing");
    let result = del_log::find_latest_del_log(&logs_dir).unwrap();
    assert!(result.is_none());
}

#[test]
fn del_log_unit_find_latest_del_log_empty_dir_returns_none() {
    let temp = TempDir::new().unwrap();
    let logs_dir = temp.path().join("logs");
    fs::create_dir_all(logs_dir.join("del_logs")).unwrap();

    let result = del_log::find_latest_del_log(&logs_dir).unwrap();
    assert!(result.is_none());
}

#[test]
fn del_log_unit_find_latest_del_log_single_file() {
    let temp = TempDir::new().unwrap();
    let logs_dir = temp.path().join("logs");
    let del_logs = logs_dir.join("del_logs");
    fs::create_dir_all(&del_logs).unwrap();
    let log_path = del_logs.join("20260124120000fundel.log");
    fs::write(&log_path, "deleted:/a\nsource:/b\n").unwrap();

    let result = del_log::find_latest_del_log(&logs_dir).unwrap();
    assert!(result.is_some());
    assert_eq!(result.unwrap(), log_path);
}

#[test]
fn del_log_unit_find_latest_del_log_multiple_files_picks_newest() {
    let temp = TempDir::new().unwrap();
    let logs_dir = temp.path().join("logs");
    let del_logs = logs_dir.join("del_logs");
    fs::create_dir_all(&del_logs).unwrap();

    let old = del_logs.join("20260124120000fundel.log");
    let new = del_logs.join("20260124130000fundel.log");
    fs::write(&old, "").unwrap();
    thread::sleep(Duration::from_millis(10));
    fs::write(&new, "").unwrap();

    let result = del_log::find_latest_del_log(&logs_dir).unwrap();
    assert!(result.is_some());
    assert_eq!(result.unwrap(), new);
}

#[test]
fn del_log_unit_find_latest_ignores_non_fundel_files() {
    let temp = TempDir::new().unwrap();
    let logs_dir = temp.path().join("logs");
    let del_logs = logs_dir.join("del_logs");
    fs::create_dir_all(&del_logs).unwrap();
    fs::write(del_logs.join("other.log"), "").unwrap();
    fs::write(del_logs.join("fun.log"), "").unwrap();

    let result = del_log::find_latest_del_log(&logs_dir).unwrap();
    assert!(result.is_none());
}

#[test]
fn del_log_unit_roundtrip_write_then_parse() {
    let temp = TempDir::new().unwrap();
    let logs_dir = temp.path().join("logs");
    let (path, mut file) = del_log::create_del_log(&logs_dir).unwrap();

    let pairs = [
        (Path::new("/d1"), Path::new("/s1")),
        (Path::new("/d2"), Path::new("/s2")),
    ];
    for (d, s) in &pairs {
        del_log::write_record(&mut file, d, s).unwrap();
    }
    drop(file);

    let records = del_log::parse_del_log(&path).unwrap();
    assert_eq!(records.len(), 2);
    assert_eq!(records[0].0, pairs[0].0);
    assert_eq!(records[0].1, pairs[0].1);
    assert_eq!(records[1].0, pairs[1].0);
    assert_eq!(records[1].1, pairs[1].1);
}

// =============================================================================
// Config / CLI - Unit tests
// =============================================================================

#[test]
fn config_cli_no_delete_log_sets_false() {
    let config = ConfigFile::from_cli(&CliOptions::parse_from([
        "fundoubler",
        ".",
        "--no-delete-log",
    ]))
    .unwrap();
    assert!(!config.delete_log);
}

#[test]
fn config_cli_default_delete_log_true() {
    let config = ConfigFile::from_cli(&CliOptions::parse_from([
        "fundoubler",
        ".",
    ]))
    .unwrap();
    assert!(config.delete_log);
}

#[test]
fn config_file_delete_log_false() {
    let temp = TempDir::new().unwrap();
    let config_path = temp.path().join("config.toml");
    fs::write(
        &config_path,
        r#"
path_start = "."
delete_log = false
compare_by_size = true
compare_by_xxh3 = true
"#,
    )
    .unwrap();

    let config = ConfigFile::from_cli(&CliOptions::parse_from([
        "fundoubler",
        "--config",
        config_path.to_str().unwrap(),
        ".",
    ]))
    .unwrap();
    assert!(!config.delete_log);
}

#[test]
fn config_file_delete_log_true_overridden_by_cli() {
    let temp = TempDir::new().unwrap();
    let config_path = temp.path().join("config.toml");
    fs::write(
        &config_path,
        r#"
path_start = "."
delete_log = true
compare_by_size = true
compare_by_xxh3 = true
"#,
    )
    .unwrap();

    let config = ConfigFile::from_cli(&CliOptions::parse_from([
        "fundoubler",
        "--config",
        config_path.to_str().unwrap(),
        "--no-delete-log",
        ".",
    ]))
    .unwrap();
    assert!(!config.delete_log);
}

// =============================================================================
// Integration - Delete log creation
// =============================================================================

#[test]
fn integration_delete_log_created_on_deletion() {
    let temp_dir = TempDir::new().unwrap();
    let logs_dir = temp_dir.path().join("logs");

    create_test_file(&temp_dir, "a_kept.txt", "content");
    create_test_file(&temp_dir, "z_del.txt", "content");

    let (_stdout, stderr, status) = run_fundoubler(&[
        temp_dir.path().to_str().unwrap(),
        "--md5",
        "--delete",
        "--force-delete",
        "--skip-confirm",
        "--sort=name",
        "--logs-dir",
        logs_dir.to_str().unwrap(),
    ]);

    assert!(status.success(), "stderr: {}", stderr);
    let del_logs = logs_dir.join("del_logs");
    assert!(del_logs.exists());
    let entries: Vec<_> = fs::read_dir(&del_logs).unwrap().collect();
    assert!(!entries.is_empty());
    let content = fs::read_to_string(entries[0].as_ref().unwrap().path()).unwrap();
    assert!(content.contains("deleted:"));
    assert!(content.contains("source:"));
    assert!(content.contains("z_del") || content.contains("z_del.txt"));
    assert!(content.contains("a_kept") || content.contains("a_kept.txt"));

    let records = del_log::parse_del_log(&entries[0].as_ref().unwrap().path()).unwrap();
    assert_eq!(records.len(), 1);
    assert!(records[0].0.to_string_lossy().contains("z_del"));
    assert!(records[0].1.to_string_lossy().contains("a_kept"));
}

#[test]
fn integration_no_delete_log_disables_logging() {
    let temp_dir = TempDir::new().unwrap();
    let logs_dir = temp_dir.path().join("logs");

    create_test_file(&temp_dir, "a.txt", "x");
    create_test_file(&temp_dir, "z.txt", "x");

    let (_, stderr, status) = run_fundoubler(&[
        temp_dir.path().to_str().unwrap(),
        "--md5",
        "--delete",
        "--force-delete",
        "--skip-confirm",
        "--no-delete-log",
        "--logs-dir",
        logs_dir.to_str().unwrap(),
    ]);

    assert!(status.success(), "stderr: {}", stderr);
    let del_logs = logs_dir.join("del_logs");
    assert!(!del_logs.exists(), "del_logs should not be created when --no-delete-log");
}

#[test]
fn integration_delete_log_via_config_file() {
    let temp_dir = TempDir::new().unwrap();
    let logs_dir = temp_dir.path().join("logs");
    let config_path = temp_dir.path().join("config.toml");
    let logs_dir_str = logs_dir.to_string_lossy().replace('\\', "/");
    fs::write(
        &config_path,
        format!(
            r#"
compare_by_size = true
compare_by_xxh3 = true
delete = true
force_delete = true
sort_orders = ["Name"]
logs_dir = "{}"
delete_log = true
"#,
            logs_dir_str
        ),
    )
    .unwrap();

    create_test_file(&temp_dir, "a.txt", "dup");
    create_test_file(&temp_dir, "z.txt", "dup");

    let (_, stderr, status) = run_fundoubler(&[
        "--config",
        config_path.to_str().unwrap(),
        "--delete",
        "--force-delete",
        "--skip-confirm",
        temp_dir.path().to_str().unwrap(),
    ]);
    assert!(status.success(), "stderr: {}", stderr);
    assert!(logs_dir.join("del_logs").exists());
}

#[test]
fn integration_dry_run_does_not_create_delete_log() {
    let temp_dir = TempDir::new().unwrap();
    let logs_dir = temp_dir.path().join("logs");

    create_test_file(&temp_dir, "a.txt", "x");
    create_test_file(&temp_dir, "z.txt", "x");

    let (_, _, status) = run_fundoubler(&[
        temp_dir.path().to_str().unwrap(),
        "--md5",
        "--delete",
        "--dry-run",
        "--logs-dir",
        logs_dir.to_str().unwrap(),
    ]);
    assert!(status.success());
    assert!(!logs_dir.join("del_logs").exists());
}

#[test]
fn integration_multiple_groups_multiple_records_in_log() {
    let temp_dir = TempDir::new().unwrap();
    let logs_dir = temp_dir.path().join("logs");

    create_test_file(&temp_dir, "a1.txt", "content1");
    create_test_file(&temp_dir, "b1.txt", "content1");
    create_test_file(&temp_dir, "a2.txt", "content2");
    create_test_file(&temp_dir, "b2.txt", "content2");

    let (_, stderr, status) = run_fundoubler(&[
        temp_dir.path().to_str().unwrap(),
        "--md5",
        "--delete",
        "--force-delete",
        "--skip-confirm",
        "--sort=name",
        "--logs-dir",
        logs_dir.to_str().unwrap(),
    ]);
    assert!(status.success(), "stderr: {}", stderr);

    let del_logs = logs_dir.join("del_logs");
    let entries: Vec<_> = fs::read_dir(&del_logs).unwrap().collect();
    assert!(!entries.is_empty());
    let records = del_log::parse_del_log(&entries[0].as_ref().unwrap().path()).unwrap();
    assert_eq!(records.len(), 2);
}

// =============================================================================
// Integration - Restore
// =============================================================================

#[test]
fn integration_restore_from_specific_log() {
    let temp_dir = TempDir::new().unwrap();
    let logs_dir = temp_dir.path().join("logs");

    create_test_file(&temp_dir, "a_kept.txt", "content");
    let deleted_path = temp_dir.path().join("z_deleted.txt");
    fs::write(&deleted_path, "content").unwrap();

    let (_, _, status1) = run_fundoubler(&[
        temp_dir.path().to_str().unwrap(),
        "--md5",
        "--delete",
        "--force-delete",
        "--skip-confirm",
        "--sort=name",
        "--logs-dir",
        logs_dir.to_str().unwrap(),
    ]);
    assert!(status1.success());
    assert!(!deleted_path.exists(), "z_deleted.txt should be removed");

    let log_file = logs_dir
        .join("del_logs")
        .read_dir()
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();

    let (stdout, stderr, status2) = run_fundoubler(&[
        "--restore",
        log_file.to_str().unwrap(),
        "--skip-confirm",
    ]);
    assert!(status2.success(), "restore failed: {}", stderr);
    assert!(deleted_path.exists(), "restore should recreate z_deleted.txt");
    assert!(stdout.contains("Restored"));

    let restored_content = fs::read_to_string(&deleted_path).unwrap();
    assert_eq!(restored_content, "content");
}

#[test]
fn integration_restore_uses_latest_when_no_path() {
    let temp_dir = TempDir::new().unwrap();
    let logs_dir = temp_dir.path().join("mylogs");

    create_test_file(&temp_dir, "a.txt", "dup");
    create_test_file(&temp_dir, "z.txt", "dup");

    let (_, _, status1) = run_fundoubler(&[
        temp_dir.path().to_str().unwrap(),
        "--md5",
        "--delete",
        "--force-delete",
        "--skip-confirm",
        "--sort=name",
        "--logs-dir",
        logs_dir.to_str().unwrap(),
    ]);
    assert!(status1.success());

    let (stdout, stderr, status2) = run_fundoubler(&[
        "--restore",
        "--logs-dir",
        logs_dir.to_str().unwrap(),
        "--skip-confirm",
    ]);
    assert!(status2.success(), "restore with --logs-dir: {}", stderr);
    assert!(stdout.contains("Restored") || stdout.contains("Restoring"));
}

#[test]
fn integration_restore_empty_log() {
    let temp_dir = TempDir::new().unwrap();
    let del_logs = temp_dir.path().join("del_logs");
    fs::create_dir_all(&del_logs).unwrap();
    let log_path = del_logs.join("20260124120000fundel.log");
    fs::write(&log_path, "").unwrap();

    let (stdout, _stderr, status) = run_fundoubler(&["--restore", log_path.to_str().unwrap()]);
    assert!(status.success());
    assert!(stdout.contains("empty") || stdout.contains("Nothing to restore"));
}

#[test]
fn config_cli_restore_parsing() {
    use clap::Parser;
    
    let cli = CliOptions::parse_from(["fundoubler", "--restore"]);
    assert!(cli.restore.is_some());
    
    let cli_with_path = CliOptions::parse_from([
        "fundoubler",
        "--restore",
        "/path/to/log/fundel.log",
    ]);
    assert!(cli_with_path.restore.is_some());
    assert_eq!(
        cli_with_path.restore.as_ref().unwrap().to_string_lossy(),
        "/path/to/log/fundel.log"
    );
}

#[test]
fn integration_restore_nonexistent_log_fails() {
    let temp_dir = TempDir::new().unwrap();
    let nonexistent = temp_dir.path().join("nonexistent_fundel.log");

    let (_stdout, stderr, status) = run_fundoubler(&["--restore", nonexistent.to_str().unwrap()]);
    assert!(!status.success(), "restore with nonexistent log should fail");
}

#[test]
fn integration_restore_no_log_found_fails() {
    let temp_dir = TempDir::new().unwrap();
    let logs_dir = temp_dir.path().join("empty_logs");
    fs::create_dir_all(&logs_dir).unwrap();

    let (_stdout, stderr, status) = run_fundoubler(&[
        "--restore",
        "--logs-dir",
        logs_dir.to_str().unwrap(),
    ]);
    assert!(!status.success());
    assert!(stderr.contains("No delete log") || stderr.contains("not found"));
}

#[test]
fn integration_restore_when_source_missing_reports_error() {
    let temp_dir = TempDir::new().unwrap();
    let log_path = temp_dir.path().join("restore.log");
    let source_path = temp_dir.path().join("source_gone.txt");
    let deleted_path = temp_dir.path().join("to_restore.txt");

    fs::write(
        &log_path,
        format!(
            "deleted:{}\nsource:{}\n",
            deleted_path.display(),
            source_path.display()
        ),
    )
    .unwrap();

    let (stdout, stderr, status) = run_fundoubler(&[
        "--restore",
        log_path.to_str().unwrap(),
        "--skip-confirm",
    ]);
    assert!(status.success()); // restore still exits 0 but reports errors per record
    assert!(
        stderr.contains("no longer exists") || stderr.contains("Source") || stdout.contains("error"),
        "expected error about missing source, got stdout: {} stderr: {}",
        stdout,
        stderr
    );
}

#[test]
fn integration_restore_skips_when_deleted_already_exists() {
    let temp_dir = TempDir::new().unwrap();
    let log_path = temp_dir.path().join("restore.log");
    let source_path = temp_dir.path().join("source.txt");
    let deleted_path = temp_dir.path().join("deleted.txt");

    fs::write(&source_path, "content").unwrap();
    fs::write(&deleted_path, "existing").unwrap();
    fs::write(
        &log_path,
        format!(
            "deleted:{}\nsource:{}\n",
            deleted_path.display(),
            source_path.display()
        ),
    )
    .unwrap();

    let (stdout, stderr, status) = run_fundoubler(&[
        "--restore",
        log_path.to_str().unwrap(),
        "--skip-confirm",
    ]);
    assert!(status.success());
    assert!(
        stderr.contains("already exists") || stdout.contains("Skipping"),
        "expected skip message, got stdout: {} stderr: {}",
        stdout,
        stderr
    );
    assert_eq!(fs::read_to_string(&deleted_path).unwrap(), "existing");
}

#[test]
fn integration_restore_asks_confirmation_without_skip_confirm() {
    let temp_dir = TempDir::new().unwrap();
    let log_path = temp_dir.path().join("restore.log");
    let source_path = temp_dir.path().join("source.txt");
    let deleted_path = temp_dir.path().join("deleted.txt");

    fs::write(&source_path, "content").unwrap();
    fs::write(
        &log_path,
        format!(
            "deleted:{}\nsource:{}\n",
            deleted_path.display(),
            source_path.display()
        ),
    )
    .unwrap();

    // Pipe "n" to decline - file should not be restored
    let (stdout, stderr, _status) = run_fundoubler_with_stdin(&["--restore", log_path.to_str().unwrap()], "n\n");
    // Status may be non-zero if dialoguer fails on non-TTY; main assertion is file not restored
    assert!(!deleted_path.exists(), "file should not be restored when user declines, stdout: {} stderr: {}", stdout, stderr);
}

#[test]
fn integration_restore_creates_parent_dirs() {
    let temp_dir = TempDir::new().unwrap();
    let sub = temp_dir.path().join("sub").join("nested");
    fs::create_dir_all(temp_dir.path().join("sub")).unwrap();
    let source_path = temp_dir.path().join("source.txt");
    let deleted_path = sub.join("deleted.txt");

    fs::write(&source_path, "content").unwrap();
    fs::write(
        &temp_dir.path().join("restore.log"),
        format!(
            "deleted:{}\nsource:{}\n",
            deleted_path.display(),
            source_path.display()
        ),
    )
    .unwrap();

    let (_, stderr, status) = run_fundoubler(&[
        "--restore",
        temp_dir.path().join("restore.log").to_str().unwrap(),
        "--skip-confirm",
    ]);
    assert!(status.success(), "stderr: {}", stderr);
    assert!(deleted_path.exists());
    assert_eq!(fs::read_to_string(&deleted_path).unwrap(), "content");
}
