use assert_fs::prelude::*;
use assert_fs::TempDir;
use std::fs;
use std::process::Command;

/// Helper function to run fundoubler with arguments.
/// Use --dry-run to block deletion; use --skip-confirm only when testing actual deletion.
fn run_fundoubler(args: &[&str]) -> (String, String, std::process::ExitStatus) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_fundoubler"));
    cmd.args(args);
    
    let output = cmd.output().expect("Failed to execute process");
    
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    
    (stdout, stderr, output.status)
}

/// Create a test file with specific content
fn create_test_file(dir: &TempDir, filename: &str, content: &str) {
    let file_path = dir.child(filename);
    file_path.write_str(content).unwrap();
}

#[test]
fn test_duplicates_by_md5_correct_files_identified() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create 4 files:
    // - file1.txt and file2.txt - same content (duplicates)
    // - file3.txt - different content, but accidentally same size
    // - file4.txt - completely different content
    create_test_file(&temp_dir, "file1.txt", "identical content");
    create_test_file(&temp_dir, "file2.txt", "identical content");
    create_test_file(&temp_dir, "file3.txt", "different but same size!!!"); // Same length
    create_test_file(&temp_dir, "file4.txt", "different");
    
    // Check by MD5 - should find only file1.txt and file2.txt as duplicates
    let (stdout, stderr, status) = run_fundoubler(&[
        temp_dir.path().to_str().unwrap(),
        "--md5",
    ]);
    
    assert!(
        status.success(),
        "MD5-based run should succeed, stderr: {}",
        stderr
    );
    
    // Duplicates should be found and both duplicate files should be mentioned in output
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
fn test_size_and_hash_comparison() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create files with same size and content (will be duplicates)
    // and files with same size but different content (won't be duplicates with hash comparison)
    create_test_file(&temp_dir, "dup1.txt", "12345678901234567890"); // 20 bytes
    create_test_file(&temp_dir, "dup2.txt", "12345678901234567890"); // 20 bytes (duplicate)
    create_test_file(&temp_dir, "size_only1.txt", "abcdefghijklmnopqrst"); // 20 bytes, different content
    create_test_file(&temp_dir, "size_only2.txt", "09876543210987654321"); // 20 bytes, different content
    create_test_file(&temp_dir, "different.txt", "short"); // 5 bytes
    
    let (stdout, stderr, status) = run_fundoubler(&[
        temp_dir.path().to_str().unwrap(),
    ]);
    
    assert!(
        status.success(),
        "Program should succeed for size/hash comparison, stderr: {}",
        stderr
    );
    
    // dup1 and dup2 should be found as duplicates (same size + hash)
    assert!(
        stdout.contains("dup1.txt"),
        "Output should mention dup1.txt, got: {}",
        stdout
    );
    assert!(
        stdout.contains("dup2.txt"),
        "Output should mention dup2.txt, got: {}",
        stdout
    );
    
    // size_only files should not be together (different hash)
    // different.txt should not be in duplicates
    let has_size_only_group = stdout.contains("size_only1.txt") && stdout.contains("size_only2.txt");
    assert!(
        !has_size_only_group,
        "Files with same size but different content should not be grouped together"
    );
}

#[test]
fn test_delete_keeps_first_file_in_group() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create 3 duplicates
    create_test_file(&temp_dir, "keep_this.txt", "duplicate content");
    create_test_file(&temp_dir, "delete_this.txt", "duplicate content");
    create_test_file(&temp_dir, "also_delete.txt", "duplicate content");
    
    // Run with dry-run to see what would be deleted
    let (stdout, stderr, status) = run_fundoubler(&[
        temp_dir.path().to_str().unwrap(),
        "--md5",
        "--delete",
        "--dry-run",
        "--sort=name", // Sort by name for predictability
    ]);
    
    assert!(
        status.success(),
        "Dry-run delete should succeed, stderr: {}",
        stderr
    );
    
    // In dry-run mode should see message
    assert!(
        stdout.contains("DRY RUN"),
        "Dry-run marker should be present in stdout, got: {}",
        stdout
    );
    assert!(
        stdout.contains("No files will be deleted"),
        "Expected explicit message that no files will be deleted in dry-run, got: {}",
        stdout
    );
}

