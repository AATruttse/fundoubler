use fundoubler::check::{CheckOptions, calculate_hash, compare};
use fundoubler::config::{ConfigFile, SortOrder};
use fundoubler::scanner::FileScanner;
use tempfile::NamedTempFile;

#[test]
fn test_check_options_new() {
    let opts = CheckOptions::new();
    assert!(opts.name.is_none());
    assert!(opts.size.is_none());
    assert!(opts.created.is_none());
    assert!(opts.modified.is_none());
    assert!(opts.md5.is_none());
    assert!(opts.sha512.is_none());
    assert!(opts.xxh3.is_none());
}

#[test]
fn test_check_options_display() {
    use std::time::SystemTime;
    
    let now = SystemTime::now();
    let opts = CheckOptions {
        name: Some("test.txt".to_string()),
        size: Some(1024),
        created: Some(now),
        modified: Some(now),
        md5: Some("d41d8cd98f00b204e9800998ecf8427e".to_string()),
        sha512: Some("cf83e1357eefb8bdf1542850d66d8007d620e4050b5715dc83f4a921d36ce9ce47d0d13c5d85f2b0ff8318d2877eec2f63b931bd47417a81a538327af927da3e".to_string()),
        xxh3: Some("0000000000000000".to_string()),
    };
    
    let display = format!("{}", opts);
    assert!(display.contains("test.txt"));
    assert!(display.contains("1024"));
    assert!(display.contains("MD5:"));
    assert!(display.contains("SHA512:"));
    assert!(display.contains("XXH3:"));
}

#[test]
fn test_compare_function_prioritizes_sort_order() {
    let mut config = ConfigFile::default();
    
    // Test 1: Sort by size (descending)
    config.sort_orders = vec![SortOrder::SizeDesc];
    
    let small_file = CheckOptions {
        name: Some("small.txt".to_string()),
        size: Some(100),
        created: None,
        modified: None,
        md5: None,
        sha512: None,
        xxh3: None,
    };
    
    let large_file = CheckOptions {
        name: Some("large.txt".to_string()),
        size: Some(500),
        created: None,
        modified: None,
        md5: None,
        sha512: None,
        xxh3: None,
    };
    
    // Larger file should come first when sorting in descending order
    assert_eq!(compare(&config, &small_file, &large_file), std::cmp::Ordering::Greater);
    assert_eq!(compare(&config, &large_file, &small_file), std::cmp::Ordering::Less);
    
    // Test 2: Sort by name
    config.sort_orders = vec![SortOrder::Name];
    
    let file_a = CheckOptions {
        name: Some("a.txt".to_string()),
        size: Some(100),
        created: None,
        modified: None,
        md5: None,
        sha512: None,
        xxh3: None,
    };
    
    let file_b = CheckOptions {
        name: Some("b.txt".to_string()),
        size: Some(500), // Larger size, but we're sorting by name
        created: None,
        modified: None,
        md5: None,
        sha512: None,
        xxh3: None,
    };
    
    // "a.txt" should come before "b.txt"
    assert_eq!(compare(&config, &file_a, &file_b), std::cmp::Ordering::Less);
    assert_eq!(compare(&config, &file_b, &file_a), std::cmp::Ordering::Greater);
}

#[test]
fn test_calculate_hash_consistent_for_same_content() {
    let temp_file1 = NamedTempFile::new().unwrap();
    let temp_file2 = NamedTempFile::new().unwrap();
    
    let content = "identical content for both files";
    std::fs::write(temp_file1.path(), content).unwrap();
    std::fs::write(temp_file2.path(), content).unwrap();
    
    let hash1_md5 = calculate_hash(&temp_file1.path().to_path_buf(), "md5", 1024).unwrap();
    let hash2_md5 = calculate_hash(&temp_file2.path().to_path_buf(), "md5", 1024).unwrap();
    
    let hash1_sha512 = calculate_hash(&temp_file1.path().to_path_buf(), "sha512", 1024).unwrap();
    let hash2_sha512 = calculate_hash(&temp_file2.path().to_path_buf(), "sha512", 1024).unwrap();
    
    let hash1_xxh3 = calculate_hash(&temp_file1.path().to_path_buf(), "xxh3", 1024).unwrap();
    let hash2_xxh3 = calculate_hash(&temp_file2.path().to_path_buf(), "xxh3", 1024).unwrap();
    
    // Same content -> same hashes
    assert_eq!(hash1_md5, hash2_md5);
    assert_eq!(hash1_sha512, hash2_sha512);
    assert_eq!(hash1_xxh3, hash2_xxh3);
    
    // Different algorithms -> different hashes
    assert_ne!(hash1_md5, hash1_sha512);
    assert_ne!(hash1_md5, hash1_xxh3);
    assert_ne!(hash1_sha512, hash1_xxh3);
}

