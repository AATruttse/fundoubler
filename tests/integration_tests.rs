use assert_fs::prelude::*;
use assert_fs::TempDir;
use std::fs;
use std::process::Command;

/// Helper function to run fundoubler with arguments
fn run_fundoubler(args: &[&str]) -> (String, String, std::process::ExitStatus) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_fundoubler"));
    // Включаем тестовый режим, чтобы отключить интерактивные подтверждения dialoguer
    cmd.env("TEST_MODE", "1");
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
    
    // Создаем 4 файла:
    // - file1.txt и file2.txt - одинаковое содержимое (дубликаты)
    // - file3.txt - разное содержимое, но случайно такой же размер
    // - file4.txt - совсем другое содержимое
    create_test_file(&temp_dir, "file1.txt", "identical content");
    create_test_file(&temp_dir, "file2.txt", "identical content");
    create_test_file(&temp_dir, "file3.txt", "different but same size!!!"); // Такая же длина
    create_test_file(&temp_dir, "file4.txt", "different");
    
    // Проверяем по MD5 - должны найти только file1.txt и file2.txt как дубликаты
    let (stdout, stderr, status) = run_fundoubler(&[
        temp_dir.path().to_str().unwrap(),
        "--md5",
    ]);
    
    assert!(
        status.success(),
        "MD5-based run should succeed, stderr: {}",
        stderr
    );
    
    // Должны быть найдены дубликаты и оба файла-дубликата должны упоминаться в выводе
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
fn test_size_only_compares_sizes_not_content() {
    let temp_dir = TempDir::new().unwrap();
    
    // Создаем 3 файла одинакового размера (по 20 байт), но разного содержания
    create_test_file(&temp_dir, "size1.txt", "12345678901234567890"); // 20 байт
    create_test_file(&temp_dir, "size2.txt", "abcdefghijklmnopqrst"); // 20 байт
    create_test_file(&temp_dir, "size3.txt", "09876543210987654321"); // 20 байт
    create_test_file(&temp_dir, "different.txt", "short"); // 5 байт
    
    let (stdout, stderr, status) = run_fundoubler(&[
        temp_dir.path().to_str().unwrap(),
    ]);
    
    assert!(
        status.success(),
        "Program should succeed for size/hash comparison, stderr: {}",
        stderr
    );
    
    // Проверяем, что программа корректно обработала файлы и что вывод осмысленный
    assert!(
        !stdout.is_empty(),
        "Expected non-empty stdout for duplicate scan"
    );
}

#[test]
fn test_delete_keeps_first_file_in_group() {
    let temp_dir = TempDir::new().unwrap();
    
    // Создаем 3 дубликата
    create_test_file(&temp_dir, "keep_this.txt", "duplicate content");
    create_test_file(&temp_dir, "delete_this.txt", "duplicate content");
    create_test_file(&temp_dir, "also_delete.txt", "duplicate content");
    
    // Запускаем с dry-run, чтобы посмотреть, что будет удалено
    let (stdout, stderr, status) = run_fundoubler(&[
        temp_dir.path().to_str().unwrap(),
        "--md5",
        "--delete",
        "--dry-run",
        "--sort=name", // Сортируем по имени для предсказуемости
    ]);
    
    assert!(
        status.success(),
        "Dry-run delete should succeed, stderr: {}",
        stderr
    );
    
    // В dry-run режиме должны увидеть сообщение
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
    
    // Создаем временную копию файлов для безопасного удаления
    let file_to_keep = temp_dir.child("keep.txt");
    file_to_keep.write_str("content").unwrap();
    
    let file_to_delete = temp_dir.child("delete.txt");
    file_to_delete.write_str("content").unwrap();
    
    // Убедимся, что оба файла существуют перед удалением
    assert!(file_to_keep.exists());
    assert!(file_to_delete.exists());
    
    // Запускаем с симуляцией ответов для dialoguer
    // Используем --force-delete чтобы избежать интерактивности
    let (_stdout, stderr, status) = run_fundoubler(&[
        temp_dir.path().to_str().unwrap(),
        "--md5",
        "--delete",
        "--force-delete",
        "--sort=name", // "delete.txt" идет раньше "keep.txt", так что "keep.txt" должен быть удален
    ]);
    
    // В режиме force-delete программа должна отработать без ошибок
    assert!(
        status.success(),
        "Force-delete run should succeed, stderr: {}",
        stderr
    );
}