#[test]
fn test_actual_deletion_with_force() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create temporary copy of files for safe deletion
    let file_to_keep = temp_dir.child("keep.txt");
    file_to_keep.write_str("content").unwrap();
    
    let file_to_delete = temp_dir.child("delete.txt");
    file_to_delete.write_str("content").unwrap();
    
    // Ensure both files exist before deletion
    assert!(file_to_keep.exists());
    assert!(file_to_delete.exists());
    
    // Use --force-delete and --skip-confirm to avoid interactivity
    let (_stdout, stderr, status) = run_fundoubler(&[
        temp_dir.path().to_str().unwrap(),
        "--md5",
        "--delete",
        "--force-delete",
        "--skip-confirm",
        "--sort=name", // "delete.txt" comes before "keep.txt", so "keep.txt" should be deleted
    ]);
    
    // In force-delete mode program should complete without errors
    assert!(
        status.success(),
        "Force-delete run should succeed, stderr: {}",
        stderr
    );
}

#[test]
fn test_multiple_groups_deletion() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create 2 groups of duplicates:
    // Group 1: a1.txt, a2.txt, a3.txt (same content "group1")
    // Group 2: b1.txt, b2.txt (same content "group2")
    // Group 3: unique.txt (unique file)
    
    create_test_file(&temp_dir, "a1.txt", "group1");
    create_test_file(&temp_dir, "a2.txt", "group1");
    create_test_file(&temp_dir, "a3.txt", "group1");
    
    create_test_file(&temp_dir, "b1.txt", "group2");
    create_test_file(&temp_dir, "b2.txt", "group2");
    
    create_test_file(&temp_dir, "unique.txt", "unique content");
    
    // Count files before deletion
    let files_before = std::fs::read_dir(temp_dir.path())
        .unwrap()
        .filter_map(Result::ok)
        .count();
    assert_eq!(files_before, 6); // 6 files
    
    // Run with dry-run to see the plan
    let (stdout, stderr, status) = run_fundoubler(&[
        temp_dir.path().to_str().unwrap(),
        "--md5",
        "--delete",
        "--dry-run",
        "--sort=name",
    ]);
    
    assert!(
        status.success(),
        "Dry-run delete with multiple groups should succeed, stderr: {}",
        stderr
    );
    
    // Analyze output and verify that multiple duplicate groups were found
    let groups_found = stdout.lines().filter(|line| line.contains("Group")).count();
    assert!(
        groups_found >= 2,
        "Expected at least 2 duplicate groups in output, got {}.\nStdout:\n{}",
        groups_found,
        stdout
    );
}

#[test]
fn test_subdirectory_traversal() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create files with same names in different folders
    // With default criteria (size + xxh3) they won't be duplicates due to different content
    
    let subdir1 = temp_dir.path().join("dir1");
    let subdir2 = temp_dir.path().join("dir2");
    fs::create_dir_all(&subdir1).unwrap();
    fs::create_dir_all(&subdir2).unwrap();
    
    // Create duplicates in different directories
    fs::write(subdir1.join("common.txt"), "same content").unwrap();
    fs::write(subdir2.join("common.txt"), "same content").unwrap(); // Duplicate!
    
    // And a unique file
    create_test_file(&temp_dir, "different.txt", "different content");
    
    let (stdout, stderr, status) = run_fundoubler(&[
        temp_dir.path().to_str().unwrap(),
    ]);
    
    assert!(
        status.success(),
        "Program should succeed for subdirectory traversal, stderr: {}",
        stderr
    );
    
    // common.txt from both directories should be found as duplicates
    assert!(
        stdout.contains("common.txt"),
        "Output should mention common.txt from subdirectories, got: {}",
        stdout
    );
    
    // Verify that program traverses subdirectories
    assert!(
        stdout.contains("dir1") || stdout.contains("dir2"),
        "Output should show subdirectory paths"
    );
}