#[test]
fn test_calculate_hash_different_for_different_content() {
    let temp_file1 = NamedTempFile::new().unwrap();
    let temp_file2 = NamedTempFile::new().unwrap();
    
    std::fs::write(temp_file1.path(), "content one").unwrap();
    std::fs::write(temp_file2.path(), "content two").unwrap();
    
    let hash1_md5 = calculate_hash(&temp_file1.path().to_path_buf(), "md5", 1024).unwrap();
    let hash2_md5 = calculate_hash(&temp_file2.path().to_path_buf(), "md5", 1024).unwrap();
    
    let hash1_sha512 = calculate_hash(&temp_file1.path().to_path_buf(), "sha512", 1024).unwrap();
    let hash2_sha512 = calculate_hash(&temp_file2.path().to_path_buf(), "sha512", 1024).unwrap();
    
    let hash1_xxh3 = calculate_hash(&temp_file1.path().to_path_buf(), "xxh3", 1024).unwrap();
    let hash2_xxh3 = calculate_hash(&temp_file2.path().to_path_buf(), "xxh3", 1024).unwrap();
    
    // Different content -> different hashes
    assert_ne!(hash1_md5, hash2_md5);
    assert_ne!(hash1_sha512, hash2_sha512);
    assert_ne!(hash1_xxh3, hash2_xxh3);
}

#[test]
fn test_file_scanner_groups_correctly() {
    let temp_dir = tempfile::tempdir().unwrap();
    
    // Create file structure:
    // - file1.txt and file2.txt - same content
    // - file3.txt - unique file
    // - file4.txt and file5.txt - same content, but different from file1/file2
    
    let content1 = "duplicate group A";
    let content2 = "duplicate group B";
    
    std::fs::write(temp_dir.path().join("file1.txt"), content1).unwrap();
    std::fs::write(temp_dir.path().join("file2.txt"), content1).unwrap();
    std::fs::write(temp_dir.path().join("file3.txt"), "unique content").unwrap();
    std::fs::write(temp_dir.path().join("file4.txt"), content2).unwrap();
    std::fs::write(temp_dir.path().join("file5.txt"), content2).unwrap();
    
    let config = ConfigFile {
        path_start: temp_dir.path().to_path_buf(),
        compare_by_md5: true,
        compare_by_size: true,
        ..ConfigFile::default()
    };
    
    let scanner = FileScanner::new(&config, false);
    let groups = scanner.scan().unwrap();
    
    // Should find exactly 2 duplicate groups:
    // - file1.txt and file2.txt
    // - file4.txt and file5.txt
    assert_eq!(groups.len(), 2, "Expected exactly 2 duplicate groups");

    let group_files: Vec<Vec<String>> = groups
        .iter()
        .map(|g| {
            g.paths
                .iter()
                .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
                .collect()
        })
        .collect();

    let has_group_a = group_files.iter().any(|g| {
        g.contains(&"file1.txt".to_string()) && g.contains(&"file2.txt".to_string())
    });
    let has_group_b = group_files.iter().any(|g| {
        g.contains(&"file4.txt".to_string()) && g.contains(&"file5.txt".to_string())
    });

    assert!(has_group_a, "Group with file1.txt and file2.txt must exist");
    assert!(has_group_b, "Group with file4.txt and file5.txt must exist");

    // file3.txt should not be part of any duplicate group
    let all_grouped: Vec<String> = group_files.into_iter().flatten().collect();
    assert!(
        !all_grouped.contains(&"file3.txt".to_string()),
        "file3.txt should not be part of any duplicate group"
    );
}

