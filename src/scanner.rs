use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use walkdir::WalkDir;

use crate::check::{calculate_hash, CheckOptions};
use crate::config::ConfigFile;
use crate::error::Result;
use crate::filters;
use crate::hash_cache::HashCache;

pub struct FileScanner {
    config: Arc<ConfigFile>,
    progress_bar: Option<ProgressBar>,
    cache: Option<Arc<HashCache>>,
}

#[derive(Clone)]
struct CandidateFile {
    path: PathBuf,
    base_key: CheckOptions,
    size: u64,
    mtime: Option<std::time::SystemTime>,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct PreGroupKey {
    base_key: CheckOptions,
    // When any hash comparison is enabled, equal size is required before hashing.
    size_gate: Option<u64>,
}

impl FileScanner {
    pub fn new(config: &ConfigFile, show_progress: bool) -> Self {
        let progress_bar = if show_progress && !config.silent && !config.no_progress_bar {
            let pb = ProgressBar::new(0);
            pb.set_style(
                ProgressStyle::default_bar()
                    .template("{spinner:.green} [{bar:40}] {pos}/{len} {msg}")
                    .unwrap()
                    .progress_chars("=>-"),
            );
            pb.enable_steady_tick(Duration::from_millis(100));
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
        let search_dirs = config.search_dirs.clone();

        // Build list of roots to scan: path_start + search_dirs (dedupe)
        let mut roots: Vec<PathBuf> = vec![path_start.clone()];
        for d in &search_dirs {
            let resolved = if d.is_absolute() {
                d.clone()
            } else {
                path_start.join(d)
            };
            let r = normalize_path(&resolved);
            if !roots.iter().any(|x| normalize_path(x) == r) {
                roots.push(resolved);
            }
        }

        crate::log::log_debug(&format!(
            "Scanning directories: {:?}",
            roots.iter().map(|p| p.display().to_string()).collect::<Vec<_>>()
        ));

        // Collect all files from each root (dedupe by path)
        let mut seen = std::collections::HashSet::<String>::new();
        let mut entries: Vec<walkdir::DirEntry> = Vec::new();
        for root in &roots {
            for e in WalkDir::new(root)
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
            {
                let key = normalize_path(e.path());
                if seen.insert(key) {
                    entries.push(e);
                }
            }
        }

        crate::log::log_debug(&format!("Collected {} files to process", entries.len()));

        if let Some(pb) = &self.progress_bar {
            let len = entries.len().max(1) as u64;
            pb.set_length(len);
            pb.set_message("Scanning files...");
        }

        // Process files in parallel (Ok(Some) = include, Ok(None) = filtered, Err = real error)
        let results: Vec<_> = entries
            .par_iter()
            .map(|entry| {
                let res = self.process_file_metadata(entry);
                if let Some(pb) = &self.progress_bar {
                    // Count completed files, not scheduled files, so progress reflects real work.
                    pb.inc(1);
                }
                res
            })
            .collect();

        let candidates: Vec<_> = match results.into_iter().collect::<Result<Vec<_>>>() {
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
            pb.disable_steady_tick();
            pb.finish_with_message("Scanning complete");
        }

        if let Some(cache) = &self.cache {
            if let Err(e) = cache.save() {
                crate::log::log_error(&format!("Hash cache save failed: {}", e));
            } else {
                crate::log::log_debug("Hash cache saved");
            }
        }

        let use_hashes = self.config.compare_by_md5 || self.config.compare_by_sha512 || self.config.compare_by_xxh3;

        // Pre-group using cheap criteria (+ size gate when hashes are enabled).
        // This avoids hashing files that cannot possibly be duplicates.
        let mut pre_groups: HashMap<PreGroupKey, Vec<CandidateFile>> = HashMap::new();
        for candidate in candidates {
            let pre_key = PreGroupKey {
                base_key: candidate.base_key.clone(),
                size_gate: if use_hashes { Some(candidate.size) } else { None },
            };
            pre_groups.entry(pre_key).or_default().push(candidate);
        }

        // Final grouping with hashes (lazy: only inside pre-groups that have >1 files).
        crate::log::log_debug(&format!(
            "Grouped files into {} pre-keys; building final duplicate keys",
            pre_groups.len()
        ));
        let mut groups: HashMap<CheckOptions, Vec<PathBuf>> = HashMap::new();

        let buf_size = self
            .config
            .hash_buffer_size
            .try_into()
            .unwrap_or(usize::MAX)
            .max(256);

        for (_pre_key, files) in pre_groups {
            // No need to hash singleton pre-groups.
            if use_hashes && files.len() <= 1 {
                continue;
            }

            for candidate in files {
                let mut key = candidate.base_key;
                if self.config.compare_by_md5 {
                    key.md5 = Some(self.get_or_compute_hash(
                        &candidate.path,
                        candidate.size,
                        candidate.mtime,
                        "md5",
                        buf_size,
                    )?);
                }
                if self.config.compare_by_sha512 {
                    key.sha512 = Some(self.get_or_compute_hash(
                        &candidate.path,
                        candidate.size,
                        candidate.mtime,
                        "sha512",
                        buf_size,
                    )?);
                }
                if self.config.compare_by_xxh3 {
                    key.xxh3 = Some(self.get_or_compute_hash(
                        &candidate.path,
                        candidate.size,
                        candidate.mtime,
                        "xxh3",
                        buf_size,
                    )?);
                }
                groups.entry(key).or_default().push(candidate.path);
            }
        }

        // Filter groups with duplicates; when search_dirs is set, only keep groups that have at least one file in search_dirs
        let source_dirs = config.source_dirs.clone();
        let path_start = config.path_start.clone();
        let unique = config.unique;
        let mut result: Vec<_> = groups
            .into_iter()
            .filter(|(_, paths)| paths.len() > 1)
            .filter(|(_, paths)| {
                search_dirs.is_empty()
                    || paths
                        .iter()
                        .any(|p| path_is_in_search_dirs(p, &search_dirs, &path_start))
            })
            .filter(|(_, paths)| {
                // --unique: only groups where NO file is in source_dirs (files unique to search area)
                if !unique {
                    return true;
                }
                !paths
                    .iter()
                    .any(|p| path_is_in_source(p, &source_dirs, &path_start))
            })
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
    /// This stage only collects cheap metadata and non-hash key parts.
    fn process_file_metadata(&self, entry: &walkdir::DirEntry) -> Result<Option<CandidateFile>> {
        let path = entry.path().to_path_buf();

        if self.config.verbose > 2 {
            println!("{:#?}", path);
        }

        let metadata = entry.metadata()?;

        // Apply filters (return None = excluded by design, not an error)
        if metadata.len() < self.config.min_size || metadata.len() > self.config.max_size {
            return Ok(None);
        }

        if let Some(filter) = &self.config.name_filter {
            let re = regex::Regex::new(filter)?;
            if !re.is_match(&path.to_string_lossy()) {
                return Ok(None);
            }
        }

        // Time filters (Windows and Linux)
        let min_ct = self.config.min_create_time.as_ref().and_then(|s| filters::parse_datetime(s));
        let max_ct = self.config.max_create_time.as_ref().and_then(|s| filters::parse_datetime(s));
        let created = metadata.created().ok();
        if !filters::time_in_range(created, min_ct.as_ref(), max_ct.as_ref()) {
            return Ok(None);
        }
        let min_mt = self.config.min_mod_time.as_ref().and_then(|s| filters::parse_datetime(s));
        let max_mt = self.config.max_mod_time.as_ref().and_then(|s| filters::parse_datetime(s));
        let modified = metadata.modified().ok();
        if !filters::time_in_range(modified, min_mt.as_ref(), max_mt.as_ref()) {
            return Ok(None);
        }

        // User/group filters (Unix: filter; Windows: no-op)
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            let uid = metadata.uid();
            let gid = metadata.gid();
            if !filters::matches_user_filter(&path, uid, self.config.user_filter.as_deref()) {
                return Ok(None);
            }
            if !filters::matches_group_filter(&path, gid, self.config.group_filter.as_deref()) {
                return Ok(None);
            }
        }

        // Build base key based on enabled non-hash comparison criteria.
        let mut base_key = CheckOptions::new();

        if self.config.compare_by_name {
            base_key.name = Some(entry.file_name().to_string_lossy().to_string());
        }

        if self.config.compare_by_size {
            base_key.size = Some(metadata.len());
        }

        if self.config.compare_by_created {
            base_key.created = metadata.created().ok();
        }

        if self.config.compare_by_modified {
            base_key.modified = metadata.modified().ok();
        }

        Ok(Some(CandidateFile {
            path,
            base_key,
            size: metadata.len(),
            mtime: metadata.modified().ok(),
        }))
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

/// Returns true if the file is under one of search_dirs (used to restrict reported groups).
fn path_is_in_search_dirs(file_path: &Path, search_dirs: &[PathBuf], path_start: &Path) -> bool {
    if search_dirs.is_empty() {
        return false;
    }
    let parent = file_path.parent().unwrap_or(file_path);
    let p = normalize_path(parent);
    for search in search_dirs {
        let s = if search.is_absolute() {
            normalize_path(search)
        } else {
            let resolved = path_start.join(search);
            normalize_path(&resolved)
        };
        if p == s || p.starts_with(&format!("{}/", s)) || p.starts_with(&s) {
            return true;
        }
    }
    false
}
