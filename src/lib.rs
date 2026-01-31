use clap::Parser; 

pub mod check;
pub mod config;
pub mod error;
pub mod scanner;

pub use check::{CheckOptions, calculate_hash, compare};
pub use config::{ConfigFile, CliOptions, SortOrder, DEFAULT_HASH_BUFFER_SIZE};
pub use error::{AppError, Result};
pub use scanner::{FileScanner, FileGroup};

use std::fs::File;
use std::io::Write;
use dialoguer::Confirm;

/// Main application function
pub fn run() -> Result<()> {
    // Parse CLI arguments
    let cli = CliOptions::parse();

    // Create default config file and exit if requested
    if let Some(path) = &cli.init_config {
        let mut default_config = ConfigFile::default();
        // TOML uses i64 for integers; u64::MAX overflows. Use a large but safe value.
        default_config.max_size = i64::MAX as u64;
        let toml = toml::to_string_pretty(&default_config)
            .map_err(|e| AppError::Config(format!("Failed to serialize config: {}", e)))?;
        std::fs::write(path, toml)?;
        if !cli.silent {
            println!("Created default config at {}", path.display());
        }
        return Ok(());
    }
    
    // Load configuration (from --config file if set, then CLI overrides)
    let config = ConfigFile::from_cli(&cli)?;
    
    // Validate configuration
    config.validate()?;
    
    // Show config if verbose
    if config.verbose > 1 {
        println!("Configuration: {:#?}", config);
    }
    
    // Scan for duplicates
    let scanner = FileScanner::new(&config, true);
    let groups = scanner.scan()?;
    
    // Display results
    if !config.silent {
        display_results(&groups, &config);
    }
    
    // Write to output file if specified
    if let Some(output_path) = &config.output {
        write_results(&groups, output_path)?;
    }
    
    // Handle deletion
    if config.delete {
        handle_deletion(&groups, &config)?;
    }
    
    Ok(())
}

fn display_results(groups: &[FileGroup], config: &ConfigFile) {
    if groups.is_empty() {
        println!("No duplicates found.");
        return;
    }
    
    println!("\nFound {} groups of duplicates:", groups.len());
    println!("{}", "=".repeat(80));
    
    for (i, group) in groups.iter().enumerate() {
        println!("\nGroup {}:", i + 1);
        println!("Criteria: {}", group.key);
        println!("Files:");
        
        for path in &group.paths {
            println!("  {}", path.display());
        }
        
        if config.verbose > 0 {
            if let Some(size) = group.key.size {
                let total_size = size * (group.paths.len() as u64 - 1);
                println!("Wasted space: {} bytes", total_size);
            }
        }
    }
    
    // Summary
    let total_duplicates: usize = groups.iter()
        .map(|g| g.paths.len() - 1)
        .sum();
    let total_files: usize = groups.iter()
        .map(|g| g.paths.len())
        .sum();
    
    println!("\n{}", "=".repeat(80));
    println!("Summary:");
    println!("  Total duplicate groups: {}", groups.len());
    println!("  Total duplicate files: {}", total_duplicates);
    println!("  Total files in groups: {}", total_files);
    
    if let Some(wasted) = calculate_wasted_space(groups) {
        println!("  Wasted space: {} bytes", wasted);
    }
}

fn write_results(groups: &[FileGroup], output_path: &std::path::Path) -> Result<()> {
    let mut file = File::create(output_path)
        .map_err(|e| AppError::Io(e))?;
    
    writeln!(file, "Duplicate Report")?;
    writeln!(file, "{}", "=".repeat(80))?;
    
    for group in groups {
        writeln!(file, "\nCriteria: {}", group.key)?;
        for path in &group.paths {
            writeln!(file, "  {}", path.display())?;
        }
    }
    
    Ok(())
}

fn handle_deletion(groups: &[FileGroup], config: &ConfigFile) -> Result<()> {
    if config.dry_run {
        println!("\nDRY RUN: No files will be deleted.");
        return Ok(());
    }
    
    if config.force_delete {
        println!("\nWARNING: Force delete enabled. Files will be deleted without confirmation!");
        
        if !config.skip_confirm {
            if !Confirm::new()
                .with_prompt("Are you absolutely sure?")
                .default(false)
                .interact()
                .map_err(|e| AppError::Dialoguer(e))?
            {
                println!("Deletion cancelled.");
                return Ok(());
            }
        }
    }
    
    let mut deleted_count = 0;
    
    for group in groups {
        // Keep the first file, delete the rest
        for (_i, path) in group.paths.iter().enumerate().skip(1) {
            if !config.force_delete {
                let prompt = format!(
                    "Delete {}? (Keep: {})",
                    path.display(),
                    group.paths[0].display()
                );
                
                if config.skip_confirm {
                    // Assume yes - proceed to delete
                } else if !Confirm::new()
                    .with_prompt(&prompt)
                    .default(false)
                    .interact()
                    .map_err(|e| AppError::Dialoguer(e))?
                {
                    continue;
                }
            }
            
            if config.verbose > 0 {
                println!("Deleting: {}", path.display());
            }
            
            match std::fs::remove_file(path) {
                Ok(_) => {
                    deleted_count += 1;
                    if config.verbose > 0 {
                        println!("  Deleted successfully.");
                    }
                }
                Err(e) => {
                    eprintln!("Error deleting {}: {}", path.display(), e);
                }
            }
        }
    }
    
    println!("\nDeleted {} duplicate files.", deleted_count);
    Ok(())
}

fn calculate_wasted_space(groups: &[FileGroup]) -> Option<u64> {
    let mut total = 0u64;
    
    for group in groups {
        let size = group.key.size.or_else(|| {
            // When not comparing by size, get size from first file for display
            std::fs::metadata(&group.paths[0]).ok().map(|m| m.len())
        }).unwrap_or(0);
        if size > 0 && group.paths.len() > 1 {
            total += size * (group.paths.len() as u64 - 1);
        }
    }
    
    if total > 0 { Some(total) } else { None }
}