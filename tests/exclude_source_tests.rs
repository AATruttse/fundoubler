//! Comprehensive tests for exclude_dirs and source_dirs functionality.
//! Covers CLI, config file, unit-level scanner, and integration scenarios.

use assert_fs::prelude::*;
use assert_fs::TempDir;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use fundoubler::config::ConfigFile;
use fundoubler::scanner::FileScanner;

// --- Helpers ---

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

/// Normalize path for cross-platform comparison (forward slashes)
fn norm(p: &std::path::Path) -> String {
    p.to_string_lossy().replace('\\', "/")
}

// =============================================================================
// EXCLUDE_DIRS - Unit tests (scanner)
// =============================================================================

#[test]
fn exclude_unit_empty_exclude_dirs_scans_all() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::write(root.join("a.txt"), "x").unwrap();
    std::fs::write(root.join("b.txt"), "x").unwrap();
    let sub = root.join("sub");
    std::fs::create_dir_all(&sub).unwrap();
    std::fs::write(sub.join("c.txt"), "x").unwrap();

    let mut config = ConfigFile::default();
    config.path_start = root.to_path_buf();
    config.compare_by_xxh3 = true;
    config.compare_by_size = true;
    config.exclude_dirs = vec![];

    let scanner = FileScanner::new(&config, false);
    let groups = scanner.scan().unwrap();
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].paths.len(), 3);
}

#[test]
fn exclude_unit_single_exclude_dir() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::write(root.join("a.txt"), "same").unwrap();
    std::fs::write(root.join("b.txt"), "same").unwrap();
    let skip = root.join("skip_me");
    std::fs::create_dir_all(&skip).unwrap();
    std::fs::write(skip.join("c.txt"), "same").unwrap();

    let mut config = ConfigFile::default();
    config.path_start = root.to_path_buf();
    config.compare_by_xxh3 = true;
    config.compare_by_size = true;
    config.exclude_dirs = vec![skip.clone()];

    let scanner = FileScanner::new(&config, false);
    let groups = scanner.scan().unwrap();
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].paths.len(), 2);
    assert!(!groups[0].paths.iter().any(|p| norm(p).contains("skip_me")));
}

#[test]
fn exclude_unit_multiple_exclude_dirs() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::write(root.join("a.txt"), "same").unwrap();
    let skip1 = root.join("skip1");
    let skip2 = root.join("skip2");
    std::fs::create_dir_all(&skip1).unwrap();
    std::fs::create_dir_all(&skip2).unwrap();
    std::fs::write(skip1.join("b.txt"), "same").unwrap();
    std::fs::write(skip2.join("c.txt"), "same").unwrap();

    let mut config = ConfigFile::default();
    config.path_start = root.to_path_buf();
    config.compare_by_xxh3 = true;
    config.compare_by_size = true;
    config.exclude_dirs = vec![skip1.clone(), skip2.clone()];

    let scanner = FileScanner::new(&config, false);
    let groups = scanner.scan().unwrap();
    assert!(
        groups.is_empty(),
        "Only one file (a.txt) remains, no duplicates"
    );
}

#[test]
fn exclude_unit_relative_exclude_path() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::write(root.join("a.txt"), "same").unwrap();
    let skip = root.join("ignored");
    std::fs::create_dir_all(&skip).unwrap();
    std::fs::write(skip.join("b.txt"), "same").unwrap();

    let mut config = ConfigFile::default();
    config.path_start = root.to_path_buf();
    config.compare_by_xxh3 = true;
    config.compare_by_size = true;
    config.exclude_dirs = vec![PathBuf::from("ignored")];

    let scanner = FileScanner::new(&config, false);
    let groups = scanner.scan().unwrap();
    // a.txt only (b.txt in excluded ignored/); no duplicates
    assert!(groups.is_empty(), "Only a.txt remains, no duplicate group");
}

#[test]
fn exclude_unit_exclude_subdir_with_similar_name() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let exclude = root.join("cache");
    std::fs::create_dir_all(&exclude).unwrap();
    std::fs::write(exclude.join("x.txt"), "dup").unwrap();
    let keep = root.join("cache_backup");
    std::fs::create_dir_all(&keep).unwrap();
    std::fs::write(keep.join("x.txt"), "dup").unwrap();

    let mut config = ConfigFile::default();
    config.path_start = root.to_path_buf();
    config.compare_by_xxh3 = true;
    config.compare_by_size = true;
    config.exclude_dirs = vec![exclude.clone()];

    let scanner = FileScanner::new(&config, false);
    let groups = scanner.scan().unwrap();
    assert!(
        groups.is_empty(),
        "cache_backup and cache have same file but only cache_backup scanned - one file, no dup"
    );
}

