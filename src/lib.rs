use clap::Parser;

pub mod check;
pub mod config;
pub mod error;
pub mod hash_cache;
pub mod log;
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
    let cli = CliOptions::parse();

    // Initialize logging early (from CLI) so config/validation errors are logged
    if cli.log_level > 0 {
        let logs_dir = cli.logs_dir.clone().unwrap_or_else(|| std::path::PathBuf::from("./logs"));
        if let Err(e) = log::init(cli.log_level, &logs_dir) {
            eprintln!("Warning: failed to init logging: {}", e);
        }
    }

    // Create default config file and exit if requested
    if let Some(path) = &cli.init_config {
        let mut default_config = ConfigFile::default();
        // TOML uses i64 for integers; u64::MAX overflows. Use a large but safe value.
        default_config.max_size = i64::MAX as u64;
        let toml = toml::to_string_pretty(&default_config)
            .map_err(|e| AppError::Config(format!("Failed to serialize config: {}", e)))?;
        std::fs::write(path, toml)?;
        log::log_info(&format!("Created default config at {}", path.display()));
        if !cli.silent {
            println!("Created default config at {}", path.display());
        }
        return Ok(());
    }

    // Load configuration (from --config file if set, then CLI overrides)
    let config = match ConfigFile::from_cli(&cli) {
        Ok(c) => c,
        Err(e) => {
            log::log_error(&format!("Config load failed: {}", e));
            return Err(e);
        }
    };

    // Re-init logger from config (config file may override CLI log settings)
    if config.log_level > 0 {
        if let Err(e) = log::init(config.log_level, &config.logs_dir) {
            eprintln!("Warning: failed to init logging to {:?}: {}", config.logs_dir, e);
        } else {
            log::log_info("Logging initialized from config");
        }
    }

    // Validate configuration
    if let Err(e) = config.validate() {
        log::log_error(&format!("Validation failed: {}", e));
        return Err(e);
    }

    log::log_info(&format!("Starting scan at {}", config.path_start.display()));

    // Show config if verbose
    if config.verbose > 1 {
        println!("Configuration: {:#?}", config);
    }
    
    // Scan for duplicates
    let scanner = FileScanner::new(&config, true);
    let groups = match scanner.scan() {
        Ok(g) => {
            log::log_info(&format!("Scan complete: {} duplicate groups found", g.len()));
            g
        }
        Err(e) => {
            log::log_error(&format!("Scan failed: {}", e));
            return Err(e);
        }
    };

    // Display results
    if !config.silent {
        display_results(&groups, &config);
    }
    
    // Write to output file if specified
    if let Some(output_path) = &config.output {
        if let Err(e) = write_results(&groups, output_path) {
            log::log_error(&format!("Failed to write output to {}: {}", output_path.display(), e));
            return Err(e);
        }
        log::log_info(&format!("Results written to {}", output_path.display()));
    }

    // Handle deletion
    if config.delete {
        if let Err(e) = handle_deletion(&groups, &config) {
            log::log_error(&format!("Deletion failed: {}", e));
            return Err(e);
        }
    }

    log::log_info("Run completed successfully");
    Ok(())
}

fn display_results(groups: &[FileGroup], config: &ConfigFile) {
    if groups.is_empty() {
        log::log_info("No duplicates found");
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
        log::log_info("Deletion skipped (dry run)");
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
                log::log_info("Deletion cancelled by user");
                println!("Deletion cancelled.");
                return Ok(());
            }
        }
        log::log_info("Force delete confirmed");
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
                    let msg = format!("Error deleting {}: {}", path.display(), e);
                    log::log_error(&msg);
                    eprintln!("{}", msg);
                }
            }
        }
    }
    
    log::log_info(&format!("Deleted {} duplicate files", deleted_count));
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