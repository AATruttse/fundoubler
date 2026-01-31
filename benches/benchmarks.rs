use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};
use fundoubler::check::calculate_hash;
use fundoubler::DEFAULT_HASH_BUFFER_SIZE;
use std::time::Duration;
use tempfile::NamedTempFile;

fn create_test_file_of_size(size_mb: usize) -> NamedTempFile {
    let temp_file = NamedTempFile::new().unwrap();
    let content = vec![0u8; size_mb * 1024 * 1024];
    std::fs::write(temp_file.path(), content).unwrap();
    temp_file
}

fn bench_hash_algorithms(c: &mut Criterion) {
    let mut group = c.benchmark_group("hash_algorithms");
    
    for size in [1, 10, 100].iter() {
        // 100MB hashes are very slow; allow longer measurement time
        if *size == 100 {
            group.sample_size(10);
            group.measurement_time(Duration::from_secs(60));
        }
        let temp_file = create_test_file_of_size(*size);
        let path = temp_file.path().to_path_buf();
        
        group.bench_with_input(
            BenchmarkId::new("md5", format!("{}MB", size)),
            size,
            |b, _| b.iter(|| calculate_hash(&path, "md5", DEFAULT_HASH_BUFFER_SIZE as usize).unwrap())
        );
        
        group.bench_with_input(
            BenchmarkId::new("sha512", format!("{}MB", size)),
            size,
            |b, _| b.iter(|| calculate_hash(&path, "sha512", DEFAULT_HASH_BUFFER_SIZE as usize).unwrap())
        );
        
        group.bench_with_input(
            BenchmarkId::new("xxh3", format!("{}MB", size)),
            size,
            |b, _| b.iter(|| calculate_hash(&path, "xxh3", DEFAULT_HASH_BUFFER_SIZE as usize).unwrap())
        );
    }
    
    group.finish();
}

fn bench_duplicate_detection(c: &mut Criterion) {
    use fundoubler::scanner::FileScanner;
    use fundoubler::config::ConfigFile;
    use tempfile::TempDir;
    use std::fs;
    
    let mut group = c.benchmark_group("duplicate_detection");
    
    for file_count in [100, 1000, 5000].iter() {
        // Allow longer measurement time for slow scans
        if *file_count == 100 {
            group.sample_size(30);
            group.measurement_time(Duration::from_secs(20)); // ~16.3s needed for 30 samples
        } else if *file_count == 1000 {
            group.sample_size(10);
            group.measurement_time(Duration::from_secs(40));
        } else if *file_count == 5000 {
            group.sample_size(10);
            group.measurement_time(Duration::from_secs(90)); // ~85s needed for 10 samples
        }
        group.bench_with_input(
            BenchmarkId::new("scan_files", format!("{} files", file_count)),
            file_count,
            |b, &count| {
                b.iter_batched(
                    || {
                        // Setup: create test directory with files
                        let temp_dir = TempDir::new().unwrap();
                        
                        // Create files, half of them duplicates
                        for i in 0..count {
                            let content = if i % 2 == 0 {
                                "even content"
                            } else {
                                "odd content"
                            };
                            let path = temp_dir.path().join(format!("file{}.txt", i));
                            fs::write(path, content).unwrap();
                        }
                        
                        let config = ConfigFile {
                            path_start: temp_dir.path().to_path_buf(),
                            compare_by_xxh3: true,
                            compare_by_size: true,
                            ..ConfigFile::default()
                        };
                        
                        (temp_dir, config)
                    },
                    |(temp_dir, config)| {
                        // Measurement: scan for duplicates
                        let scanner = FileScanner::new(&config, false);
                        scanner.scan().unwrap();
                        
                        // Keep temp_dir alive
                        temp_dir
                    },
                    criterion::BatchSize::PerIteration,
                )
            }
        );
    }
    
    group.finish();
}

criterion_group!(
    benches,
    bench_hash_algorithms,
    bench_duplicate_detection,
);
criterion_main!(benches);