#[test]
fn exclude_unit_nested_excluded_dir() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::write(root.join("a.txt"), "same").unwrap();
    let skip = root.join("skip");
    std::fs::create_dir_all(skip.join("nested/deep")).unwrap();
    std::fs::write(skip.join("nested/deep/file.txt"), "same").unwrap();

    let mut config = ConfigFile::default();
    config.path_start = root.to_path_buf();
    config.compare_by_xxh3 = true;
    config.compare_by_size = true;
    config.exclude_dirs = vec![skip.clone()];

    let scanner = FileScanner::new(&config, false);
    let groups = scanner.scan().unwrap();
    // Only a.txt; file.txt is in excluded skip/
    assert!(groups.is_empty(), "No duplicates - nested dir excluded");
}

// =============================================================================
// SOURCE_DIRS - Unit tests (scanner)
// =============================================================================

#[test]
fn source_unit_empty_source_dirs_path_sort() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let d1 = root.join("dir_a");
    let d2 = root.join("dir_b");
    std::fs::create_dir_all(&d1).unwrap();
    std::fs::create_dir_all(&d2).unwrap();
    std::fs::write(d1.join("f.txt"), "dup").unwrap();
    std::fs::write(d2.join("f.txt"), "dup").unwrap();

    let mut config = ConfigFile::default();
    config.path_start = root.to_path_buf();
    config.compare_by_xxh3 = true;
    config.compare_by_size = true;
    config.source_dirs = vec![];

    let scanner = FileScanner::new(&config, false);
    let groups = scanner.scan().unwrap();
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].paths.len(), 2);
    // First should be lexicographically smaller path
    assert!(groups[0].paths[0] < groups[0].paths[1]);
}

#[test]
fn source_unit_single_source_dir_first() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let source = root.join("source");
    let other = root.join("other");
    std::fs::create_dir_all(&source).unwrap();
    std::fs::create_dir_all(&other).unwrap();
    std::fs::write(source.join("keep.txt"), "dup").unwrap();
    std::fs::write(other.join("del.txt"), "dup").unwrap();

    let mut config = ConfigFile::default();
    config.path_start = root.to_path_buf();
    config.compare_by_xxh3 = true;
    config.compare_by_size = true;
    config.source_dirs = vec![source.clone()];

    let scanner = FileScanner::new(&config, false);
    let groups = scanner.scan().unwrap();
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].paths.len(), 2);
    assert!(
        norm(&groups[0].paths[0]).contains("source"),
        "First (kept) should be from source"
    );
}

#[test]
fn source_unit_multiple_source_dirs() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let src1 = root.join("src1");
    let src2 = root.join("src2");
    let other = root.join("other");
    std::fs::create_dir_all(&src1).unwrap();
    std::fs::create_dir_all(&src2).unwrap();
    std::fs::create_dir_all(&other).unwrap();
    std::fs::write(src1.join("a.txt"), "dup").unwrap();
    std::fs::write(src2.join("b.txt"), "dup").unwrap();
    std::fs::write(other.join("c.txt"), "dup").unwrap();

    let mut config = ConfigFile::default();
    config.path_start = root.to_path_buf();
    config.compare_by_xxh3 = true;
    config.compare_by_size = true;
    config.source_dirs = vec![src1.clone(), src2.clone()];

    let scanner = FileScanner::new(&config, false);
    let groups = scanner.scan().unwrap();
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].paths.len(), 3);
    // First should be in src1 or src2 (both are source)
    let first_in_source =
        norm(&groups[0].paths[0]).contains("src1") || norm(&groups[0].paths[0]).contains("src2");
    assert!(first_in_source, "First path should be from a source dir");
}

#[test]
fn source_unit_both_in_source_dirs() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let src = root.join("source");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(src.join("a.txt"), "dup").unwrap();
    std::fs::write(src.join("b.txt"), "dup").unwrap();

    let mut config = ConfigFile::default();
    config.path_start = root.to_path_buf();
    config.compare_by_xxh3 = true;
    config.compare_by_size = true;
    config.source_dirs = vec![src.clone()];

    let scanner = FileScanner::new(&config, false);
    let groups = scanner.scan().unwrap();
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].paths.len(), 2);
    assert!(norm(&groups[0].paths[0]).contains("source"));
    assert!(norm(&groups[0].paths[1]).contains("source"));
}

