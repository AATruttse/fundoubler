use std::path::PathBuf;

use clap::{Parser, ValueEnum};

/// Default hash read buffer size in bytes (64KB).
pub const DEFAULT_HASH_BUFFER_SIZE: u64 = 65536;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fs;

/// Serialize PathBuf as string for TOML.
fn path_buf_to_str<S>(p: &PathBuf, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    s.serialize_str(p.to_string_lossy().as_ref())
}

/// Deserialize a TOML string into PathBuf (TOML has no native path type).
fn path_buf_from_str<'de, D>(d: D) -> Result<PathBuf, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(d)?;
    Ok(PathBuf::from(s))
}

/// Serialize Option<PathBuf> as optional string for TOML.
fn opt_path_buf_to_str<S>(p: &Option<PathBuf>, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match p {
        Some(path) => s.serialize_some(path.to_string_lossy().as_ref()),
        None => s.serialize_none(),
    }
}

/// Deserialize Option<String> from TOML into Option<PathBuf>.
fn opt_path_buf_from_str<'de, D>(d: D) -> Result<Option<PathBuf>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt = Option::<String>::deserialize(d)?;
    Ok(opt.map(PathBuf::from))
}

/// Serialize Vec<PathBuf> as array of strings for TOML.
fn vec_path_to_str<S>(v: &[PathBuf], s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let strs: Vec<String> = v.iter().map(|p| p.to_string_lossy().to_string()).collect();
    strs.serialize(s)
}

/// Deserialize array of strings into Vec<PathBuf>.
fn vec_path_from_str<'de, D>(d: D) -> Result<Vec<PathBuf>, D::Error>
where
    D: Deserializer<'de>,
{
    let v: Vec<String> = Vec::deserialize(d)?;
    Ok(v.into_iter().map(PathBuf::from).collect())
}

fn default_path_start() -> PathBuf {
    PathBuf::from(".")
}

fn default_hash_buffer_size() -> u64 {
    DEFAULT_HASH_BUFFER_SIZE
}

fn default_hash_cache_dir() -> PathBuf {
    PathBuf::from(".fundoubler/.hashcache")
}

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
    
    /// Check files by name
    #[arg(short = 'n', long)]
    pub name: bool,
    
    /// Check files by size
    #[arg(long)]
    pub size: bool,
    
    /// Check files by creation date
    #[arg(long)]
    pub create_date: bool,
    
    /// Check files by last modified date
    #[arg(long)]
    pub mod_date: bool,
    
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
    
    /// Directory to exclude from search (can be repeated)
    #[arg(long)]
    pub exclude_dir: Vec<PathBuf>,
    
    /// Directory treated as source (files here are kept over duplicates elsewhere)
    #[arg(long)]
    pub source_dir: Vec<PathBuf>,
    
    /// Hash read buffer size in bytes (default: 64KB)
    #[arg(long)]
    pub hash_buffer_size: Option<u64>,
    
    /// Enable hash caching to avoid re-hashing on re-scan
    #[arg(long)]
    pub hash_cache: bool,
    
    /// Directory for hash cache files
    #[arg(long)]
    pub hash_cache_dir: Option<PathBuf>,
    
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
    
    /// Create a default config file and exit (default: fundoubler.toml)
    #[arg(long, value_name = "PATH", num_args = 0..=1, default_missing_value = "fundoubler.toml")]
    pub init_config: Option<PathBuf>,
    
    /// Show only N groups of duplicates
    #[arg(long)]
    pub limit: Option<usize>,
    
    /// Skip confirmation prompts; assume "yes" to all (for scripts, CI)
    #[arg(long)]
    pub skip_confirm: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct ConfigFile {
    // Core settings (custom de/serialize for TOML: paths are strings in file)
    #[serde(default = "default_path_start", serialize_with = "path_buf_to_str", deserialize_with = "path_buf_from_str")]
    pub path_start: PathBuf,
    #[serde(serialize_with = "opt_path_buf_to_str", deserialize_with = "opt_path_buf_from_str")]
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
    #[serde(default, serialize_with = "vec_path_to_str", deserialize_with = "vec_path_from_str")]
    pub exclude_dirs: Vec<PathBuf>,
    #[serde(default, serialize_with = "vec_path_to_str", deserialize_with = "vec_path_from_str")]
    pub source_dirs: Vec<PathBuf>,
    
    #[serde(default = "default_hash_buffer_size")]
    pub hash_buffer_size: u64,
    
    pub hash_cache: bool,
    #[serde(default = "default_hash_cache_dir", serialize_with = "path_buf_to_str", deserialize_with = "path_buf_from_str")]
    pub hash_cache_dir: PathBuf,
    
    // Output control
    pub sort_orders: Vec<SortOrder>,
    pub limit: Option<usize>,
    pub verbose: u8,
    pub silent: bool,
    
    // Deletion
    pub delete: bool,
    pub force_delete: bool,
    pub dry_run: bool,

    #[serde(skip_serializing, default)]
    pub skip_confirm: bool
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
            exclude_dirs: Vec::new(),
            source_dirs: Vec::new(),
            hash_buffer_size: DEFAULT_HASH_BUFFER_SIZE,
            hash_cache: false,
            hash_cache_dir: default_hash_cache_dir(),
            sort_orders: vec![SortOrder::SizeDesc, SortOrder::Name],
            limit: None,
            verbose: 0,
            silent: false,
            delete: false,
            force_delete: false,
            dry_run: false,
            skip_confirm: false,
        }
    }
}

