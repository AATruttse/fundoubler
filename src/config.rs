use std::path::PathBuf;

use clap::{Parser, ValueEnum};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, ValueEnum, PartialEq)]
pub enum SortOrder {
    Name,
    NameDesc,
    Size,
    SizeDesc,
    Created,
    CreatedDesc,
    Modified,
    ModifiedDesc,
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct CliOptions {
    /// Start path (default: current directory)
    #[arg(default_value = ".")]
    pub path_start: PathBuf,
    
    /// Output file (default: stdout)
    pub output: Option<PathBuf>,
    
    /// Check files by content (implies hashing)
    #[arg(short = 't', long)]
    pub content: bool,
    
    /// Check files by MD5 hash
    #[arg(long)]
    pub md5: bool,
    
    /// Check files by SHA512 hash
    #[arg(long)]
    pub sha512: bool,
    
    /// Check files by XXH3 hash (fast)
    #[arg(long)]
    pub xxh3: bool,
    
    /// Delete duplicates after confirmation
    #[arg(short = 'd', long)]
    pub delete: bool,
    
    /// Force delete without confirmation (DANGEROUS!)
    #[arg(short = 'f', long)]
    pub force_delete: bool,
    
    /// Dry run - don't actually delete
    #[arg(long)]
    pub dry_run: bool,
    
    /// Minimum file size (bytes)
    #[arg(long)]
    pub min_size: Option<u64>,
    
    /// Maximum file size (bytes)
    #[arg(long)]
    pub max_size: Option<u64>,
    
    /// File name filter (regex)
    #[arg(long)]
    pub filter: Option<String>,
    
    /// Sort order (can be specified multiple times)
    #[arg(long, value_enum)]
    pub sort: Vec<SortOrder>,
    
    /// Verbosity level (can be used multiple times)
    #[arg(short = 'v', long, action = clap::ArgAction::Count)]
    pub verbose: u8,
    
    /// Silent mode (no output)
    #[arg(short = 's', long)]
    pub silent: bool,
    
    /// Configuration file
    #[arg(long)]
    pub config: Option<PathBuf>,
    
    /// Show only N groups of duplicates
    #[arg(long)]
    pub limit: Option<usize>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConfigFile {
    // Core settings
    pub path_start: PathBuf,
    pub output: Option<PathBuf>,
    
    // Comparison criteria
    pub compare_by_name: bool,
    pub compare_by_size: bool,
    pub compare_by_created: bool,
    pub compare_by_modified: bool,
    pub compare_by_md5: bool,
    pub compare_by_sha512: bool,
    pub compare_by_xxh3: bool,
    
    // Filters
    pub min_size: u64,
    pub max_size: u64,
    pub name_filter: Option<String>,
    
    // Output control
    pub sort_orders: Vec<SortOrder>,
    pub limit: Option<usize>,
    pub verbose: u8,
    pub silent: bool,
    
    // Deletion
    pub delete: bool,
    pub force_delete: bool,
    pub dry_run: bool,

    pub test_mode: bool
}

impl Default for ConfigFile {
    fn default() -> Self {
        Self {
            path_start: PathBuf::from("."),
            output: None,
            compare_by_name: false,
            compare_by_size: true,
            compare_by_created: false,
            compare_by_modified: false,
            compare_by_md5: false,
            compare_by_sha512: false,
            compare_by_xxh3: true,
            min_size: 0,
            max_size: u64::MAX,
            name_filter: None,
            sort_orders: vec![SortOrder::SizeDesc, SortOrder::Name],
            limit: None,
            verbose: 0,
            silent: false,
            delete: false,
            force_delete: false,
            dry_run: false,
            test_mode: false,
        }
    }
}

impl ConfigFile {
    pub fn from_cli(cli: &CliOptions) -> Self {
        let mut config = Self::default();
        
        // Core positional args
        config.path_start = cli.path_start.clone();
        config.output = cli.output.clone();

        // Merge CLI options with defaults
        if cli.content {
            config.compare_by_md5 = true;
            config.compare_by_sha512 = true;
            config.compare_by_xxh3 = true;
        }
        
        config.compare_by_md5 = cli.md5 || config.compare_by_md5;
        config.compare_by_sha512 = cli.sha512 || config.compare_by_sha512;
        config.compare_by_xxh3 = cli.xxh3 || config.compare_by_xxh3;
        
        config.delete = cli.delete;
        config.force_delete = cli.force_delete;
        config.dry_run = cli.dry_run;
        config.silent = cli.silent;
        config.verbose = if config.silent { 0 } else { cli.verbose };
        
        if let Some(min) = cli.min_size {
            config.min_size = min;
        }
        
        if let Some(max) = cli.max_size {
            config.max_size = max;
        }
        
        if let Some(filter) = &cli.filter {
            config.name_filter = Some(filter.clone());
        }
        
        if !cli.sort.is_empty() {
            config.sort_orders = cli.sort.clone();
        }
        
        if let Some(limit) = cli.limit {
            config.limit = Some(limit);
        }

        // Check if we're in a test environment (via environment variable)
        config.test_mode = std::env::var("CARGO_TARGET_DIR").is_ok() 
            || std::env::var("TEST_MODE").is_ok();        
        
        config
    }
    
    pub fn validate(&self) -> crate::error::Result<()> {
        if !self.compare_by_name
            && !self.compare_by_size
            && !self.compare_by_created
            && !self.compare_by_modified
            && !self.compare_by_md5
            && !self.compare_by_sha512
            && !self.compare_by_xxh3
        {
            return Err(crate::error::AppError::Config(
                "At least one comparison criteria must be enabled".to_string(),
            ));
        }
        
        Ok(())
    }
}