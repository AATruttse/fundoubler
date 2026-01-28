use fundoubler::check::{CheckOptions, calculate_hash, compare};
use fundoubler::config::{ConfigFile, SortOrder};
use fundoubler::scanner::FileScanner;  // ДОБАВЛЕНО
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
    
    // Тест 1: Сортировка по размеру (по убыванию)
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
    
    // Больший файл должен идти первым при сортировке по убыванию
    assert_eq!(compare(&config, &small_file, &large_file), std::cmp::Ordering::Greater);
    assert_eq!(compare(&config, &large_file, &small_file), std::cmp::Ordering::Less);
    
    // Тест 2: Сортировка по имени
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
        size: Some(500), // Больший размер, но сортируем по имени
        created: None,
        modified: None,
        md5: None,
        sha512: None,
        xxh3: None,
    };
    
    // "a.txt" должен идти до "b.txt"
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
    
    // Одинаковое содержимое -> одинаковые хеши
    assert_eq!(hash1_md5, hash2_md5);
    assert_eq!(hash1_sha512, hash2_sha512);
    assert_eq!(hash1_xxh3, hash2_xxh3);
    
    // Разные алгоритмы -> разные хеши
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
    
    // Разное содержимое -> разные хеши
    assert_ne!(hash1_md5, hash2_md5);
    assert_ne!(hash1_sha512, hash2_sha512);
    assert_ne!(hash1_xxh3, hash2_xxh3);
}

#[test]
fn test_file_scanner_groups_correctly() {
    let temp_dir = tempfile::tempdir().unwrap();
    
    // Создаем структуру файлов:
    // - file1.txt и file2.txt - одинаковое содержимое
    // - file3.txt - уникальный файл
    // - file4.txt и file5.txt - одинаковое содержимое, но другое
    
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
    
    // Должны найти ровно 2 группы дубликатов:
    // - file1.txt и file2.txt
    // - file4.txt и file5.txt
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

    // file3.txt не должен попадать в группы дубликатов
    let all_grouped: Vec<String> = group_files.into_iter().flatten().collect();
    assert!(
        !all_grouped.contains(&"file3.txt".to_string()),
        "file3.txt should not be part of any duplicate group"
    );
}

#[test]
fn test_config_combination_logic() {
    // Тестируем логику комбинирования критериев из CLI
    
    use clap::Parser;
    use fundoubler::config::{CliOptions, ConfigFile};
    
    // Тест 1: --content должен включать все хеши
    let args = ["fundoubler", "--content"];
    let cli = CliOptions::parse_from(args);
    let config = ConfigFile::from_cli(&cli);
    
    assert!(
        config.compare_by_md5 && config.compare_by_sha512 && config.compare_by_xxh3,
        "--content should enable all hash algorithms"
    );
    
    // Тест 2: --md5 должен включать MD5 (остальные алгоритмы не должны включаться дополнительно)
    let args = ["fundoubler", "--md5"];
    let cli = CliOptions::parse_from(args);
    let config = ConfigFile::from_cli(&cli);
    
    assert!(config.compare_by_md5);
}

#[test]
fn test_memory_efficiency_large_file() {
    // Создаем файл больше размера буфера
    let temp_file = NamedTempFile::new().unwrap();
    let buffer_size = 8192;
    let file_size = buffer_size * 3; // 24KB
    
    let content = vec![65u8; file_size]; // 'A' repeated
    std::fs::write(temp_file.path(), &content).unwrap();
    
    // Рассчитываем хеш с маленьким буфером
    let hash = calculate_hash(&temp_file.path().to_path_buf(), "md5", buffer_size).unwrap();
    
    // Рассчитываем ожидаемый хеш напрямую
    use md5::Context;
    let mut context = Context::new();
    context.consume(&content);
    let expected = format!("{:x}", context.finalize());
    
    assert_eq!(hash, expected, "Хеш должен быть правильным даже с буферизацией");
}

#[test]
fn test_filter_logic() {
    // Тест на работу фильтров: должны учитываться только файлы,
    // удовлетворяющие regex-фильтру по имени.
    let temp_dir = tempfile::tempdir().unwrap();
    
    // Создаем дубликаты только среди jpg/png, плюс pdf/txt которые должны быть отфильтрованы
    std::fs::write(temp_dir.path().join("image1a.jpg"), "group1").unwrap();
    std::fs::write(temp_dir.path().join("image1b.jpg"), "group1").unwrap();
    std::fs::write(temp_dir.path().join("picture.png"), "group2").unwrap();
    std::fs::write(temp_dir.path().join("picture_copy.png"), "group2").unwrap();
    std::fs::write(temp_dir.path().join("document.pdf"), "fake pdf").unwrap();
    std::fs::write(temp_dir.path().join("data.txt"), "text").unwrap();
    
    // Создаем конфиг с фильтром
    let mut config = ConfigFile::default();
    config.path_start = temp_dir.path().to_path_buf();
    config.compare_by_size = true;
    config.compare_by_md5 = true;
    config.name_filter = Some(".*\\.(jpg|png)$".to_string());
    
    let scanner = FileScanner::new(&config, false);
    let groups = scanner.scan().unwrap();
    
    // Должны быть найдены группы только из jpg/png файлов
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