impl ConfigFile {
    /// Build config from CLI. If `--config <path>` is set, load and merge that TOML file first (CLI overrides file).
    pub fn from_cli(cli: &CliOptions) -> crate::error::Result<Self> {
        let mut config = if let Some(path) = &cli.config {
            let contents = fs::read_to_string(path).map_err(|e| {
                crate::error::AppError::Config(format!("Failed to read config file '{}': {}", path.display(), e))
            })?;
            toml::from_str(&contents).map_err(|e| {
                crate::error::AppError::Config(format!("Invalid config file '{}': {}", path.display(), e))
            })?
        } else {
            Self::default()
        };

        // Overlay CLI options (CLI overrides config file)
        config.path_start = cli.path_start.clone();
        if cli.output.is_some() {
            config.output = cli.output.clone();
        }

        // If user passed any comparison flag, use ONLY those (don't add to defaults).
        // This makes e.g. `--size` fast (no hashing) instead of adding to size+xxh3.
        let cli_specified_comparison = cli.name || cli.size || cli.create_date || cli.mod_date
            || cli.content || cli.md5 || cli.sha512 || cli.xxh3;

        if cli_specified_comparison {
            config.compare_by_name = cli.name;
            config.compare_by_size = cli.size;
            config.compare_by_created = cli.create_date;
            config.compare_by_modified = cli.mod_date;
            if cli.content {
                config.compare_by_md5 = true;
                config.compare_by_sha512 = true;
                config.compare_by_xxh3 = true;
            } else {
                config.compare_by_md5 = cli.md5;
                config.compare_by_sha512 = cli.sha512;
                config.compare_by_xxh3 = cli.xxh3;
            }
        } else {
            // No CLI comparison flags: keep config file / default values, additive overlay
            if cli.content {
                config.compare_by_md5 = true;
                config.compare_by_sha512 = true;
                config.compare_by_xxh3 = true;
            }
            if cli.md5 {
                config.compare_by_md5 = true;
            }
            if cli.sha512 {
                config.compare_by_sha512 = true;
            }
            if cli.xxh3 {
                config.compare_by_xxh3 = true;
            }
        }

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
        if !cli.exclude_dir.is_empty() {
            config.exclude_dirs = cli.exclude_dir.clone();
        }
        if !cli.source_dir.is_empty() {
            config.source_dirs = cli.source_dir.clone();
        }
        if let Some(buf) = cli.hash_buffer_size {
            config.hash_buffer_size = buf;
        }
        if cli.hash_cache {
            config.hash_cache = true;
        }
        if let Some(dir) = &cli.hash_cache_dir {
            config.hash_cache_dir = dir.clone();
        }
        if !cli.sort.is_empty() {
            config.sort_orders = cli.sort.clone();
        }
        if let Some(limit) = cli.limit {
            config.limit = Some(limit);
        }

        config.skip_confirm = cli.skip_confirm;

        Ok(config)
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