#[test]
fn source_unit_relative_source_path() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let source = root.join("primary");
    let other = root.join("secondary");
    std::fs::create_dir_all(&source).unwrap();
    std::fs::create_dir_all(&other).unwrap();
    std::fs::write(source.join("f.txt"), "dup").unwrap();
    std::fs::write(other.join("f.txt"), "dup").unwrap();

    let mut config = ConfigFile::default();
    config.path_start = root.to_path_buf();
    config.compare_by_xxh3 = true;
    config.compare_by_size = true;
    config.source_dirs = vec![PathBuf::from("primary")];

    let scanner = FileScanner::new(&config, false);
    let groups = scanner.scan().unwrap();
    assert_eq!(groups.len(), 1);
    assert!(
        norm(&groups[0].paths[0]).contains("primary"),
        "Primary (source) should be first"
    );
}

#[test]
fn source_unit_nested_in_source() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let source = root.join("source");
    std::fs::create_dir_all(source.join("sub/deep")).unwrap();
    std::fs::write(source.join("sub/deep/file.txt"), "dup").unwrap();
    let other = root.join("other");
    std::fs::create_dir_all(&other).unwrap();
    std::fs::write(other.join("file.txt"), "dup").unwrap();

    let mut config = ConfigFile::default();
    config.path_start = root.to_path_buf();
    config.compare_by_xxh3 = true;
    config.compare_by_size = true;
    config.source_dirs = vec![source.clone()];

    let scanner = FileScanner::new(&config, false);
    let groups = scanner.scan().unwrap();
    assert_eq!(groups.len(), 1);
    assert!(
        norm(&groups[0].paths[0]).contains("source"),
        "File in source/sub/deep should count as in source"
    );
}

// =============================================================================
// EXCLUDE_DIRS - Integration tests (CLI)
// =============================================================================

#[test]
fn exclude_integration_cli_single() {
    let temp = TempDir::new().unwrap();
    create_test_file(&temp, "a.txt", "same");
    let skip = temp.child("skip_me");
    skip.create_dir_all().unwrap();
    skip.child("b.txt").write_str("same").unwrap();

    let path_str = temp.path().join("skip_me");
    let (stdout, stderr, status) = run_fundoubler(&[
        temp.path().to_str().unwrap(),
        "--md5",
        "--exclude-dir",
        path_str.to_str().unwrap(),
    ]);
    assert!(status.success(), "stderr: {}", stderr);
    assert!(
        !stdout.contains("b.txt") || stdout.contains("No duplicates"),
        "b.txt in excluded dir should not appear"
    );
}

#[test]
fn exclude_integration_cli_multiple() {
    let temp = TempDir::new().unwrap();
    create_test_file(&temp, "only.txt", "unique");
    let skip1 = temp.child("skip1");
    let skip2 = temp.child("skip2");
    skip1.create_dir_all().unwrap();
    skip2.create_dir_all().unwrap();
    skip1.child("x.txt").write_str("dup").unwrap();
    skip2.child("y.txt").write_str("dup").unwrap();

    let (stdout, stderr, status) = run_fundoubler(&[
        temp.path().to_str().unwrap(),
        "--md5",
        "--exclude-dir",
        temp.path().join("skip1").to_str().unwrap(),
        "--exclude-dir",
        temp.path().join("skip2").to_str().unwrap(),
    ]);
    assert!(status.success(), "stderr: {}", stderr);
    assert!(stdout.contains("No duplicates") || !stdout.contains("dup"));
}

#[test]
fn exclude_integration_config_file() {
    let temp = TempDir::new().unwrap();
    create_test_file(&temp, "a.txt", "same");
    let skip = temp.child("ignored");
    skip.create_dir_all().unwrap();
    skip.child("b.txt").write_str("same").unwrap();

    let config_path = temp.path().join("config.toml");
    let skip_path = temp.path().join("ignored");
    let skip_str = skip_path.to_str().unwrap().replace('\\', "/");
    fs::write(
        &config_path,
        format!(
            r#"
path_start = "."
compare_by_size = true
compare_by_xxh3 = true
exclude_dirs = ["{}"]
"#,
            skip_str
        ),
    )
    .unwrap();

    let (stdout, stderr, status) = run_fundoubler(&[
        "--config",
        config_path.to_str().unwrap(),
        temp.path().to_str().unwrap(),
    ]);
    assert!(status.success(), "stderr: {}", stderr);
    assert!(
        !stdout.contains("b.txt") || stdout.contains("No duplicates"),
        "exclude_dirs from config should work"
    );
}