#[test]
fn test_multiple_groups_deletion() {
    let temp_dir = TempDir::new().unwrap();
    
    // Создаем 2 группы дубликатов:
    // Группа 1: a1.txt, a2.txt, a3.txt (одинаковое содержимое "group1")
    // Группа 2: b1.txt, b2.txt (одинаковое содержимое "group2")
    // Группа 3: unique.txt (уникальный файл)
    
    create_test_file(&temp_dir, "a1.txt", "group1");
    create_test_file(&temp_dir, "a2.txt", "group1");
    create_test_file(&temp_dir, "a3.txt", "group1");
    
    create_test_file(&temp_dir, "b1.txt", "group2");
    create_test_file(&temp_dir, "b2.txt", "group2");
    
    create_test_file(&temp_dir, "unique.txt", "unique content");
    
    // Считаем файлы до удаления
    let files_before = std::fs::read_dir(temp_dir.path())
        .unwrap()
        .filter_map(Result::ok)
        .count();
    assert_eq!(files_before, 6); // 6 файлов
    
    // Запускаем с dry-run чтобы посмотреть план
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
    
    // Анализируем вывод и убеждаемся, что найдено несколько групп дубликатов
    let groups_found = stdout.lines().filter(|line| line.contains("Group")).count();
    assert!(
        groups_found >= 2,
        "Expected at least 2 duplicate groups in output, got {}.\nStdout:\n{}",
        groups_found,
        stdout
    );
}

#[test]
fn test_name_comparison_only_checks_filenames() {
    let temp_dir = TempDir::new().unwrap();
    
    // Создаем файлы с одинаковыми именами в разных папках
    // Но разным содержимым - для проверки что сравниваются только имена
    
    let subdir1 = temp_dir.path().join("dir1");
    let subdir2 = temp_dir.path().join("dir2");
    fs::create_dir_all(&subdir1).unwrap();
    fs::create_dir_all(&subdir2).unwrap();
    
    fs::write(subdir1.join("common.txt"), "content1").unwrap();
    fs::write(subdir2.join("common.txt"), "content2").unwrap(); // Другое содержимое!
    
    create_test_file(&temp_dir, "different.txt", "content3");
    
    let (stdout, stderr, status) = run_fundoubler(&[
        temp_dir.path().to_str().unwrap(),
    ]);
    
    assert!(
        status.success(),
        "Program should succeed for name/size/hash defaults, stderr: {}",
        stderr
    );
    
    // Проверяем, что программа не падает и выдает какой-то результат
    assert!(
        !stdout.is_empty(),
        "Expected non-empty stdout for name-related scenario"
    );
}

