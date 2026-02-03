use assert_fs::TempDir;
use std::fs;
use std::process::Command;

fn run_fundoubler(args: &[&str]) -> (String, String, std::process::ExitStatus) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_fundoubler"));
    cmd.args(args);

    let output = cmd.output().expect("Failed to execute process");

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    (stdout, stderr, output.status)
}

#[test]
fn test_basic_duplicate_detection() {
    let temp_dir = TempDir::new().unwrap();

    // Create duplicates
    fs::write(temp_dir.path().join("file1.txt"), "same content").unwrap();
    fs::write(temp_dir.path().join("file2.txt"), "same content").unwrap();
    fs::write(temp_dir.path().join("unique.txt"), "different content").unwrap();

    let (stdout, stderr, status) = run_fundoubler(&[temp_dir.path().to_str().unwrap(), "--md5"]);

    // Program should complete successfully
    assert!(
        status.success(),
        "Program should succeed, stderr: {}",
        stderr
    );

    // Both duplicate files should be displayed in output
    assert!(
        stdout.contains("file1.txt"),
        "Output should mention file1.txt, got: {}",
        stdout
    );
    assert!(
        stdout.contains("file2.txt"),
        "Output should mention file2.txt, got: {}",
        stdout
    );
}

#[test]
fn test_dry_run_mode() {
    let temp_dir = TempDir::new().unwrap();

    fs::write(temp_dir.path().join("a.txt"), "content").unwrap();
    fs::write(temp_dir.path().join("b.txt"), "content").unwrap();

    let (stdout, stderr, status) = run_fundoubler(&[
        temp_dir.path().to_str().unwrap(),
        "--md5",
        "--delete",
        "--dry-run",
    ]);

    // Program should complete successfully
    assert!(
        status.success(),
        "Program should succeed in dry-run mode, stderr: {}",
        stderr
    );

    // Should have explicit dry-run message
    assert!(
        stdout.contains("DRY RUN"),
        "Dry run marker should be present in stdout, got: {}",
        stdout
    );

    // Files should not be deleted in dry-run mode
    assert!(
        temp_dir.path().join("a.txt").exists(),
        "a.txt should not be deleted in dry-run mode"
    );
    assert!(
        temp_dir.path().join("b.txt").exists(),
        "b.txt should not be deleted in dry-run mode"
    );
}

#[test]
fn test_size_comparison() {
    let temp_dir = TempDir::new().unwrap();

    // Create files of same size and content (so they are duplicates
    // with default criteria: size + xxh3)
    fs::write(temp_dir.path().join("size1.txt"), "12345").unwrap(); // 5 bytes
    fs::write(temp_dir.path().join("size2.txt"), "12345").unwrap(); // 5 bytes (duplicate)
    fs::write(temp_dir.path().join("diff.txt"), "1").unwrap(); // 1 byte

    let (stdout, stderr, status) = run_fundoubler(&[temp_dir.path().to_str().unwrap()]);

    // Program should complete successfully
    assert!(
        status.success(),
        "Program should succeed for size/hash comparison, stderr: {}",
        stderr
    );

    // Duplicate files should be found
    assert!(
        stdout.contains("size1.txt"),
        "Output should mention size1.txt, got: {}",
        stdout
    );
    assert!(
        stdout.contains("size2.txt"),
        "Output should mention size2.txt, got: {}",
        stdout
    );

    // File of different size should not appear among duplicates
    assert!(
        !stdout.contains("diff.txt") || stdout.contains("No duplicates"),
        "diff.txt should not be treated as duplicate: {}",
        stdout
    );
}