// =============================================================================
// SOURCE_DIRS - Integration tests (CLI)
// =============================================================================

#[test]
fn source_integration_cli_keeps_source_on_deletion() {
    let temp = TempDir::new().unwrap();
    let source = temp.child("source");
    source.create_dir_all().unwrap();
    source.child("keep.txt").write_str("dup").unwrap();
    let other = temp.child("other");
    other.create_dir_all().unwrap();
    other.child("del.txt").write_str("dup").unwrap();

    let (_, stderr, status) = run_fundoubler(&[
        temp.path().to_str().unwrap(),
        "--md5",
        "--delete",
        "--force-delete",
        "--skip-confirm",
        "--source-dir",
        temp.path().join("source").to_str().unwrap(),
    ]);
    assert!(status.success(), "stderr: {}", stderr);
    assert!(source.child("keep.txt").path().exists());
    assert!(!other.child("del.txt").path().exists());
}

#[test]
fn source_integration_display_order_source_first() {
    let temp = TempDir::new().unwrap();
    let source = temp.child("source");
    source.create_dir_all().unwrap();
    source.child("first.txt").write_str("dup").unwrap();
    let other = temp.child("other");
    other.create_dir_all().unwrap();
    other.child("second.txt").write_str("dup").unwrap();

    let (stdout, stderr, status) = run_fundoubler(&[
        temp.path().to_str().unwrap(),
        "--md5",
        "--source-dir",
        temp.path().join("source").to_str().unwrap(),
    ]);
    assert!(status.success(), "stderr: {}", stderr);
    // First file in "Files:" section should be from source (first.txt)
    let file_section = stdout.find("Files:").and_then(|i| stdout.get(i..));
    assert!(
        file_section
            .map(|s| s.contains("first.txt"))
            .unwrap_or(false),
        "Output should list first.txt (from source)"
    );
    // first.txt should appear before second.txt in the output
    let first_pos = stdout.find("first.txt");
    let second_pos = stdout.find("second.txt");
    assert!(first_pos.is_some() && second_pos.is_some());
    assert!(
        first_pos.unwrap() < second_pos.unwrap(),
        "Source file (first.txt) should appear before other (second.txt)"
    );
}

#[test]
fn source_integration_config_file() {
    let temp = TempDir::new().unwrap();
    let source = temp.child("primary");
    source.create_dir_all().unwrap();
    source.child("keep.txt").write_str("dup").unwrap();
    let other = temp.child("other");
    other.create_dir_all().unwrap();
    other.child("del.txt").write_str("dup").unwrap();

    let config_path = temp.path().join("config.toml");
    let source_str = temp
        .path()
        .join("primary")
        .to_str()
        .unwrap()
        .replace('\\', "/");
    fs::write(
        &config_path,
        format!(
            r#"
path_start = "."
compare_by_size = true
compare_by_xxh3 = true
source_dirs = ["{}"]
"#,
            source_str
        ),
    )
    .unwrap();

    let (_, stderr, status) = run_fundoubler(&[
        "--config",
        config_path.to_str().unwrap(),
        temp.path().to_str().unwrap(),
        "--delete",
        "--force-delete",
        "--skip-confirm",
    ]);
    assert!(status.success(), "stderr: {}", stderr);
    assert!(source.child("keep.txt").path().exists());
    assert!(!other.child("del.txt").path().exists());
}

// =============================================================================
// COMBINED - Exclude + Source
// =============================================================================

#[test]
fn combined_exclude_and_source() {
    let temp = TempDir::new().unwrap();
    let source = temp.child("source");
    source.create_dir_all().unwrap();
    source.child("keep.txt").write_str("dup").unwrap();
    let other = temp.child("other");
    other.create_dir_all().unwrap();
    other.child("del.txt").write_str("dup").unwrap();
    let excluded = temp.child("excluded");
    excluded.create_dir_all().unwrap();
    excluded.child("also_dup.txt").write_str("dup").unwrap();

    let (_, stderr, status) = run_fundoubler(&[
        temp.path().to_str().unwrap(),
        "--md5",
        "--exclude-dir",
        temp.path().join("excluded").to_str().unwrap(),
        "--source-dir",
        temp.path().join("source").to_str().unwrap(),
        "--delete",
        "--force-delete",
        "--skip-confirm",
    ]);
    assert!(status.success(), "stderr: {}", stderr);
    assert!(source.child("keep.txt").path().exists());
    assert!(!other.child("del.txt").path().exists());
    assert!(
        excluded.child("also_dup.txt").path().exists(),
        "Excluded file untouched"
    );
}