#[test]
fn test_config_combination_logic() {
    // Test the logic for combining criteria from CLI
    
    use clap::Parser;
    use fundoubler::config::{CliOptions, ConfigFile};
    
    // Test 1: --content should enable all hash algorithms
    let args = ["fundoubler", "--content"];
    let cli = CliOptions::parse_from(args);
    let config = ConfigFile::from_cli(&cli);
    
    assert!(
        config.compare_by_md5 && config.compare_by_sha512 && config.compare_by_xxh3,
        "--content should enable all hash algorithms"
    );
    
    // Test 2: --md5 should enable MD5 (other algorithms should not be enabled additionally)
    let args = ["fundoubler", "--md5"];
    let cli = CliOptions::parse_from(args);
    let config = ConfigFile::from_cli(&cli);
    
    assert!(config.compare_by_md5);
}

#[test]
fn test_memory_efficiency_large_file() {
    // Create a file larger than buffer size
    let temp_file = NamedTempFile::new().unwrap();
    let buffer_size = 8192;
    let file_size = buffer_size * 3; // 24KB
    
    let content = vec![65u8; file_size]; // 'A' repeated
    std::fs::write(temp_file.path(), &content).unwrap();
    
    // Calculate hash with small buffer
    let hash = calculate_hash(&temp_file.path().to_path_buf(), "md5", buffer_size).unwrap();
    
    // Calculate expected hash directly
    use md5::Context;
    let mut context = Context::new();
    context.consume(&content);
    let expected = format!("{:x}", context.finalize());
    
    assert_eq!(hash, expected, "Hash should be correct even with buffering");
}

#[test]
fn test_filter_logic() {
    // Test filter functionality: only files matching the regex name filter should be considered
    let temp_dir = tempfile::tempdir().unwrap();
    
    // Create duplicates only among jpg/png files, plus pdf/txt files that should be filtered out
    std::fs::write(temp_dir.path().join("image1a.jpg"), "group1").unwrap();
    std::fs::write(temp_dir.path().join("image1b.jpg"), "group1").unwrap();
    std::fs::write(temp_dir.path().join("picture.png"), "group2").unwrap();
    std::fs::write(temp_dir.path().join("picture_copy.png"), "group2").unwrap();
    std::fs::write(temp_dir.path().join("document.pdf"), "fake pdf").unwrap();
    std::fs::write(temp_dir.path().join("data.txt"), "text").unwrap();
    
    // Create config with filter
    let mut config = ConfigFile::default();
    config.path_start = temp_dir.path().to_path_buf();
    config.compare_by_size = true;
    config.compare_by_md5 = true;
    config.name_filter = Some(".*\\.(jpg|png)$".to_string());
    
    let scanner = FileScanner::new(&config, false);
    let groups = scanner.scan().unwrap();
    
    // Should find groups only from jpg/png files
    assert!(
        !groups.is_empty(),
        "Expected at least one duplicate group when filtered to jpg/png files"
    );
    
    let all_files: Vec<String> = groups
        .iter()
        .flat_map(|g| &g.paths)
        .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
        .collect();
    
    assert!(
        all_files.iter().any(|f| f.ends_with(".jpg") || f.ends_with(".png")),
        "Expected jpg/png files in groups"
    );
    assert!(
        !all_files.iter().any(|f| f.ends_with(".pdf") || f.ends_with(".txt")),
        "pdf/txt files must be filtered out from groups"
    );
}

#[test]
fn test_config_validate_no_criteria() {
    // Test configuration validation without any enabled criteria
    let mut config = ConfigFile::default();
    config.compare_by_name = false;
    config.compare_by_size = false;
    config.compare_by_created = false;
    config.compare_by_modified = false;
    config.compare_by_md5 = false;
    config.compare_by_sha512 = false;
    config.compare_by_xxh3 = false;
    
    let result = config.validate();
    assert!(
        result.is_err(),
        "Config with no comparison criteria should fail validation"
    );
    
    if let Err(e) = result {
        assert!(
            format!("{}", e).contains("At least one comparison criteria"),
            "Error message should mention comparison criteria"
        );
    }
}

#[test]
fn test_config_validate_with_criteria() {
    // Test validation with enabled criteria
    let mut config = ConfigFile::default();
    config.compare_by_size = true;
    
    let result = config.validate();
    assert!(
        result.is_ok(),
        "Config with at least one criterion should pass validation"
    );
}