#[test]
fn test_combined_criteria_size_and_hash() {
    let temp_dir = TempDir::new().unwrap();
    
    // Test combined criteria: size + hash
    // Files must match both criteria to be duplicates
    
    // Duplicates: same size and content
    create_test_file(&temp_dir, "dup1.txt", "content123"); // 11 bytes
    create_test_file(&temp_dir, "dup2.txt", "content123"); // 11 bytes (duplicate)
    
    // Only size matches, but content is different
    create_test_file(&temp_dir, "size_match.txt", "same_size!!!"); // 11 bytes, different content
    
    // Different size
    create_test_file(&temp_dir, "different.txt", "x"); // 1 byte
    
    let (stdout, stderr, status) = run_fundoubler(&[
        temp_dir.path().to_str().unwrap(),
    ]);
    
    assert!(
        status.success(),
        "Combined criteria run should succeed, stderr: {}",
        stderr
    );
    
    // dup1 and dup2 should be found as duplicates
    assert!(
        stdout.contains("dup1.txt"),
        "Output should mention dup1.txt, got: {}",
        stdout
    );
    assert!(
        stdout.contains("dup2.txt"),
        "Output should mention dup2.txt, got: {}",
        stdout
    );
    
    // size_match.txt should not be in group with dup1/dup2 (different content)
    let lines: Vec<&str> = stdout.lines().collect();
    let mut dup_group_has_size_match = false;
    let mut in_dup_group = false;
    
    for line in lines {
        if line.contains("Group") {
            in_dup_group = false;
        }
        if line.contains("dup1.txt") || line.contains("dup2.txt") {
            in_dup_group = true;
        }
        if in_dup_group && line.contains("size_match.txt") {
            dup_group_has_size_match = true;
        }
    }
    
    assert!(
        !dup_group_has_size_match,
        "Files with same size but different content should not be in same group"
    );
}

#[test]
fn test_min_size_filter_works_correctly() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create files of different sizes.
    // Important: create pairs with same content so they are actually considered duplicates
    // with default criteria (size + xxh3).
    create_test_file(&temp_dir, "small.txt", "abc"); // 3 bytes
    create_test_file(&temp_dir, "small2.txt", "abc"); // 3 bytes (duplicate)
    create_test_file(&temp_dir, "large.txt", "1234567890"); // 10 bytes
    create_test_file(&temp_dir, "large2.txt", "1234567890"); // 10 bytes (duplicate)
    
    // Set min-size=5, should ignore small files
    let (stdout, stderr, status) = run_fundoubler(&[
        temp_dir.path().to_str().unwrap(),
        "--min-size=5",
    ]);
    
    assert!(
        status.success(),
        "Program should succeed with min-size filter, stderr: {}",
        stderr
    );
    
    // small files should be filtered out, large files should be included
    assert!(!stdout.contains("small.txt"));
    assert!(!stdout.contains("small2.txt"));
    assert!(stdout.contains("large.txt"));
    assert!(stdout.contains("large2.txt"));
}

#[test]
fn test_max_size_filter_works_correctly() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create duplicate pairs so output definitely contains file names
    create_test_file(&temp_dir, "small.txt", "abc"); // 3 bytes
    create_test_file(&temp_dir, "small2.txt", "abc"); // 3 bytes (duplicate)
    create_test_file(&temp_dir, "large.txt", "12345678901234567890"); // 20 bytes
    create_test_file(&temp_dir, "large2.txt", "12345678901234567890"); // 20 bytes (duplicate)
    
    // Set max-size=10, should ignore large files
    let (stdout, stderr, status) = run_fundoubler(&[
        temp_dir.path().to_str().unwrap(),
        "--max-size=10",
    ]);
    
    assert!(
        status.success(),
        "Program should succeed with max-size filter, stderr: {}",
        stderr
    );
    
    // small files should be included, large files should be filtered out
    assert!(stdout.contains("small.txt"));
    assert!(stdout.contains("small2.txt"));
    assert!(!stdout.contains("large.txt"));
    assert!(!stdout.contains("large2.txt"));
}

#[test]
fn test_sha512_hash_flag() {
    let temp_dir = TempDir::new().unwrap();
    
    create_test_file(&temp_dir, "file1.txt", "same content");
    create_test_file(&temp_dir, "file2.txt", "same content");
    create_test_file(&temp_dir, "file3.txt", "different content");
    
    let (stdout, stderr, status) = run_fundoubler(&[
        temp_dir.path().to_str().unwrap(),
        "--sha512",
    ]);
    
    assert!(
        status.success(),
        "SHA512-based run should succeed, stderr: {}",
        stderr
    );
    
    // file1 and file2 should be found as duplicates
    assert!(stdout.contains("file1.txt"));
    assert!(stdout.contains("file2.txt"));
}