#[test]
fn source_integration_dry_run_shows_correct_order() {
    let temp = TempDir::new().unwrap();
    let source = temp.child("source");
    source.create_dir_all().unwrap();
    source.child("keep.txt").write_str("dup").unwrap();
    let other = temp.child("other");
    other.create_dir_all().unwrap();
    other.child("del.txt").write_str("dup").unwrap();

    let (stdout, stderr, status) = run_fundoubler(&[
        temp.path().to_str().unwrap(),
        "--md5",
        "--source-dir",
        temp.path().join("source").to_str().unwrap(),
        "--delete",
        "--dry-run",
    ]);
    assert!(status.success(), "stderr: {}", stderr);
    assert!(stdout.contains("DRY RUN"));
    assert!(source.child("keep.txt").path().exists());
    assert!(
        other.child("del.txt").path().exists(),
        "Dry run must not delete"
    );
}

#[test]
fn combined_one_dup_in_excluded_one_scanned_no_group() {
    let temp = TempDir::new().unwrap();
    create_test_file(&temp, "only.txt", "content");
    let skip = temp.child("skip");
    skip.create_dir_all().unwrap();
    skip.child("same.txt").write_str("content").unwrap();

    let (stdout, stderr, status) = run_fundoubler(&[
        temp.path().to_str().unwrap(),
        "--md5",
        "--exclude-dir",
        temp.path().join("skip").to_str().unwrap(),
    ]);
    assert!(status.success(), "stderr: {}", stderr);
    assert!(stdout.contains("No duplicates"));
}

// =============================================================================
// CONFIG - Parsing and override
// =============================================================================

#[test]
fn config_cli_parse_exclude_and_source() {
    use clap::Parser;
    use fundoubler::config::CliOptions;

    let args = [
        "fundoubler",
        "--exclude-dir",
        "node_modules",
        "--exclude-dir",
        "/tmp/cache",
        "--source-dir",
        "./backup",
        "--source-dir",
        "/primary",
    ];
    let cli = CliOptions::parse_from(args);
    let config = ConfigFile::from_cli(&cli).unwrap();
    assert_eq!(config.exclude_dirs.len(), 2);
    assert!(config.exclude_dirs.contains(&PathBuf::from("node_modules")));
    assert!(config.exclude_dirs.contains(&PathBuf::from("/tmp/cache")));
    assert_eq!(config.source_dirs.len(), 2);
    assert!(config.source_dirs.contains(&PathBuf::from("./backup")));
    assert!(config.source_dirs.contains(&PathBuf::from("/primary")));
}

#[test]
fn config_cli_exclude_overrides_config_file() {
    use clap::Parser;
    use fundoubler::config::CliOptions;

    let temp = tempfile::tempdir().unwrap();
    let config_path = temp.path().join("cfg.toml");
    fs::write(
        &config_path,
        r#"
path_start = "."
compare_by_size = true
compare_by_xxh3 = true
exclude_dirs = ["from_config"]
"#,
    )
    .unwrap();

    let args = [
        "fundoubler",
        "--config",
        config_path.to_str().unwrap(),
        "--exclude-dir",
        "from_cli",
    ];
    let cli = CliOptions::parse_from(args);
    let config = ConfigFile::from_cli(&cli).unwrap();
    assert_eq!(config.exclude_dirs.len(), 1);
    assert_eq!(config.exclude_dirs[0], PathBuf::from("from_cli"));
}

#[test]
fn config_cli_source_overrides_config_file() {
    use clap::Parser;
    use fundoubler::config::CliOptions;

    let temp = tempfile::tempdir().unwrap();
    let config_path = temp.path().join("cfg.toml");
    fs::write(
        &config_path,
        r#"
path_start = "."
compare_by_size = true
compare_by_xxh3 = true
source_dirs = ["from_config"]
"#,
    )
    .unwrap();

    let args = [
        "fundoubler",
        "--config",
        config_path.to_str().unwrap(),
        "--source-dir",
        "from_cli",
    ];
    let cli = CliOptions::parse_from(args);
    let config = ConfigFile::from_cli(&cli).unwrap();
    assert_eq!(config.source_dirs.len(), 1);
    assert_eq!(config.source_dirs[0], PathBuf::from("from_cli"));
}
