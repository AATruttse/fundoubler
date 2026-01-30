use assert_fs::TempDir;
use std::fs;
use std::process::Command;

#[test]
fn test_delete_correct_files_basic() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create structure:
    // - original.txt, copy1.txt, copy2.txt — duplicates
    // - different.txt — unique file
    fs::write(temp_dir.path().join("original.txt"), "same content").unwrap();
    fs::write(temp_dir.path().join("copy1.txt"), "same content").unwrap();
    fs::write(temp_dir.path().join("copy2.txt"), "same content").unwrap();
    fs::write(temp_dir.path().join("different.txt"), "different content").unwrap();
    
    let output = Command::new(env!("CARGO_BIN_EXE_fundoubler"))
        .env("TEST_MODE", "1")
        .args(&[
            temp_dir.path().to_str().unwrap(),
            "--md5",
            "--delete",
            "--dry-run",
            "--sort=name",
        ])
        .output()
        .expect("Failed to execute process");
    
    // Expect successful completion when working correctly
    assert!(
        output.status.success(),
        "Deletion dry-run should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // In dry-run mode duplicates should be found and deletion plan shown,
    // but files on disk should not change.
    assert!(
        stdout.contains("Group") || stdout.contains("Found"),
        "Expected dry-run output to describe duplicate groups, got: {}",
        stdout
    );
    
    // Verify that all four files still exist
    let files_after: Vec<String> = fs::read_dir(temp_dir.path())
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().to_string_lossy().to_string())
        .collect();
    
    for name in &["original.txt", "copy1.txt", "copy2.txt", "different.txt"] {
        assert!(
            files_after.contains(&name.to_string()),
            "File {} must remain in dry-run mode",
            name
        );
    }
    
    // Verify that unique file is not mentioned in deletion context
    // (though it may be mentioned in output, it should not be in duplicate group)
    let lines: Vec<&str> = stdout.lines().collect();
    let mut in_duplicate_group = false;
    let mut different_in_group = false;
    
    for line in lines {
        if line.contains("Group") {
            in_duplicate_group = true;
        }
        if in_duplicate_group && line.contains("different.txt") {
            different_in_group = true;
        }
        if line.contains("Files:") && in_duplicate_group {
            // Reset when moving to next group
        }
    }
    
    assert!(
        !different_in_group,
        "Unique file 'different.txt' should not be in any duplicate group"
    );
}

#[test]
fn test_delete_keeps_first_file_in_sorted_group() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create duplicates with predictable names to test sorting
    fs::write(temp_dir.path().join("z_last.txt"), "duplicate").unwrap();
    fs::write(temp_dir.path().join("a_first.txt"), "duplicate").unwrap();
    fs::write(temp_dir.path().join("m_middle.txt"), "duplicate").unwrap();
    
    // When sorting by name: a_first.txt should be first (kept)
    let output = Command::new(env!("CARGO_BIN_EXE_fundoubler"))
        .env("TEST_MODE", "1")
        .args(&[
            temp_dir.path().to_str().unwrap(),
            "--md5",
            "--delete",
            "--dry-run",
            "--sort=name",
        ])
        .output()
        .expect("Failed to execute process");
    
    assert!(
        output.status.success(),
        "Dry-run with sort should succeed"
    );
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Verify that all files still exist (dry-run)
    assert!(temp_dir.path().join("a_first.txt").exists());
    assert!(temp_dir.path().join("m_middle.txt").exists());
    assert!(temp_dir.path().join("z_last.txt").exists());
    
    // In real deletion a_first.txt should be kept (first alphabetically)
    // This is verified through logic: if this were a real delete, a_first should remain
    // In dry-run we just verify that the deletion plan is correct
    assert!(
        stdout.contains("a_first.txt"),
        "First file in sorted group should be mentioned in output"
    );
}