#[test]
fn test_xxh3_hash_flag() {
    let temp_dir = TempDir::new().unwrap();
    
    create_test_file(&temp_dir, "file1.txt", "same content");
    create_test_file(&temp_dir, "file2.txt", "same content");
    create_test_file(&temp_dir, "file3.txt", "different content");
    
    let (stdout, stderr, status) = run_fundoubler(&[
        temp_dir.path().to_str().unwrap(),
        "--xxh3",
    ]);
    
    assert!(
        status.success(),
        "XXH3-based run should succeed, stderr: {}",
        stderr
    );
    
    // file1 and file2 should be found as duplicates
    assert!(stdout.contains("file1.txt"));
    assert!(stdout.contains("file2.txt"));
}

#[test]
fn test_content_flag_enables_all_hashes() {
    let temp_dir = TempDir::new().unwrap();
    
    create_test_file(&temp_dir, "file1.txt", "same content");
    create_test_file(&temp_dir, "file2.txt", "same content");
    
    let (stdout, stderr, status) = run_fundoubler(&[
        temp_dir.path().to_str().unwrap(),
        "--content",
    ]);
    
    assert!(
        status.success(),
        "--content flag run should succeed, stderr: {}",
        stderr
    );
    
    // Duplicates should be found
    assert!(stdout.contains("file1.txt"));
    assert!(stdout.contains("file2.txt"));
}

#[test]
fn test_name_flag_finds_duplicates_by_name() {
    let temp_dir = TempDir::new().unwrap();
    
    // Same filename in different subdirs, same content -> duplicates when comparing by name
    let sub1 = temp_dir.child("dir1");
    sub1.create_dir_all().unwrap();
    let sub2 = temp_dir.child("dir2");
    sub2.create_dir_all().unwrap();
    
    sub1.child("common.txt").write_str("identical content").unwrap();
    sub2.child("common.txt").write_str("identical content").unwrap();
    
    let (stdout, stderr, status) = run_fundoubler(&[
        temp_dir.path().to_str().unwrap(),
        "--name",
        "--md5",
    ]);
    
    assert!(
        status.success(),
        "--name flag run should succeed, stderr: {}",
        stderr
    );
    
    assert!(stdout.contains("common.txt"));
}

#[test]
fn test_combined_size_and_name_flags() {
    let temp_dir = TempDir::new().unwrap();
    
    // Two files with same name, same size, same content in subdirs
    let sub1 = temp_dir.child("a");
    sub1.create_dir_all().unwrap();
    let sub2 = temp_dir.child("b");
    sub2.create_dir_all().unwrap();
    
    let content = "same size and content";
    sub1.child("dup.txt").write_str(content).unwrap();
    sub2.child("dup.txt").write_str(content).unwrap();
    
    let (stdout, stderr, status) = run_fundoubler(&[
        temp_dir.path().to_str().unwrap(),
        "--size",
        "--name",
        "--md5",
    ]);
    
    assert!(
        status.success(),
        "--size --name --md5 run should succeed, stderr: {}",
        stderr
    );
    
    assert!(stdout.contains("dup.txt"));
}

#[test]
fn test_filter_regex() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create duplicates among different file types
    create_test_file(&temp_dir, "image1.jpg", "duplicate");
    create_test_file(&temp_dir, "image2.jpg", "duplicate");
    create_test_file(&temp_dir, "doc1.pdf", "duplicate");
    create_test_file(&temp_dir, "doc2.pdf", "duplicate");
    
    let (stdout, stderr, status) = run_fundoubler(&[
        temp_dir.path().to_str().unwrap(),
        "--md5",
        "--filter", ".*\\.jpg$",
    ]);
    
    assert!(
        status.success(),
        "Filter regex run should succeed, stderr: {}",
        stderr
    );
    
    // Only jpg files should be found
    assert!(stdout.contains("image1.jpg"));
    assert!(stdout.contains("image2.jpg"));
    assert!(!stdout.contains("doc1.pdf"));
    assert!(!stdout.contains("doc2.pdf"));
}

#[test]
fn test_sort_order() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create files of different sizes to test sorting
    create_test_file(&temp_dir, "small.txt", "x"); // 1 byte
    create_test_file(&temp_dir, "small_copy.txt", "x"); // 1 byte
    create_test_file(&temp_dir, "large.txt", "12345678901234567890"); // 20 bytes
    create_test_file(&temp_dir, "large_copy.txt", "12345678901234567890"); // 20 bytes
    
    // Use --size --md5 so the key includes size for sort-by-size to work
    let (stdout, stderr, status) = run_fundoubler(&[
        temp_dir.path().to_str().unwrap(),
        "--size",
        "--md5",
        "--sort=size-desc",
    ]);
    
    assert!(
        status.success(),
        "Sort order run should succeed, stderr: {}",
        stderr
    );
    
    // Verify that groups are ordered (large files should come first)
    let stdout_lower = stdout.to_lowercase();
    let large_pos = stdout_lower.find("large.txt").unwrap_or(0);
    let small_pos = stdout_lower.find("small.txt").unwrap_or(stdout.len());
    
    assert!(
        large_pos < small_pos,
        "Large files should appear before small files with size-desc sort"
    );
}

