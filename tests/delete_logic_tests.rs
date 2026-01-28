use assert_fs::TempDir;
use std::fs;
use std::process::Command;

#[test]
fn test_delete_correct_files_basic() {
    let temp_dir = TempDir::new().unwrap();
    
    // Создаем структуру:
    // - original.txt, copy1.txt, copy2.txt — дубликаты
    // - different.txt — уникальный файл
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
    
    // Ожидаем успешное завершение при корректной работе
    assert!(
        output.status.success(),
        "Deletion dry-run should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // В режиме dry-run должны быть найдены дубликаты и показан план удаления,
    // но файлы на диске не должны измениться.
    assert!(
        stdout.contains("Group") || stdout.contains("Found"),
        "Expected dry-run output to describe duplicate groups, got: {}",
        stdout
    );
    
    // Проверяем, что все четыре файла по-прежнему существуют
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
}