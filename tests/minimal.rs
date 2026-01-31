use assert_fs::TempDir;
use std::fs;

#[test]
fn test_program_starts() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_fundoubler"))
        .arg("--help")
        .output()
        .expect("Failed to execute process");
    
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Basic smoke test: help output should mention the program name and be non-empty
    assert!(
        stdout.contains("fundoubler") && !stdout.is_empty(),
        "Help output should mention program name, got: {}",
        stdout
    );
}

#[test]
fn test_duplicate_detection_simple() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create two identical files
    let content = "identical content";
    fs::write(temp_dir.path().join("file1.txt"), content).unwrap();
    fs::write(temp_dir.path().join("file2.txt"), content).unwrap();
    
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_fundoubler"))
        .arg(temp_dir.path())
        .arg("--md5")
        .output()
        .expect("Failed to execute process");
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Program should complete successfully
    assert!(
        output.status.success(),
        "Program should succeed, stderr: {}",
        stderr
    );

    // Both duplicate files should be mentioned in output
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