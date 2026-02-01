use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use walkdir::WalkDir;

use crate::check::{CheckOptions, calculate_hash};
use crate::config::ConfigFile;
use crate::error::Result;
use crate::hash_cache::HashCache;

pub struct FileScanner {
    config: Arc<ConfigFile>,
    progress_bar: Option<ProgressBar>,
    cache: Option<Arc<HashCache>>,
}

impl FileScanner {
    pub fn new(config: &ConfigFile, show_progress: bool) -> Self {
        let progress_bar = if show_progress && !config.silent {
            let pb = ProgressBar::new_spinner();
            pb.set_style(
                ProgressStyle::default_spinner()
                    .template("{spinner:.green} [{elapsed_precise}] {msg}")
                    .unwrap(),
            );
            Some(pb)
        } else {
            None
        };
        
        let cache = if config.hash_cache {
            Some(Arc::new(HashCache::load(&config.hash_cache_dir)))
        } else {
            None
        };
        
        Self {
            config: Arc::new(config.clone()),
            progress_bar,
            cache,
        }
    }
    
    pub fn scan(&self) -> Result<Vec<FileGroup>> {
        let config = self.config.clone();
        let exclude_dirs = config.exclude_dirs.clone();
        let path_start = config.path_start.clone();
        
        crate::log::log_debug(&format!("Scanning directory: {}", config.path_start.display()));

        // Collect all files first (this is IO bound)
        let entries: Vec<_> = WalkDir::new(&config.path_start)
            .into_iter()
            .filter_entry(|e| {
                if e.file_type().is_dir() {
                    !path_is_excluded(e.path(), &exclude_dirs, &path_start)
                } else {
                    true
                }
            })
            .filter_map(|e| e.ok())
            .filter(|e| !e.file_type().is_dir())
            .collect();
        
        crate::log::log_debug(&format!("Collected {} files to process", entries.len()));

        if let Some(pb) = &self.progress_bar {
            pb.set_length(entries.len() as u64);
            pb.set_message("Scanning files...");
        }
        
        // Process files in parallel (Ok(Some) = include, Ok(None) = filtered, Err = real error)
        let results: Vec<_> = entries
            .par_iter()
            .map(|entry| {
                if let Some(pb) = &self.progress_bar {
                    pb.inc(1);
                }
                self.process_file(entry)
            })
            .collect();
        
        let file_infos: Vec<_> = match results.into_iter().collect::<Result<Vec<_>>>() {
            Ok(v) => v,
            Err(e) => {
                crate::log::log_error(&format!("File processing error: {}", e));
                return Err(e);
            }
        }
        .into_iter()
        .filter_map(|x| x)
        .collect();
        
        if let Some(pb) = &self.progress_bar {
            pb.finish_with_message("Scanning complete");
        }
        
        if let Some(cache) = &self.cache {
            if let Err(e) = cache.save() {
                crate::log::log_error(&format!("Hash cache save failed: {}", e));
            } else {
                crate::log::log_debug("Hash cache saved");
            }
        }
        
        // Group duplicates
        crate::log::log_debug(&format!("Grouped {} files into unique keys, filtering duplicates", file_infos.len()));
        let mut groups: HashMap<CheckOptions, Vec<PathBuf>> = HashMap::new();

        for (key, path) in file_infos {
            groups.entry(key).or_default().push(path);
        }

        // Filter groups with duplicates and apply limit
        let source_dirs = config.source_dirs.clone();
        let path_start = config.path_start.clone();
        let mut result: Vec<_> = groups
            .into_iter()
            .filter(|(_, paths)| paths.len() > 1)
            .map(|(key, mut paths)| {
                sort_paths_source_first(&mut paths, &source_dirs, &path_start);
                FileGroup { key, paths }
            })
            .collect();
        
        // Sort groups
        if !config.sort_orders.is_empty() {
            result.sort_by(|a, b| crate::check::compare(&config, &a.key, &b.key));
        }
        
        // Apply limit
        if let Some(limit) = config.limit {
            result.truncate(limit);
        }
        
        Ok(result)
    }
    