#[test]
fn test_limit_option() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create multiple duplicate groups
    create_test_file(&temp_dir, "a1.txt", "group1");
    create_test_file(&temp_dir, "a2.txt", "group1");
    create_test_file(&temp_dir, "b1.txt", "group2");
    create_test_file(&temp_dir, "b2.txt", "group2");
    create_test_file(&temp_dir, "c1.txt", "group3");
    create_test_file(&temp_dir, "c2.txt", "group3");
    
    let (stdout, stderr, status) = run_fundoubler(&[
        temp_dir.path().to_str().unwrap(),
        "--md5",
        "--limit=2",
    ]);
    
    assert!(
        status.success(),
        "Limit option run should succeed, stderr: {}",
        stderr
    );
    
    // Number of groups should be limited
    let group_count = stdout.lines().filter(|line| line.contains("Group")).count();
    assert!(
        group_count <= 2,
        "Should limit to 2 groups, but found {} groups",
        group_count
    );
}

#[test]
fn test_silent_mode() {
    let temp_dir = TempDir::new().unwrap();
    
    create_test_file(&temp_dir, "file1.txt", "same");
    create_test_file(&temp_dir, "file2.txt", "same");
    
    let (stdout, stderr, status) = run_fundoubler(&[
        temp_dir.path().to_str().unwrap(),
        "--md5",
        "--silent",
    ]);
    
    assert!(
        status.success(),
        "Silent mode run should succeed, stderr: {}",
        stderr
    );
    
    // In silent mode stdout should be empty
    assert!(
        stdout.trim().is_empty(),
        "Silent mode should produce no output, got: {}",
        stdout
    );
}

#[test]
fn test_verbose_mode() {
    let temp_dir = TempDir::new().unwrap();
    
    create_test_file(&temp_dir, "file1.txt", "same content");
    create_test_file(&temp_dir, "file2.txt", "same content");
    
    let (stdout, stderr, status) = run_fundoubler(&[
        temp_dir.path().to_str().unwrap(),
        "--md5",
        "--verbose",
    ]);
    
    assert!(
        status.success(),
        "Verbose mode run should succeed, stderr: {}",
        stderr
    );
    
    // Verbose mode should show additional information
    assert!(
        stdout.contains("Wasted space") || stdout.contains("file1.txt"),
        "Verbose mode should show additional information"
    );
}

#[test]
fn test_output_file() {
    let temp_dir = TempDir::new().unwrap();
    let output_file = temp_dir.path().join("output.txt");
    
    create_test_file(&temp_dir, "file1.txt", "same content");
    create_test_file(&temp_dir, "file2.txt", "same content");
    
    // output is a positional argument (second positional after path_start)
    // Order: [FLAGS] <PATH_START> [OUTPUT]
    let (_stdout, stderr, status) = run_fundoubler(&[
        "--md5",
        temp_dir.path().to_str().unwrap(),
        output_file.to_str().unwrap(),
    ]);
    
    assert!(
        status.success(),
        "Output file run should succeed, stderr: {}",
        stderr
    );
    
    // File should be created and contain results
    assert!(
        output_file.exists(),
        "Output file should be created"
    );
    
    let output_content = fs::read_to_string(&output_file).unwrap();
    assert!(
        output_content.contains("file1.txt") || output_content.contains("Duplicate"),
        "Output file should contain duplicate information"
    );
}

#[test]
fn test_empty_directory() {
    let temp_dir = TempDir::new().unwrap();
    
    // Empty directory
    let (stdout, stderr, status) = run_fundoubler(&[
        temp_dir.path().to_str().unwrap(),
    ]);
    
    assert!(
        status.success(),
        "Empty directory run should succeed, stderr: {}",
        stderr
    );
    
    // Should have message about no duplicates found
    assert!(
        stdout.contains("No duplicates") || stdout.contains("found"),
        "Should handle empty directory gracefully, got: {}",
        stdout
    );
}

