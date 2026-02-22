//! Tests for search_dirs: only report duplicate groups that have at least one file in search_dirs.

use assert_fs::prelude::*;
use assert_fs::TempDir;
use std::process::Command;

use fundoubler::config::{CliOptions, ConfigFile};
use fundoubler::scanner::FileScanner;
use clap::Parser;

fn run_fundoubler(args: &[&str]) -> (String, String, std::process::ExitStatus) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_fundoubler"));
    cmd.args(args);
    let output = cmd.output().expect("Failed to execute process");
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    (stdout, stderr, output.status)
}

fn norm(p: &std::path::Path) -> String {
    p.to_string_lossy().replace('\\', "/")
}

// ============= Config: CLI =============

#[test]
fn config_cli_search_dir_parsing() {
    let config = ConfigFile::from_cli(&CliOptions::parse_from([
        "fundoubler",
        ".",
        "--search-dir",
        "search_a",
        "--search-dir",
        "search_b",
    ]))
    .unwrap();
    assert_eq!(config.search_dirs.len(), 2);
    assert!(config.search_dirs.iter().any(|p| p.to_string_lossy().contains("search_a")));
    assert!(config.search_dirs.iter().any(|p| p.to_string_lossy().contains("search_b")));
}

#[test]
fn config_cli_search_dir_empty_by_default() {
    let config = ConfigFile::from_cli(&CliOptions::parse_from(["fundoubler", "."])).unwrap();
    assert!(config.search_dirs.is_empty());
}

// ============= Config: file =============

#[test]
fn config_file_search_dirs() {
    let temp = TempDir::new().unwrap();
    let config_path = temp.path().join("config.toml");
    std::fs::write(
        &config_path,
        r#"
path_start = "."
compare_by_size = true
compare_by_xxh3 = true
search_dirs = ["search_here", "and_here"]
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
    assert_eq!(config.search_dirs.len(), 2);
}

#[test]
fn config_file_search_dirs_overridden_by_cli() {
    let temp = TempDir::new().unwrap();
    let config_path = temp.path().join("config.toml");
    std::fs::write(
        &config_path,
        r#"
path_start = "."
compare_by_size = true
compare_by_xxh3 = true
search_dirs = ["old_search"]
"#,
    )
    .unwrap();

    let config = ConfigFile::from_cli(&CliOptions::parse_from([
        "fundoubler",
        "--config",
        config_path.to_str().unwrap(),
        ".",
        "--search-dir",
        "cli_search",
    ]))
    .unwrap();
    assert_eq!(config.search_dirs.len(), 1);
    assert!(config.search_dirs[0].to_string_lossy().contains("cli_search"));
}

// ============= Unit: scanner search_dirs empty =============

#[test]
fn search_unit_empty_search_dirs_all_groups_shown() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::write(root.join("a.txt"), "dup").unwrap();
    std::fs::write(root.join("b.txt"), "dup").unwrap();
    let sub = root.join("sub");
    std::fs::create_dir_all(&sub).unwrap();
    std::fs::write(sub.join("c.txt"), "other").unwrap();
    std::fs::write(root.join("d.txt"), "other").unwrap();

    let mut config = ConfigFile::default();
    config.path_start = root.to_path_buf();
    config.compare_by_xxh3 = true;
    config.compare_by_size = true;
    config.search_dirs = vec![];

    let scanner = FileScanner::new(&config, false);
    let groups = scanner.scan().unwrap();
    // Two groups: (a.txt, b.txt) and (sub/c.txt, d.txt). search_dirs empty => no filter, all shown.
    assert_eq!(groups.len(), 2);
    assert!(groups.iter().all(|g| g.paths.len() == 2));
}

// ============= Unit: scanner search_dirs restricts groups =============

#[test]
fn search_unit_only_groups_with_file_in_search_dir() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let in_search = root.join("in_search");
    let outside = root.join("outside");
    std::fs::create_dir_all(&in_search).unwrap();
    std::fs::create_dir_all(&outside).unwrap();

    // Duplicate pair 1: one in search, one outside -> report
    std::fs::write(in_search.join("f1.txt"), "content1").unwrap();
    std::fs::write(outside.join("f1.txt"), "content1").unwrap();

    // Duplicate pair 2: both outside search -> do not report
    std::fs::write(outside.join("f2.txt"), "content2").unwrap();
    std::fs::write(root.join("f2.txt"), "content2").unwrap();

    let mut config = ConfigFile::default();
    config.path_start = root.to_path_buf();
    config.compare_by_xxh3 = true;
    config.compare_by_size = true;
    config.search_dirs = vec![in_search.clone()];

    let scanner = FileScanner::new(&config, false);
    let groups = scanner.scan().unwrap();
    assert_eq!(groups.len(), 1, "Only group with a file in in_search");
    assert!(groups[0].paths.iter().any(|p| norm(p).contains("in_search")));
}

