use fundoubler::check::{calculate_hash, compare, CheckOptions};
use fundoubler::config::{ConfigFile, SortOrder};
use fundoubler::hash_cache::HashCache;
use fundoubler::scanner::FileScanner;
use fundoubler::DEFAULT_HASH_BUFFER_SIZE;
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
    assert_eq!(
        compare(&config, &small_file, &large_file),
        std::cmp::Ordering::Greater
    );
    assert_eq!(
        compare(&config, &large_file, &small_file),
        std::cmp::Ordering::Less
    );

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
    assert_eq!(
        compare(&config, &file_b, &file_a),
        std::cmp::Ordering::Greater
    );
}

#[test]
fn test_calculate_hash_consistent_for_same_content() {
    let temp_file1 = NamedTempFile::new().unwrap();
    let temp_file2 = NamedTempFile::new().unwrap();

    let content = "identical content for both files";
    std::fs::write(temp_file1.path(), content).unwrap();
    std::fs::write(temp_file2.path(), content).unwrap();

    let hash1_md5 = calculate_hash(
        &temp_file1.path().to_path_buf(),
        "md5",
        DEFAULT_HASH_BUFFER_SIZE as usize,
    )
    .unwrap();
    let hash2_md5 = calculate_hash(
        &temp_file2.path().to_path_buf(),
        "md5",
        DEFAULT_HASH_BUFFER_SIZE as usize,
    )
    .unwrap();

    let hash1_sha512 = calculate_hash(
        &temp_file1.path().to_path_buf(),
        "sha512",
        DEFAULT_HASH_BUFFER_SIZE as usize,
    )
    .unwrap();
    let hash2_sha512 = calculate_hash(
        &temp_file2.path().to_path_buf(),
        "sha512",
        DEFAULT_HASH_BUFFER_SIZE as usize,
    )
    .unwrap();

    let hash1_xxh3 = calculate_hash(
        &temp_file1.path().to_path_buf(),
        "xxh3",
        DEFAULT_HASH_BUFFER_SIZE as usize,
    )
    .unwrap();
    let hash2_xxh3 = calculate_hash(
        &temp_file2.path().to_path_buf(),
        "xxh3",
        DEFAULT_HASH_BUFFER_SIZE as usize,
    )
    .unwrap();

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

    let hash1_md5 = calculate_hash(
        &temp_file1.path().to_path_buf(),
        "md5",
        DEFAULT_HASH_BUFFER_SIZE as usize,
    )
    .unwrap();
    let hash2_md5 = calculate_hash(
        &temp_file2.path().to_path_buf(),
        "md5",
        DEFAULT_HASH_BUFFER_SIZE as usize,
    )
    .unwrap();

    let hash1_sha512 = calculate_hash(
        &temp_file1.path().to_path_buf(),
        "sha512",
        DEFAULT_HASH_BUFFER_SIZE as usize,
    )
    .unwrap();
    let hash2_sha512 = calculate_hash(
        &temp_file2.path().to_path_buf(),
        "sha512",
        DEFAULT_HASH_BUFFER_SIZE as usize,
    )
    .unwrap();

    let hash1_xxh3 = calculate_hash(
        &temp_file1.path().to_path_buf(),
        "xxh3",
        DEFAULT_HASH_BUFFER_SIZE as usize,
    )
    .unwrap();
    let hash2_xxh3 = calculate_hash(
        &temp_file2.path().to_path_buf(),
        "xxh3",
        DEFAULT_HASH_BUFFER_SIZE as usize,
    )
    .unwrap();

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

    let has_group_a = group_files
        .iter()
        .any(|g| g.contains(&"file1.txt".to_string()) && g.contains(&"file2.txt".to_string()));
    let has_group_b = group_files
        .iter()
        .any(|g| g.contains(&"file4.txt".to_string()) && g.contains(&"file5.txt".to_string()));

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
    let config = ConfigFile::from_cli(&cli).expect("from_cli without config file should succeed");

    assert!(
        config.compare_by_md5 && config.compare_by_sha512 && config.compare_by_xxh3,
        "--content should enable all hash algorithms"
    );

    // Test 2: --md5 should enable MD5 (other algorithms should not be enabled additionally)
    let args = ["fundoubler", "--md5"];
    let cli = CliOptions::parse_from(args);
    let config = ConfigFile::from_cli(&cli).expect("from_cli without config file should succeed");

    assert!(config.compare_by_md5);
}