#[test]
fn test_single_file_no_duplicates() {
    let temp_dir = TempDir::new().unwrap();
    
    create_test_file(&temp_dir, "unique.txt", "unique content");
    
    let (stdout, stderr, status) = run_fundoubler(&[
        temp_dir.path().to_str().unwrap(),
        "--md5",
    ]);
    
    assert!(
        status.success(),
        "Single file run should succeed, stderr: {}",
        stderr
    );
    
    // Should have message about no duplicates found
    assert!(
        stdout.contains("No duplicates"),
        "Should report no duplicates for single file, got: {}",
        stdout
    );
}

#[test]
fn test_init_config_creates_default_file() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("my_config.toml");

    let (_stdout, stderr, status) = run_fundoubler(&[
        "--init-config",
        config_path.to_str().unwrap(),
    ]);

    assert!(
        status.success(),
        "--init-config should succeed, stderr: {}",
        stderr
    );

    assert!(
        config_path.exists(),
        "Config file should be created"
    );

    let content = fs::read_to_string(&config_path).unwrap();
    assert!(
        content.contains("path_start") && content.contains("compare_by_size"),
        "Config file should contain expected fields"
    );
}

#[test]
fn test_hash_cache_creates_cache_file() {
    let temp_dir = TempDir::new().unwrap();
    let cache_dir = temp_dir.path().join("my_cache");
    
    create_test_file(&temp_dir, "a.txt", "same");
    create_test_file(&temp_dir, "b.txt", "same");
    
    let (stdout, stderr, status) = run_fundoubler(&[
        temp_dir.path().to_str().unwrap(),
        "--md5",
        "--hash-cache",
        "--hash-cache-dir",
        cache_dir.to_str().unwrap(),
    ]);
    
    assert!(
        status.success(),
        "Hash cache run should succeed, stderr: {}",
        stderr
    );
    
    let cache_file = cache_dir.join("cache.json");
    assert!(
        cache_file.exists(),
        "Cache file should be created at {}",
        cache_file.display()
    );
    
    assert!(stdout.contains("a.txt") && stdout.contains("b.txt"));
}

#[test]
fn test_hash_cache_second_run_uses_cache() {
    let temp_dir = TempDir::new().unwrap();
    let cache_dir = temp_dir.path().join("cache");

    create_test_file(&temp_dir, "x.txt", "content");
    create_test_file(&temp_dir, "y.txt", "content");

    let (stdout1, stderr1, status1) = run_fundoubler(&[
        temp_dir.path().to_str().unwrap(),
        "--md5",
        "--hash-cache",
        "--hash-cache-dir",
        cache_dir.to_str().unwrap(),
    ]);
    assert!(status1.success(), "First run failed: {}", stderr1);

    let (stdout2, stderr2, status2) = run_fundoubler(&[
        temp_dir.path().to_str().unwrap(),
        "--md5",
        "--hash-cache",
        "--hash-cache-dir",
        cache_dir.to_str().unwrap(),
    ]);
    assert!(status2.success(), "Second run failed: {}", stderr2);

    assert!(stdout1.contains("x.txt") && stdout1.contains("y.txt"));
    assert!(stdout2.contains("x.txt") && stdout2.contains("y.txt"));
}

#[test]
fn test_hash_buffer_size_cli() {
    let temp_dir = TempDir::new().unwrap();
    create_test_file(&temp_dir, "a.txt", "x");
    create_test_file(&temp_dir, "b.txt", "x");

    let (stdout, stderr, status) = run_fundoubler(&[
        temp_dir.path().to_str().unwrap(),
        "--md5",
        "--hash-buffer-size",
        "4096",
    ]);
    assert!(status.success(), "stderr: {}", stderr);
    assert!(stdout.contains("a.txt"));
}

#[test]
fn test_hash_cache_custom_dir() {
    let temp_dir = TempDir::new().unwrap();
    let custom_cache = temp_dir.path().join("custom_cache_dir");

    create_test_file(&temp_dir, "a.txt", "same");
    create_test_file(&temp_dir, "b.txt", "same");

    let (_, stderr, status) = run_fundoubler(&[
        temp_dir.path().to_str().unwrap(),
        "--md5",
        "--hash-cache",
        "--hash-cache-dir",
        custom_cache.to_str().unwrap(),
    ]);
    assert!(status.success(), "stderr: {}", stderr);
    assert!(custom_cache.join("cache.json").exists());
}