#[test]
fn search_unit_multiple_search_dirs() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let s1 = root.join("s1");
    let s2 = root.join("s2");
    let other = root.join("other");
    std::fs::create_dir_all(&s1).unwrap();
    std::fs::create_dir_all(&s2).unwrap();
    std::fs::create_dir_all(&other).unwrap();

    std::fs::write(s1.join("a.txt"), "x").unwrap();
    std::fs::write(other.join("a.txt"), "x").unwrap();

    std::fs::write(s2.join("b.txt"), "y").unwrap();
    std::fs::write(other.join("b.txt"), "y").unwrap();

    let mut config = ConfigFile::default();
    config.path_start = root.to_path_buf();
    config.compare_by_xxh3 = true;
    config.compare_by_size = true;
    config.search_dirs = vec![s1.clone(), s2.clone()];

    let scanner = FileScanner::new(&config, false);
    let groups = scanner.scan().unwrap();
    assert_eq!(groups.len(), 2);
}

#[test]
fn search_unit_search_dir_and_source_dir() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let source = root.join("source");
    let search = root.join("search");
    std::fs::create_dir_all(&source).unwrap();
    std::fs::create_dir_all(&search).unwrap();

    std::fs::write(source.join("keep.txt"), "dup").unwrap();
    std::fs::write(search.join("dup.txt"), "dup").unwrap();

    let mut config = ConfigFile::default();
    config.path_start = root.to_path_buf();
    config.compare_by_xxh3 = true;
    config.compare_by_size = true;
    config.source_dirs = vec![source.clone()];
    config.search_dirs = vec![search.clone()];

    let scanner = FileScanner::new(&config, false);
    let groups = scanner.scan().unwrap();
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].paths.len(), 2);
    // Source first (kept)
    assert!(norm(&groups[0].paths[0]).contains("source"));
    assert!(norm(&groups[0].paths[1]).contains("search"));
}

// ============= Integration =============

#[test]
fn integration_search_dir_only_reports_duplicates_in_search_dir() {
    let temp = TempDir::new().unwrap();
    temp.child("search_me").create_dir_all().unwrap();
    temp.child("ignore_me").create_dir_all().unwrap();

    temp.child("search_me").child("a.txt").write_str("same").unwrap();
    temp.child("ignore_me").child("a.txt").write_str("same").unwrap();

    temp.child("ignore_me").child("b.txt").write_str("also").unwrap();
    temp.child("b.txt").write_str("also").unwrap();

    let (stdout, stderr, status) = run_fundoubler(&[
        temp.path().to_str().unwrap(),
        "--md5",
        "--size",
        "--search-dir",
        temp.path().join("search_me").to_str().unwrap(),
    ]);

    assert!(status.success(), "stderr: {}", stderr);
    assert!(stdout.contains("search_me"), "Output should mention search_me");
    assert!(stdout.contains("a.txt"));
    // Only one duplicate group (a.txt pair); the b.txt pair is both outside search_me so must not be reported
    assert!(
        stdout.contains("Found 1 groups") || (stdout.contains("Group 1") && !stdout.contains("Group 2")),
        "Exactly one group (search_me/a.txt vs ignore_me/a.txt); b.txt pair should not be reported"
    );
}

#[test]
fn integration_search_dir_with_source_dir() {
    let temp = TempDir::new().unwrap();
    let source = temp.child("source");
    let search = temp.child("search");
    source.create_dir_all().unwrap();
    search.create_dir_all().unwrap();

    source.child("orig.txt").write_str("content").unwrap();
    search.child("copy.txt").write_str("content").unwrap();

    let (stdout, stderr, status) = run_fundoubler(&[
        temp.path().to_str().unwrap(),
        "--md5",
        "--size",
        "--source-dir",
        temp.path().join("source").to_str().unwrap(),
        "--search-dir",
        temp.path().join("search").to_str().unwrap(),
    ]);

    assert!(status.success(), "stderr: {}", stderr);
    assert!(stdout.contains("Found 1 groups") || stdout.contains("Group 1"));
    assert!(stdout.contains("source") && stdout.contains("search"));
}

#[test]
fn integration_search_dir_via_config_file() {
    let temp = TempDir::new().unwrap();
    let search_sub = temp.child("only_here");
    search_sub.create_dir_all().unwrap();
    temp.child("elsewhere").create_dir_all().unwrap();

    search_sub.child("f.txt").write_str("dup").unwrap();
    temp.child("elsewhere").child("f.txt").write_str("dup").unwrap();

    let config_path = temp.path().join("fundoubler.toml");
    std::fs::write(
        &config_path,
        r#"
path_start = "."
compare_by_size = true
compare_by_xxh3 = true
search_dirs = ["only_here"]
"#,
    )
    .unwrap();

    let (stdout, stderr, status) = run_fundoubler(&[
        "--config",
        config_path.to_str().unwrap(),
        temp.path().to_str().unwrap(),
    ]);

    assert!(status.success(), "stderr: {}", stderr);
    assert!(stdout.contains("only_here") && stdout.contains("f.txt"));
}