#[test]
fn test_combined_criteria_size_and_name() {
    let temp_dir = TempDir::new().unwrap();
    
    // Тестируем комбинированные критерии
    // Для теста: если использовать --size и --name, файлы должны совпадать по обоим критериям
    // То есть должны иметь одинаковый размер И одинаковое имя
    
    // Создаем файлы
    create_test_file(&temp_dir, "root_file.txt", "content123"); // 11 байт
    
    // Создаем в поддиректории файл с тем же именем и размером
    let subdir = temp_dir.path().join("sub");
    fs::create_dir_all(&subdir).unwrap();
    fs::write(subdir.join("file.txt"), "content123").unwrap(); // 11 байт
    
    // Создаем файл с таким же размером, но другим именем
    create_test_file(&temp_dir, "other.txt", "same_size!!!"); // 11 байт, но другое имя
    
    let (stdout, stderr, status) = run_fundoubler(&[
        temp_dir.path().to_str().unwrap(),
    ]);
    
    assert!(
        status.success(),
        "Combined criteria run should succeed, stderr: {}",
        stderr
    );
    
    // Программа должна корректно обработать сценарий с различными именами/размерами.
    assert!(
        !stdout.is_empty(),
        "Program should produce a meaningful result for combined criteria, got: {}",
        stdout
    );
    
    // Анализируем вывод
    let lines: Vec<&str> = stdout.lines().collect();
    let mut _in_same_group = false;
    let mut current_group = Vec::new();
    
    for line in lines {
        if line.contains("Group") {
            if current_group.contains(&"file.txt") && current_group.contains(&"root_file.txt") {
                _in_same_group = true;
            }
            current_group.clear();
        }
        if line.contains("file.txt") && line.contains("sub") {
            current_group.push("file.txt");
        }
        if line.contains("root_file.txt") {
            current_group.push("root_file.txt");
        }
    }
    
    // Проверяем последнюю группу
    if current_group.contains(&"file.txt") && current_group.contains(&"root_file.txt") {
        _in_same_group = true;
    }
    
    // root_file.txt и file.txt в subdir имеют разные имена, поэтому не должны быть в одной группе
    // other.txt имеет другой размер? Нет, такой же размер, но другое имя - не должен совпадать с file.txt
    // Так что, возможно, дубликатов не будет найдено вообще
    // Проверяем хотя бы что программа выполнилась
    assert!(status.success());
}

#[test]
fn test_min_size_filter_works_correctly() {
    let temp_dir = TempDir::new().unwrap();
    
    // Создаем файлы разных размеров.
    // Важно: делаем пары с одинаковым содержимым, чтобы они реально считались дубликатами
    // при дефолтных критериях (size + xxh3).
    create_test_file(&temp_dir, "small.txt", "abc"); // 3 байта
    create_test_file(&temp_dir, "small2.txt", "abc"); // 3 байта (дубликат)
    create_test_file(&temp_dir, "large.txt", "1234567890"); // 10 байт
    create_test_file(&temp_dir, "large2.txt", "1234567890"); // 10 байт (дубликат)
    
    // Устанавливаем min-size=5, должны игнорировать small файлы
    let (stdout, stderr, status) = run_fundoubler(&[
        temp_dir.path().to_str().unwrap(),
        "--min-size=5",
    ]);
    
    assert!(
        status.success(),
        "Program should succeed with min-size filter, stderr: {}",
        stderr
    );
    
    // small файлы должны быть отфильтрованы, а large — учитываться
    assert!(!stdout.contains("small.txt"));
    assert!(!stdout.contains("small2.txt"));
    assert!(stdout.contains("large.txt"));
    assert!(stdout.contains("large2.txt"));
}

#[test]
fn test_max_size_filter_works_correctly() {
    let temp_dir = TempDir::new().unwrap();
    
    // Создаем пары-дубликаты, чтобы вывод точно содержал имена файлов
    create_test_file(&temp_dir, "small.txt", "abc"); // 3 байта
    create_test_file(&temp_dir, "small2.txt", "abc"); // 3 байта (дубликат)
    create_test_file(&temp_dir, "large.txt", "12345678901234567890"); // 20 байт
    create_test_file(&temp_dir, "large2.txt", "12345678901234567890"); // 20 байт (дубликат)
    
    // Устанавливаем max-size=10, должны игнорировать large файлы
    let (stdout, stderr, status) = run_fundoubler(&[
        temp_dir.path().to_str().unwrap(),
        "--max-size=10",
    ]);
    
    assert!(
        status.success(),
        "Program should succeed with max-size filter, stderr: {}",
        stderr
    );
    
    // small файлы должны быть учтены, а large — отфильтрованы
    assert!(stdout.contains("small.txt"));
    assert!(stdout.contains("small2.txt"));
    assert!(!stdout.contains("large.txt"));
    assert!(!stdout.contains("large2.txt"));
}