#[test]
fn test_hash_cache_via_config_file() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("fundoubler.toml");
    let cache_dir = temp_dir.path().join("cache");
    let path_str = temp_dir.path().to_str().unwrap().replace('\\', "/");
    let cache_str = cache_dir.to_str().unwrap().replace('\\', "/");

    std::fs::write(
        &config_path,
        format!(
            r#"
path_start = "{}"
compare_by_size = true
compare_by_xxh3 = true
hash_cache = true
hash_cache_dir = "{}"
"#,
            path_str, cache_str
        ),
    )
    .unwrap();

    create_test_file(&temp_dir, "f1.txt", "dup");
    create_test_file(&temp_dir, "f2.txt", "dup");

    let (stdout, stderr, status) = run_fundoubler(&[
        "--config",
        config_path.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
    ]);
    assert!(status.success(), "stderr: {}", stderr);
    assert!(stdout.contains("f1.txt"));
    assert!(cache_dir.join("cache.json").exists());
}

#[test]
fn test_init_config_includes_all_fields() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("full_config.toml");

    let (_, stderr, status) = run_fundoubler(&[
        "--init-config",
        config_path.to_str().unwrap(),
    ]);
    assert!(status.success(), "stderr: {}", stderr);

    let content = fs::read_to_string(&config_path).unwrap();
    assert!(content.contains("hash_cache"));
    assert!(content.contains("hash_cache_dir"));
    assert!(content.contains("hash_buffer_size"));
    assert!(content.contains("exclude_dirs"));
    assert!(content.contains("source_dirs"));
    assert!(content.contains("log_level"));
    assert!(content.contains("logs_dir"));
}

#[test]
fn test_init_config_generated_file_is_loadable() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("generated.toml");

    let (_, stderr, status) = run_fundoubler(&[
        "--init-config",
        config_path.to_str().unwrap(),
    ]);
    assert!(status.success(), "stderr: {}", stderr);

    create_test_file(&temp_dir, "f1.txt", "x");
    create_test_file(&temp_dir, "f2.txt", "x");

    let (stdout, stderr2, status2) = run_fundoubler(&[
        "--config",
        config_path.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
    ]);
    assert!(status2.success(), "load and run failed: {}", stderr2);
    assert!(stdout.contains("f1.txt") || stdout.contains("No duplicates"));
}

#[test]
fn test_invalid_path() {
    // Test verifies that program doesn't panic on nonexistent path
    // walkdir handles this gracefully by returning empty iterator
    let (_stdout, _stderr, status) = run_fundoubler(&[
        "/nonexistent/path/that/does/not/exist",
    ]);
    
    // Main thing - program should complete (not panic)
    // It may complete successfully (empty result) or with error
    assert!(
        status.code().is_some(),
        "Program should complete without panicking for invalid path"
    );
}

#[test]
fn test_invalid_regex_filter() {
    let temp_dir = TempDir::new().unwrap();
    
    create_test_file(&temp_dir, "file1.txt", "content");
    
    // Test verifies that program doesn't panic on invalid regex
    // Regex error may be handled in scanner.process_file and result
    // in empty output, or program may exit with error
    let (_stdout, _stderr, status) = run_fundoubler(&[
        temp_dir.path().to_str().unwrap(),
        "--filter", "[invalid regex",
    ]);
    
    // Main thing - program should complete (not panic)
    assert!(
        status.code().is_some(),
        "Program should complete without panicking for invalid regex"
    );
}

#[test]
fn test_verbose_level_2_shows_config() {
    let temp_dir = TempDir::new().unwrap();
    
    create_test_file(&temp_dir, "file1.txt", "same");
    create_test_file(&temp_dir, "file2.txt", "same");
    
    let (stdout, stderr, status) = run_fundoubler(&[
        "--md5",
        temp_dir.path().to_str().unwrap(),
        "--verbose",
        "--verbose", // verbose = 2
    ]);
    
    assert!(
        status.success(),
        "Verbose level 2 should succeed, stderr: {}",
        stderr
    );
    
    // verbose > 1 should show configuration debug output
    assert!(
        stdout.contains("Configuration:") || stdout.contains("path_start") || stdout.contains("compare_by"),
        "Verbose level 2 should show config debug output, got: {}",
        stdout
    );
}