    /// Returns Ok(Some(...)) to include, Ok(None) if filtered out, Err for real errors.
    fn process_file(&self, entry: &walkdir::DirEntry) -> Result<Option<(CheckOptions, PathBuf)>> {
        let path = entry.path().to_path_buf();

        if self.config.verbose > 2 {
            println!("{:#?}", path);
        }

        let metadata = entry.metadata()?;
        
        // Apply filters (return None = excluded by design, not an error)
        if metadata.len() < self.config.min_size 
            || metadata.len() > self.config.max_size 
        {
            return Ok(None);
        }
        
        if let Some(filter) = &self.config.name_filter {
            let re = regex::Regex::new(filter)?;
            if !re.is_match(&path.to_string_lossy()) {
                return Ok(None);
            }
        }
        
        // Build key based on enabled comparison criteria
        let mut key = CheckOptions::new();
        
        if self.config.compare_by_name {
            key.name = Some(entry.file_name().to_string_lossy().to_string());
        }
        
        if self.config.compare_by_size {
            key.size = Some(metadata.len());
        }
        
        if self.config.compare_by_created {
            key.created = metadata.created().ok();
        }
        
        if self.config.compare_by_modified {
            key.modified = metadata.modified().ok();
        }
        
        // Calculate hashes if needed (use cache when enabled)
        let buf_size = self.config.hash_buffer_size.try_into().unwrap_or(usize::MAX).max(256);
        let mtime = metadata.modified().ok();
        let size = metadata.len();
        
        key.md5 = if self.config.compare_by_md5 {
            Some(self.get_or_compute_hash(&path, size, mtime, "md5", buf_size)?)
        } else {
            None
        };
        key.sha512 = if self.config.compare_by_sha512 {
            Some(self.get_or_compute_hash(&path, size, mtime, "sha512", buf_size)?)
        } else {
            None
        };
        key.xxh3 = if self.config.compare_by_xxh3 {
            Some(self.get_or_compute_hash(&path, size, mtime, "xxh3", buf_size)?)
        } else {
            None
        };
        
        Ok(Some((key, path)))
    }
    
    fn get_or_compute_hash(
        &self,
        path: &PathBuf,
        size: u64,
        mtime: Option<std::time::SystemTime>,
        algorithm: &str,
        buf_size: usize,
    ) -> Result<String> {
        if let Some(cache) = &self.cache {
            if let Some(cached) = cache.get(path, size, mtime, algorithm) {
                return Ok(cached);
            }
        }
        let hash = calculate_hash(path, algorithm, buf_size)?;
        if let Some(cache) = &self.cache {
            cache.insert(path, size, mtime, algorithm, hash.clone());
        }
        Ok(hash)
    }
}

#[derive(Debug)]
pub struct FileGroup {
    pub key: CheckOptions,
    pub paths: Vec<PathBuf>,
}

/// Returns true if the given directory path should be excluded from the scan.
fn path_is_excluded(dir_path: &Path, exclude_dirs: &[PathBuf], path_start: &Path) -> bool {
    if exclude_dirs.is_empty() {
        return false;
    }
    let d = normalize_path(dir_path);
    for exclude in exclude_dirs {
        let e = if exclude.is_absolute() {
            normalize_path(exclude)
        } else {
            let resolved = path_start.join(exclude);
            normalize_path(&resolved)
        };
        if d == e || d.starts_with(&format!("{}/", e)) || d.starts_with(&e) {
            return true;
        }
    }
    false
}

fn normalize_path(p: &Path) -> String {
    p.to_string_lossy().replace('\\', "/")
}

/// Sort paths so files in source_dirs come first (and will be kept during deletion).
fn sort_paths_source_first(paths: &mut [PathBuf], source_dirs: &[PathBuf], path_start: &Path) {
    if source_dirs.is_empty() {
        paths.sort();
        return;
    }
    paths.sort_by(|a, b| {
        let a_in = path_is_in_source(a, source_dirs, path_start);
        let b_in = path_is_in_source(b, source_dirs, path_start);
        match (a_in, b_in) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.cmp(b),
        }
    });
}

fn path_is_in_source(file_path: &Path, source_dirs: &[PathBuf], path_start: &Path) -> bool {
    if source_dirs.is_empty() {
        return false;
    }
    let parent = file_path.parent().unwrap_or(file_path);
    let p = normalize_path(parent);
    for source in source_dirs {
        let s = if source.is_absolute() {
            normalize_path(source)
        } else {
            let resolved = path_start.join(source);
            normalize_path(&resolved)
        };
        if p == s || p.starts_with(&format!("{}/", s)) || p.starts_with(&s) {
            return true;
        }
    }
    false
}