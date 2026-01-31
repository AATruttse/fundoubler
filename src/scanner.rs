use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use walkdir::WalkDir;

use crate::check::{CheckOptions, calculate_hash};
use crate::config::ConfigFile;
use crate::error::Result;

pub struct FileScanner {
    config: Arc<ConfigFile>,
    progress_bar: Option<ProgressBar>,
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
        
        Self {
            config: Arc::new(config.clone()),
            progress_bar,
        }
    }
    
    pub fn scan(&self) -> Result<Vec<FileGroup>> {
        let config = self.config.clone();
        
        // Collect all files first (this is IO bound)
        let entries: Vec<_> = WalkDir::new(&config.path_start)
            .into_iter()
            .filter_map(|e| e.ok())  // Fixed: was filter_map(Result::ok)
            .filter(|e| !e.file_type().is_dir())
            .collect();
        
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
        
        let file_infos: Vec<_> = results
            .into_iter()
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .filter_map(|x| x)
            .collect();
        
        if let Some(pb) = &self.progress_bar {
            pb.finish_with_message("Scanning complete");
        }
        
        // Group duplicates
        let mut groups: HashMap<CheckOptions, Vec<PathBuf>> = HashMap::new();
        
        for (key, path) in file_infos {
            groups.entry(key).or_default().push(path);
        }
        
        // Filter groups with duplicates and apply limit
        let mut result: Vec<_> = groups
            .into_iter()
            .filter(|(_, paths)| paths.len() > 1)
            .map(|(key, paths)| FileGroup { key, paths })
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
        
        // Calculate hashes if needed
        let buf_size = self.config.hash_buffer_size.try_into().unwrap_or(usize::MAX).max(256);
        if self.config.compare_by_md5 {
            key.md5 = Some(calculate_hash(&path, "md5", buf_size)?);
        }
        if self.config.compare_by_sha512 {
            key.sha512 = Some(calculate_hash(&path, "sha512", buf_size)?);
        }
        if self.config.compare_by_xxh3 {
            key.xxh3 = Some(calculate_hash(&path, "xxh3", buf_size)?);
        }
        
        Ok(Some((key, path)))
    }
}

#[derive(Debug)]
pub struct FileGroup {
    pub key: CheckOptions,
    pub paths: Vec<PathBuf>,
}