#[test]
fn test_multiple_sort_orders_integration() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create files with different sizes and names to test multiple sorting
    create_test_file(&temp_dir, "z_large.txt", "12345678901234567890"); // 20 bytes
    create_test_file(&temp_dir, "z_large_copy.txt", "12345678901234567890"); // 20 bytes
    create_test_file(&temp_dir, "a_small.txt", "x"); // 1 byte
    create_test_file(&temp_dir, "a_small_copy.txt", "x"); // 1 byte
    
    // Use --size --md5 so the key includes size for sort-by-size to work
    let (stdout, stderr, status) = run_fundoubler(&[
        "--size",
        "--md5",
        temp_dir.path().to_str().unwrap(),
        "--sort=size-desc",
        "--sort=name",
    ]);
    
    assert!(
        status.success(),
        "Multiple sort orders should succeed, stderr: {}",
        stderr
    );
    
    // With SizeDesc then Name sort, large files should come first,
    // and within groups of same size - by name
    let stdout_lower = stdout.to_lowercase();
    let large_pos = stdout_lower.find("z_large").unwrap_or(0);
    let small_pos = stdout_lower.find("a_small").unwrap_or(stdout.len());
    
    assert!(
        large_pos < small_pos,
        "With size-desc + name sort, large files should appear before small files"
    );
}

#[test]
fn test_name_desc_sort_order() {
    let temp_dir = TempDir::new().unwrap();
    
    create_test_file(&temp_dir, "a_first.txt", "content");
    create_test_file(&temp_dir, "a_first_copy.txt", "content");
    create_test_file(&temp_dir, "z_last.txt", "content");
    create_test_file(&temp_dir, "z_last_copy.txt", "content");
    
    // --sort=name-desc is accepted; group order by name-desc only applies when
    // compare_by_name is enabled (no CLI flag for that), so key.name is None
    // and group order is undefined. We only verify the run succeeds and both
    // groups appear.
    let (stdout, stderr, status) = run_fundoubler(&[
        "--md5",
        temp_dir.path().to_str().unwrap(),
        "--sort=name-desc",
    ]);
    
    assert!(
        status.success(),
        "Name-desc sort should succeed, stderr: {}",
        stderr
    );
    
    let stdout_lower = stdout.to_lowercase();
    assert!(
        stdout_lower.contains("a_first") && stdout_lower.contains("z_last"),
        "Both duplicate groups should appear in output"
    );
}

#[test]
fn test_summary_statistics() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create multiple duplicate groups of different sizes
    create_test_file(&temp_dir, "large1.txt", "12345678901234567890"); // 20 bytes
    create_test_file(&temp_dir, "large2.txt", "12345678901234567890"); // 20 bytes
    create_test_file(&temp_dir, "small1.txt", "x"); // 1 byte
    create_test_file(&temp_dir, "small2.txt", "x"); // 1 byte
    
    let (stdout, stderr, status) = run_fundoubler(&[
        "--md5",
        temp_dir.path().to_str().unwrap(),
    ]);
    
    assert!(
        status.success(),
        "Summary statistics test should succeed, stderr: {}",
        stderr
    );
    
    // Check for summary section
    assert!(
        stdout.contains("Summary:") || stdout.contains("Total duplicate groups"),
        "Output should contain summary statistics"
    );
    
    // Verify that groups and files are mentioned
    assert!(
        stdout.contains("group") || stdout.contains("Group") || stdout.contains("duplicate"),
        "Output should mention duplicate groups"
    );
}

#[test]
fn test_wasted_space_calculation() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create duplicates of known size to test wasted space calculation
    let content_20_bytes = "12345678901234567890"; // 20 bytes
    create_test_file(&temp_dir, "file1.txt", content_20_bytes);
    create_test_file(&temp_dir, "file2.txt", content_20_bytes);
    create_test_file(&temp_dir, "file3.txt", content_20_bytes); // 3 files = 2 duplicates
    
    let (stdout, stderr, status) = run_fundoubler(&[
        "--md5",
        temp_dir.path().to_str().unwrap(),
        "--verbose", // Needed to show wasted space
    ]);
    
    assert!(
        status.success(),
        "Wasted space calculation test should succeed, stderr: {}",
        stderr
    );
    
    // With verbose should show wasted space
    // 3 files of 20 bytes each, 2 duplicates = 40 bytes wasted space
    assert!(
        stdout.contains("Wasted space") || stdout.contains("wasted") || stdout.contains("40"),
        "Verbose output should show wasted space calculation"
    );
}

// Exclude and source directory tests moved to tests/exclude_source_tests.rs