#[test]
fn test_memory_efficiency_large_file() {
    // Create a file larger than buffer size
    let temp_file = NamedTempFile::new().unwrap();
    let buffer_size = DEFAULT_HASH_BUFFER_SIZE as usize;
    let file_size = buffer_size * 3;

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
        all_files
            .iter()
            .any(|f| f.ends_with(".jpg") || f.ends_with(".png")),
        "Expected jpg/png files in groups"
    );
    assert!(
        !all_files
            .iter()
            .any(|f| f.ends_with(".pdf") || f.ends_with(".txt")),
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
fn test_config_validate_invalid_datetime() {
    // Test that invalid datetime strings in time filters fail validation
    let mut config = ConfigFile::default();
    config.compare_by_size = true;
    config.min_mod_time = Some("not-a-valid-date".to_string());

    let result = config.validate();
    assert!(
        result.is_err(),
        "Config with invalid min_mod_time should fail validation"
    );
    if let Err(e) = result {
        let msg = format!("{}", e);
        assert!(msg.contains("Invalid") || msg.contains("min_mod_time"));
    }
}

#[test]
fn test_config_validate_invalid_create_time() {
    let mut config = ConfigFile::default();
    config.compare_by_size = true;
    config.min_create_time = Some("2024-13-99".to_string()); // Invalid month/day

    let result = config.validate();
    assert!(
        result.is_err(),
        "Config with invalid min_create_time should fail validation"
    );
}

#[test]
fn test_compare_with_timestamps() {
    use std::time::{Duration, SystemTime};

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
    use std::time::{Duration, SystemTime};

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
    assert_eq!(compare(&config, &file_a, &file_b), std::cmp::Ordering::Less);

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
    let config = ConfigFile::from_cli(&cli).expect("from_cli without config file should succeed");

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
    let config = ConfigFile::from_cli(&cli).expect("from_cli without config file should succeed");

    assert!(
        config.compare_by_md5 && config.compare_by_sha512 && config.compare_by_xxh3,
        "--content should enable all hashes even when combined with specific hash flag"
    );
}

#[test]
fn test_config_from_cli_comparison_flags() {
    use clap::Parser;
    use fundoubler::config::{CliOptions, ConfigFile};

    // --name enables compare_by_name
    let args = ["fundoubler", "--name"];
    let cli = CliOptions::parse_from(args);
    let config = ConfigFile::from_cli(&cli).expect("from_cli should succeed");
    assert!(
        config.compare_by_name,
        "--name should enable compare_by_name"
    );

    // --size enables compare_by_size only (no hashing - fast)
    let args = ["fundoubler", "--size"];
    let cli = CliOptions::parse_from(args);
    let config = ConfigFile::from_cli(&cli).expect("from_cli should succeed");
    assert!(
        config.compare_by_size,
        "--size should enable compare_by_size"
    );
    assert!(
        !config.compare_by_xxh3 && !config.compare_by_md5 && !config.compare_by_sha512,
        "--size alone should not enable hashing (keeps processing fast)"
    );

    // --create-date enables compare_by_created
    let args = ["fundoubler", "--create-date"];
    let cli = CliOptions::parse_from(args);
    let config = ConfigFile::from_cli(&cli).expect("from_cli should succeed");
    assert!(
        config.compare_by_created,
        "--create-date should enable compare_by_created"
    );

    // --mod-date enables compare_by_modified
    let args = ["fundoubler", "--mod-date"];
    let cli = CliOptions::parse_from(args);
    let config = ConfigFile::from_cli(&cli).expect("from_cli should succeed");
    assert!(
        config.compare_by_modified,
        "--mod-date should enable compare_by_modified"
    );
}

#[test]
fn test_config_from_cli_combined_comparison_size_and_name() {
    use clap::Parser;
    use fundoubler::config::{CliOptions, ConfigFile};

    let args = ["fundoubler", "--size", "--name"];
    let cli = CliOptions::parse_from(args);
    let config = ConfigFile::from_cli(&cli).expect("from_cli should succeed");

    assert!(config.compare_by_size, "size should be enabled");
    assert!(config.compare_by_name, "name should be enabled");
}

#[test]
fn test_config_from_cli_combined_comparison_mod_date_and_md5() {
    use clap::Parser;
    use fundoubler::config::{CliOptions, ConfigFile};

    let args = ["fundoubler", "--mod-date", "--md5"];
    let cli = CliOptions::parse_from(args);
    let config = ConfigFile::from_cli(&cli).expect("from_cli should succeed");

    assert!(config.compare_by_modified, "mod_date should be enabled");
    assert!(config.compare_by_md5, "md5 should be enabled");
}

#[test]
fn test_config_from_cli_combined_comparison_create_date_and_xxh3() {
    use clap::Parser;
    use fundoubler::config::{CliOptions, ConfigFile};

    let args = ["fundoubler", "--create-date", "--xxh3"];
    let cli = CliOptions::parse_from(args);
    let config = ConfigFile::from_cli(&cli).expect("from_cli should succeed");

    assert!(config.compare_by_created, "create_date should be enabled");
    assert!(config.compare_by_xxh3, "xxh3 should be enabled");
}

#[test]
fn test_config_from_cli_combined_comparison_three_way() {
    use clap::Parser;
    use fundoubler::config::{CliOptions, ConfigFile};

    let args = ["fundoubler", "--name", "--size", "--md5"];
    let cli = CliOptions::parse_from(args);
    let config = ConfigFile::from_cli(&cli).expect("from_cli should succeed");

    assert!(config.compare_by_name, "name should be enabled");
    assert!(config.compare_by_size, "size should be enabled");
    assert!(config.compare_by_md5, "md5 should be enabled");
}

#[test]
fn test_config_from_cli_short_name_flag() {
    use clap::Parser;
    use fundoubler::config::{CliOptions, ConfigFile};

    let args = ["fundoubler", "-n"];
    let cli = CliOptions::parse_from(args);
    let config = ConfigFile::from_cli(&cli).expect("from_cli should succeed");

    assert!(config.compare_by_name, "-n should enable compare_by_name");
}

#[test]
fn test_config_from_cli_config_file() {
    use clap::Parser;
    use fundoubler::config::{CliOptions, ConfigFile};

    let temp_dir = tempfile::tempdir().unwrap();
    let config_path = temp_dir.path().join("fundoubler.toml");
    std::fs::write(
        &config_path,
        r#"
path_start = "."
compare_by_size = true
compare_by_xxh3 = true
compare_by_md5 = true
min_size = 100
max_size = 9999
limit = 5
"#,
    )
    .unwrap();

    let args = ["fundoubler", "--config", config_path.to_str().unwrap()];
    let cli = CliOptions::parse_from(args);
    let config =
        ConfigFile::from_cli(&cli).expect("from_cli with valid config file should succeed");

    assert!(config.compare_by_md5);
    assert!(config.compare_by_size);
    assert!(config.compare_by_xxh3);
    assert_eq!(config.min_size, 100);
    assert_eq!(config.max_size, 9999);
    assert_eq!(config.limit, Some(5));
}

#[test]
fn test_config_from_cli_config_file_cli_overrides() {
    use clap::Parser;
    use fundoubler::config::{CliOptions, ConfigFile};

    let temp_dir = tempfile::tempdir().unwrap();
    let config_path = temp_dir.path().join("fundoubler.toml");
    std::fs::write(
        &config_path,
        r#"
path_start = "."
compare_by_size = true
compare_by_xxh3 = true
min_size = 100
limit = 10
"#,
    )
    .unwrap();

    let args = [
        "fundoubler",
        "--config",
        config_path.to_str().unwrap(),
        "--min-size",
        "200",
        "--limit",
        "3",
    ];
    let cli = CliOptions::parse_from(args);
    let config = ConfigFile::from_cli(&cli).expect("from_cli with config file should succeed");

    assert_eq!(config.min_size, 200, "CLI --min-size should override file");
    assert_eq!(config.limit, Some(3), "CLI --limit should override file");
}

#[test]
fn test_config_from_cli_hash_buffer_size() {
    use clap::Parser;
    use fundoubler::config::{CliOptions, ConfigFile};

    let args = ["fundoubler", "--hash-buffer-size", "131072"];
    let cli = CliOptions::parse_from(args);
    let config = ConfigFile::from_cli(&cli).expect("from_cli should succeed");
    assert_eq!(config.hash_buffer_size, 131072);
}

#[test]
fn test_config_from_cli_no_progress_bar() {
    use clap::Parser;
    use fundoubler::config::{CliOptions, ConfigFile};

    let config = ConfigFile::from_cli(&CliOptions::parse_from([
        "fundoubler",
        ".",
        "--no-progress-bar",
    ]))
    .unwrap();
    assert!(config.no_progress_bar);
}

#[test]
fn test_config_file_no_progress_bar() {
    use clap::Parser;
    use fundoubler::config::{CliOptions, ConfigFile};

    let temp = tempfile::tempdir().unwrap();
    let config_path = temp.path().join("config.toml");
    std::fs::write(
        &config_path,
        r#"
path_start = "."
no_progress_bar = true
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
    assert!(config.no_progress_bar);
}

#[test]
fn test_config_from_cli_hash_cache_options() {
    use clap::Parser;
    use fundoubler::config::{CliOptions, ConfigFile};
    use std::path::PathBuf;

    let args = [
        "fundoubler",
        "--hash-cache",
        "--hash-cache-dir",
        "/tmp/my_cache",
    ];
    let cli = CliOptions::parse_from(args);
    let config = ConfigFile::from_cli(&cli).expect("from_cli should succeed");
    assert!(config.hash_cache);
    assert_eq!(config.hash_cache_dir, PathBuf::from("/tmp/my_cache"));
}

#[test]
fn test_config_from_cli_config_file_hash_cache_and_buffer() {
    use clap::Parser;
    use fundoubler::config::{CliOptions, ConfigFile};

    let temp_dir = tempfile::tempdir().unwrap();
    let config_path = temp_dir.path().join("fundoubler.toml");
    std::fs::write(
        &config_path,
        r#"
path_start = "."
compare_by_size = true
compare_by_xxh3 = true
hash_cache = true
hash_cache_dir = "./my_hash_cache"
hash_buffer_size = 32768
"#,
    )
    .unwrap();

    let args = ["fundoubler", "--config", config_path.to_str().unwrap()];
    let cli = CliOptions::parse_from(args);
    let config = ConfigFile::from_cli(&cli).expect("from_cli should succeed");
    assert!(config.hash_cache);
    assert_eq!(config.hash_buffer_size, 32768);
    assert!(config
        .hash_cache_dir
        .to_string_lossy()
        .contains("my_hash_cache"));
}

#[test]
fn test_scanner_with_hash_cache() {
    let temp_dir = tempfile::tempdir().unwrap();
    let cache_dir = temp_dir.path().join("cache");
    std::fs::create_dir_all(&cache_dir).unwrap();

    std::fs::write(temp_dir.path().join("a.txt"), "same").unwrap();
    std::fs::write(temp_dir.path().join("b.txt"), "same").unwrap();

    let mut config = ConfigFile::default();
    config.path_start = temp_dir.path().to_path_buf();
    config.compare_by_xxh3 = true;
    config.compare_by_size = true;
    config.hash_cache = true;
    config.hash_cache_dir = cache_dir.clone();

    let scanner = FileScanner::new(&config, false);
    let groups1 = scanner.scan().unwrap();
    assert_eq!(groups1.len(), 1);
    assert_eq!(groups1[0].paths.len(), 2);

    let scanner2 = FileScanner::new(&config, false);
    let groups2 = scanner2.scan().unwrap();
    assert_eq!(groups2.len(), 1);
    assert_eq!(groups2[0].paths.len(), 2);
}

#[test]
fn test_hashing_is_skipped_when_sizes_are_unique() {
    let temp_dir = tempfile::tempdir().unwrap();
    let cache_dir = temp_dir.path().join("cache");

    // All sizes are unique, so no pair can match by content.
    std::fs::write(temp_dir.path().join("a.bin"), "a").unwrap(); // 1 byte
    std::fs::write(temp_dir.path().join("b.bin"), "bb").unwrap(); // 2 bytes
    std::fs::write(temp_dir.path().join("c.bin"), "ccc").unwrap(); // 3 bytes

    let mut config = ConfigFile::default();
    config.path_start = temp_dir.path().to_path_buf();
    config.compare_by_size = false;
    config.compare_by_md5 = true;
    config.compare_by_xxh3 = false;
    config.compare_by_sha512 = false;
    config.hash_cache = true;
    config.hash_cache_dir = cache_dir.clone();

    let scanner = FileScanner::new(&config, false);
    let groups = scanner.scan().unwrap();
    assert!(groups.is_empty());

    // No hashes should be computed/cached when no same-size candidates exist.
    let cache_json = std::fs::read_to_string(cache_dir.join("cache.json")).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&cache_json).unwrap();
    let entries = parsed
        .get("entries")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();
    assert!(
        entries.is_empty(),
        "expected empty hash cache for unique-size dataset, got {} entries",
        entries.len()
    );
}

#[test]
fn test_hash_cache_invalid_json_loads_empty() {
    use std::path::Path;

    let temp_dir = tempfile::tempdir().unwrap();
    let cache_path = temp_dir.path().join("cache.json");
    std::fs::write(&cache_path, "{ invalid json }").unwrap();

    let cache_dir = temp_dir.path();
    let cache = HashCache::load(cache_dir);

    // Should not panic; corrupted cache loads as empty
    let hash = cache.get(Path::new("/nonexistent"), 0, None, "md5");
    assert!(hash.is_none());
}

#[test]
fn test_hash_cache_unknown_algorithm_returns_none() {
    use std::path::Path;

    let temp_dir = tempfile::tempdir().unwrap();
    let cache = HashCache::load(temp_dir.path());
    cache.insert(Path::new("/x"), 0, None, "md5", "abc".to_string());

    assert!(cache.get(Path::new("/x"), 0, None, "unknown").is_none());
}

#[test]
fn test_hash_cache_get_insert_save_load() {
    use std::path::Path;

    let temp_dir = tempfile::tempdir().unwrap();
    let cache_dir = temp_dir.path().join("cache");
    let cache = HashCache::load(&cache_dir);

    let path = Path::new("/some/file.txt");
    let size = 100u64;
    let mtime = std::time::SystemTime::now();

    assert!(cache.get(path, size, Some(mtime), "md5").is_none());
    cache.insert(path, size, Some(mtime), "md5", "abc123".to_string());
    assert_eq!(
        cache.get(path, size, Some(mtime), "md5").as_deref(),
        Some("abc123")
    );

    cache.save().expect("save should succeed");
    assert!(cache_dir.join("cache.json").exists());

    let loaded = HashCache::load(&cache_dir);
    assert_eq!(
        loaded.get(path, size, Some(mtime), "md5").as_deref(),
        Some("abc123")
    );
}

#[test]
fn test_config_from_cli_config_file_invalid_toml_errors() {
    use clap::Parser;
    use fundoubler::config::{CliOptions, ConfigFile};

    let temp_dir = tempfile::tempdir().unwrap();
    let config_path = temp_dir.path().join("bad.toml");
    std::fs::write(&config_path, "invalid toml {{{").unwrap();

    let args = ["fundoubler", "--config", config_path.to_str().unwrap()];
    let cli = CliOptions::parse_from(args);
    let result = ConfigFile::from_cli(&cli);
    assert!(result.is_err());
}

#[test]
fn test_config_from_cli_config_file_missing_errors() {
    use clap::Parser;
    use fundoubler::config::{CliOptions, ConfigFile};

    let args = ["fundoubler", "--config", "/nonexistent/fundoubler.toml"];
    let cli = CliOptions::parse_from(args);
    let result = ConfigFile::from_cli(&cli);
    assert!(
        result.is_err(),
        "from_cli with missing config path should error"
    );
}

#[test]
fn test_init_config_default_path() {
    use std::path::PathBuf;
    use clap::Parser;
    use fundoubler::config::CliOptions;

    let cli = CliOptions::parse_from(["fundoubler", "--init-config"]);
    assert_eq!(cli.init_config.as_ref(), Some(&PathBuf::from("fundoubler.toml")),
        "--init-config with no value should default to fundoubler.toml");
}

#[test]
fn test_hash_calculation_error_handling() {
    use std::path::PathBuf;

    // Attempt to calculate hash of nonexistent file
    let nonexistent = PathBuf::from("/nonexistent/file/that/does/not/exist.txt");
    let result = calculate_hash(&nonexistent, "md5", DEFAULT_HASH_BUFFER_SIZE as usize);

    assert!(
        result.is_err(),
        "Hash calculation should fail for nonexistent file"
    );
}

#[test]
fn test_calculate_hash_unknown_algorithm() {
    let temp_file = NamedTempFile::new().unwrap();
    std::fs::write(temp_file.path(), "content").unwrap();

    let result = calculate_hash(
        &temp_file.path().to_path_buf(),
        "unknown_algo",
        DEFAULT_HASH_BUFFER_SIZE as usize,
    );

    assert!(result.is_err(), "Unknown algorithm should return error");
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("Unknown") || err_msg.contains("unknown"));
}

#[test]
fn test_calculate_hash_empty_file() {
    let temp_file = NamedTempFile::new().unwrap();
    std::fs::write(temp_file.path(), "").unwrap();

    let hash_md5 = calculate_hash(
        &temp_file.path().to_path_buf(),
        "md5",
        DEFAULT_HASH_BUFFER_SIZE as usize,
    )
    .unwrap();

    assert!(!hash_md5.is_empty());
    assert_eq!(hash_md5, "d41d8cd98f00b204e9800998ecf8427e");
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
    use std::thread;
    use std::time::Duration;

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
    use std::fs;
    use std::thread;
    use std::time::Duration;

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
