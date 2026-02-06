use clap::Parser;

pub mod check;
pub mod config;
pub mod del_log;
pub mod filters;
pub mod error;
pub mod hash_cache;
pub mod links;
pub mod log;
pub mod scanner;

pub use check::{calculate_hash, compare, CheckOptions};
pub use config::{CliOptions, ConfigFile, SortOrder, DEFAULT_HASH_BUFFER_SIZE};
pub use error::{AppError, Result};
pub use scanner::{FileGroup, FileScanner};

use dialoguer::Confirm;
use std::fs::{self, File};
use std::io::Write;

/// Sentinel path for "use latest delete log"
const RESTORE_LATEST: &str = "_latest_";

/// Main application function
pub fn run() -> Result<()> {
    let cli = CliOptions::parse();

    // --restore: restore deleted files from delete log (skip normal flow)
    if let Some(restore_arg) = &cli.restore {
        let logs_dir = cli
            .logs_dir
            .clone()
            .unwrap_or_else(|| std::path::PathBuf::from("./logs"));
        let log_path = if restore_arg.to_string_lossy() == RESTORE_LATEST {
            del_log::find_latest_del_log(&logs_dir)?.ok_or_else(|| {
                AppError::Config("No delete log found. Run a deletion first.".to_string())
            })?
        } else {
            restore_arg.clone()
        };
        return run_restore(&log_path, cli.skip_confirm);
    }

    // Initialize logging early (from CLI) so config/validation errors are logged
    if cli.log_level > 0 {
        let logs_dir = cli
            .logs_dir
            .clone()
            .unwrap_or_else(|| std::path::PathBuf::from("./logs"));
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
            eprintln!(
                "Warning: failed to init logging to {:?}: {}",
                config.logs_dir, e
            );
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
            log::log_info(&format!(
                "Scan complete: {} duplicate groups found",
                g.len()
            ));
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
            log::log_error(&format!(
                "Failed to write output to {}: {}",
                output_path.display(),
                e
            ));
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

    let link_type = links::get_link_type_description(
        config.create_symlinks,
        config.create_hardlinks,
        config.create_shortcuts,
    );

    println!("\nFound {} groups of duplicates:", groups.len());
    println!("{}", "=".repeat(80));

    for (i, group) in groups.iter().enumerate() {
        println!("\nGroup {}:", i + 1);
        println!("Criteria: {}", group.key);
        println!("Files:");

        let kept = &group.paths[0];
        for (idx, path) in group.paths.iter().enumerate() {
            if idx == 0 {
                println!("  {} (kept)", path.display());
            } else {
                if config.delete && config.dry_run {
                    if let Some(lt) = link_type {
                        let link_path = links::get_link_path(path, kept, config.no_keep_link_names, config.create_shortcuts);
                        println!("  {} → would be replaced with {}: {}", path.display(), lt, link_path.display());
                    } else {
                        println!("  {} → would be deleted", path.display());
                    }
                } else {
                    println!("  {}", path.display());
                }
            }
        }

        if config.verbose > 0 {
            if let Some(size) = group.key.size {
                let total_size = size * (group.paths.len() as u64 - 1);
                println!("Wasted space: {} bytes", total_size);
            }
        }
    }

    // Summary
    let total_duplicates: usize = groups.iter().map(|g| g.paths.len() - 1).sum();
    let total_files: usize = groups.iter().map(|g| g.paths.len()).sum();

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
    let mut file = File::create(output_path).map_err(|e| AppError::Io(e))?;

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
    let link_type = links::get_link_type_description(
        config.create_symlinks,
        config.create_hardlinks,
        config.create_shortcuts,
    );

    if config.dry_run {
        if let Some(lt) = link_type {
            log::log_info(&format!("Link creation skipped (dry run): {}", lt));
            println!("\nDRY RUN: No files will be deleted. Links would be created instead.");
        } else {
            log::log_info("Deletion skipped (dry run)");
            println!("\nDRY RUN: No files will be deleted.");
        }
        return Ok(());
    }

    if config.force_delete {
        if let Some(lt) = link_type {
            println!("\nWARNING: Force mode enabled. Files will be replaced with {}s without confirmation!", lt);
        } else {
            println!("\nWARNING: Force delete enabled. Files will be deleted without confirmation!");
        }

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

    let mut del_log_file: Option<(std::path::PathBuf, File)> = None;
    if config.delete_log {
        match del_log::create_del_log(&config.logs_dir) {
            Ok((path, file)) => {
                del_log_file = Some((path, file));
                if !config.silent {
                    println!("Delete log: {}", del_log_file.as_ref().unwrap().0.display());
                }
            }
            Err(e) => {
                log::log_error(&format!("Failed to create delete log: {}", e));
                eprintln!("Warning: could not create delete log: {}", e);
            }
        }
    }

    let mut deleted_count = 0;

    for group in groups {
        let kept = &group.paths[0];
        for (_i, path) in group.paths.iter().enumerate().skip(1) {
            let link_path = if link_type.is_some() {
                links::get_link_path(path, kept, config.no_keep_link_names, config.create_shortcuts)
            } else {
                std::path::PathBuf::new() // Not used when not creating links
            };

            if !config.force_delete {
                let prompt = if let Some(lt) = link_type {
                    format!(
                        "Replace {} with {} pointing to {}?",
                        path.display(),
                        lt,
                        kept.display()
                    )
                } else {
                    format!("Delete {}? (Keep: {})", path.display(), kept.display())
                };

                if config.skip_confirm {
                    // Assume yes - proceed
                } else if !Confirm::new()
                    .with_prompt(&prompt)
                    .default(false)
                    .interact()
                    .map_err(|e| AppError::Dialoguer(e))?
                {
                    continue;
                }
            }

            // Delete the original file first
            match std::fs::remove_file(path) {
                Ok(_) => {
                    deleted_count += 1;

                    // Create link if requested
                    if let Some(lt) = link_type {
                        let result = if config.create_symlinks {
                            links::create_symlink(kept, &link_path)
                        } else if config.create_hardlinks {
                            links::create_hardlink(kept, &link_path)
                        } else if config.create_shortcuts {
                            links::create_shortcut(kept, &link_path)
                        } else {
                            Ok(())
                        };

                        match result {
                            Ok(_) => {
                                if config.verbose > 0 {
                                    println!("  Created {}: {}", lt, link_path.display());
                                }
                            }
                            Err(e) => {
                                let msg = format!("Error creating {} at {}: {}", lt, link_path.display(), e);
                                log::log_error(&msg);
                                eprintln!("{}", msg);
                            }
                        }
                    }

                    if let Some((_, ref mut f)) = &mut del_log_file {
                        let _ = del_log::write_record(f, path, kept);
                    }
                    if config.verbose > 0 && link_type.is_none() {
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

    if let Some(lt) = link_type {
        log::log_info(&format!("Replaced {} files with {}s", deleted_count, lt));
        println!("\nReplaced {} files with {}s.", deleted_count, lt);
    } else {
        log::log_info(&format!("Deleted {} duplicate files", deleted_count));
        println!("\nDeleted {} duplicate files.", deleted_count);
    }
    Ok(())
}

fn run_restore(log_path: &std::path::Path, skip_confirm: bool) -> Result<()> {
    let records = del_log::parse_del_log(log_path).map_err(|e| AppError::Io(e))?;

    if records.is_empty() {
        println!("Delete log is empty. Nothing to restore.");
        return Ok(());
    }

    println!(
        "Restoring from {} ({} records)",
        log_path.display(),
        records.len()
    );
    let mut restored = 0;
    let mut errors = 0;

    for (deleted, source) in &records {
        if !source.exists() {
            eprintln!(
                "Source no longer exists, cannot restore {}: {}",
                deleted.display(),
                source.display()
            );
            errors += 1;
            continue;
        }
        if deleted.exists() {
            eprintln!("Skipping {}: file already exists", deleted.display());
            continue;
        }
        if !skip_confirm {
            let prompt = format!("Restore {} from {}?", deleted.display(), source.display());
            if !Confirm::new()
                .with_prompt(&prompt)
                .default(false)
                .interact()
                .map_err(|e| AppError::Dialoguer(e))?
            {
                continue;
            }
        }
        if let Some(parent) = deleted.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent).map_err(AppError::Io)?;
            }
        }
        match fs::copy(source, deleted) {
            Ok(_) => {
                restored += 1;
                println!("  Restored: {}", deleted.display());
            }
            Err(e) => {
                eprintln!("  Error restoring {}: {}", deleted.display(), e);
                errors += 1;
            }
        }
    }

    println!("\nRestored {} files. {} errors.", restored, errors);
    Ok(())
}

fn calculate_wasted_space(groups: &[FileGroup]) -> Option<u64> {
    let mut total = 0u64;

    for group in groups {
        let size = group
            .key
            .size
            .or_else(|| {
                // When not comparing by size, get size from first file for display
                std::fs::metadata(&group.paths[0]).ok().map(|m| m.len())
            })
            .unwrap_or(0);
        if size > 0 && group.paths.len() > 1 {
            total += size * (group.paths.len() as u64 - 1);
        }
    }

    if total > 0 {
        Some(total)
    } else {
        None
    }
}