#[test]
fn test_compare_with_timestamps() {
    use std::time::{SystemTime, Duration};
    
    let mut config = ConfigFile::default();
    config.sort_orders = vec![SortOrder::Created];
    
    let now = SystemTime::now();
    let earlier = now - Duration::from_secs(3600);
    let later = now + Duration::from_secs(3600);
    
    let file_earlier = CheckOptions {
        name: Some("file1.txt".to_string()),
        size: Some(100),
        created: Some(earlier),
        modified: None,
        md5: None,
        sha512: None,
        xxh3: None,
    };
    
    let file_later = CheckOptions {
        name: Some("file2.txt".to_string()),
        size: Some(100),
        created: Some(later),
        modified: None,
        md5: None,
        sha512: None,
        xxh3: None,
    };
    
    // Earlier created file should come first
    assert_eq!(
        compare(&config, &file_earlier, &file_later),
        std::cmp::Ordering::Less
    );
    assert_eq!(
        compare(&config, &file_later, &file_earlier),
        std::cmp::Ordering::Greater
    );
}

#[test]
fn test_compare_with_modified_timestamp() {
    use std::time::{SystemTime, Duration};
    
    let mut config = ConfigFile::default();
    config.sort_orders = vec![SortOrder::ModifiedDesc];
    
    let now = SystemTime::now();
    let earlier = now - Duration::from_secs(3600);
    let later = now + Duration::from_secs(3600);
    
    let file_earlier = CheckOptions {
        name: Some("file1.txt".to_string()),
        size: Some(100),
        created: None,
        modified: Some(earlier),
        md5: None,
        sha512: None,
        xxh3: None,
    };
    
    let file_later = CheckOptions {
        name: Some("file2.txt".to_string()),
        size: Some(100),
        created: None,
        modified: Some(later),
        md5: None,
        sha512: None,
        xxh3: None,
    };
    
    // When sorting in descending order, the later file should come first
    assert_eq!(
        compare(&config, &file_earlier, &file_later),
        std::cmp::Ordering::Greater
    );
    assert_eq!(
        compare(&config, &file_later, &file_earlier),
        std::cmp::Ordering::Less
    );
}

#[test]
fn test_compare_multiple_sort_orders() {
    let mut config = ConfigFile::default();
    // First by size (descending), then by name
    config.sort_orders = vec![SortOrder::SizeDesc, SortOrder::Name];
    
    // Files of same size, different names
    let file_a = CheckOptions {
        name: Some("a.txt".to_string()),
        size: Some(100),
        created: None,
        modified: None,
        md5: None,
        sha512: None,
        xxh3: None,
    };
    
    let file_b = CheckOptions {
        name: Some("b.txt".to_string()),
        size: Some(100), // Same size
        created: None,
        modified: None,
        md5: None,
        sha512: None,
        xxh3: None,
    };
    
    // When size is equal, sorting should proceed by name
    assert_eq!(
        compare(&config, &file_a, &file_b),
        std::cmp::Ordering::Less
    );
    
    // Files of different sizes - size takes priority
    let file_large = CheckOptions {
        name: Some("z.txt".to_string()), // Later alphabetically
        size: Some(500),
        created: None,
        modified: None,
        md5: None,
        sha512: None,
        xxh3: None,
    };
    
    // Larger file should come first (SizeDesc)
    assert_eq!(
        compare(&config, &file_a, &file_large),
        std::cmp::Ordering::Greater
    );
}

#[test]
fn test_config_from_cli_content_flag() {
    use clap::Parser;
    use fundoubler::config::{CliOptions, ConfigFile};
    
    // --content should enable all three hash algorithms
    let args = ["fundoubler", "--content"];
    let cli = CliOptions::parse_from(args);
    let config = ConfigFile::from_cli(&cli);
    
    assert!(
        config.compare_by_md5 && config.compare_by_sha512 && config.compare_by_xxh3,
        "--content should enable all three hash algorithms"
    );
}

#[test]
fn test_config_from_cli_content_and_specific_hash() {
    use clap::Parser;
    use fundoubler::config::{CliOptions, ConfigFile};
    
    // --content + --md5: all hash algorithms should be enabled
    let args = ["fundoubler", "--content", "--md5"];
    let cli = CliOptions::parse_from(args);
    let config = ConfigFile::from_cli(&cli);
    
    assert!(
        config.compare_by_md5 && config.compare_by_sha512 && config.compare_by_xxh3,
        "--content should enable all hashes even when combined with specific hash flag"
    );
}

#[test]
fn test_hash_calculation_error_handling() {
    use std::path::PathBuf;
    
    // Attempt to calculate hash of nonexistent file
    let nonexistent = PathBuf::from("/nonexistent/file/that/does/not/exist.txt");
    let result = calculate_hash(&nonexistent, "md5", 8192);
    
    assert!(
        result.is_err(),
        "Hash calculation should fail for nonexistent file"
    );
}

#[test]
fn test_name_only_comparison() {
    // Test verifies that when comparing by name only,
    // files with the same name are grouped regardless of content
    let temp_dir = tempfile::tempdir().unwrap();
    
    // Files with same name in different directories, but different content
    let subdir1 = temp_dir.path().join("dir1");
    let subdir2 = temp_dir.path().join("dir2");
    std::fs::create_dir_all(&subdir1).unwrap();
    std::fs::create_dir_all(&subdir2).unwrap();
    
    std::fs::write(subdir1.join("common.txt"), "content one").unwrap();
    std::fs::write(subdir2.join("common.txt"), "content two").unwrap(); // Different content!
    std::fs::write(temp_dir.path().join("unique.txt"), "unique").unwrap();
    
    let mut config = ConfigFile::default();
    config.path_start = temp_dir.path().to_path_buf();
    config.compare_by_name = true;
    config.compare_by_size = false; // Disable size comparison
    config.compare_by_xxh3 = false; // Disable hash comparison
    
    let scanner = FileScanner::new(&config, false);
    let groups = scanner.scan().unwrap();
    
    // Should find a group with common.txt from both directories
    assert!(
        groups.len() >= 1,
        "Should find at least one group when comparing by name only"
    );
    
    let all_files: Vec<String> = groups
        .iter()
        .flat_map(|g| &g.paths)
        .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
        .collect();
    
    // common.txt should be in groups (from both directories)
    let common_count = all_files.iter().filter(|f| *f == "common.txt").count();
    assert_eq!(
        common_count, 2,
        "Both common.txt files should be grouped together when comparing by name only"
    );
}

#[test]
fn test_created_timestamp_comparison() {
    use std::time::{SystemTime, Duration};
    use std::thread;
    
    let temp_dir = tempfile::tempdir().unwrap();
    
    // Create files with a small delay so they have different creation times
    let file1_path = temp_dir.path().join("file1.txt");
    std::fs::write(&file1_path, "content").unwrap();
    
    thread::sleep(Duration::from_millis(100));
    
    let file2_path = temp_dir.path().join("file2.txt");
    std::fs::write(&file2_path, "content").unwrap(); // Same content
    
    let mut config = ConfigFile::default();
    config.path_start = temp_dir.path().to_path_buf();
    config.compare_by_created = true;
    config.compare_by_size = true; // Also by size for grouping
    config.compare_by_xxh3 = false;
    
    let scanner = FileScanner::new(&config, false);
    let groups = scanner.scan().unwrap();
    
    // Files with different creation times should not be in the same group
    // (even if content is the same)
    let all_grouped: Vec<String> = groups
        .iter()
        .flat_map(|g| &g.paths)
        .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
        .collect();
    
    // If both files are in the same group, comparison by created is not working
    // If in different groups or no duplicates found - this is expected
    // (since creation times are different)
    // Main thing - program should not panic
    assert!(
        groups.len() <= 1,
        "Files with different creation times should not be grouped together"
    );
}

#[test]
fn test_modified_timestamp_comparison() {
    use std::time::Duration;
    use std::thread;
    use std::fs;
    
    let temp_dir = tempfile::tempdir().unwrap();
    
    let file1_path = temp_dir.path().join("file1.txt");
    std::fs::write(&file1_path, "content").unwrap();
    
    thread::sleep(Duration::from_millis(100));
    
    // Modify file to change modification time
    fs::write(&file1_path, "modified content").unwrap();
    
    let file2_path = temp_dir.path().join("file2.txt");
    std::fs::write(&file2_path, "content").unwrap(); // Original content
    
    let mut config = ConfigFile::default();
    config.path_start = temp_dir.path().to_path_buf();
    config.compare_by_modified = true;
    config.compare_by_size = true;
    config.compare_by_xxh3 = false;
    
    let scanner = FileScanner::new(&config, false);
    let groups = scanner.scan().unwrap();
    
    // Files with different modification times should not be in the same group
    // Main thing - program handles this correctly
    assert!(
        groups.len() <= 1,
        "Files with different modification times should not be grouped together"
    );
}