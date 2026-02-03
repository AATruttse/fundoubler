//! Hash cache for avoiding re-computation on re-scans.
//! Cache key: (path, size, mtime) - file identity.
//! Cache value: hashes per algorithm.

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::sync::RwLock;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

/// Cache entry: (path, size, mtime) -> hashes.
#[derive(Default, Serialize, Deserialize)]
struct CacheFile {
    #[serde(default)]
    entries: HashMap<String, CachedHashes>,
}

#[derive(Default, Serialize, Deserialize)]
struct CachedHashes {
    #[serde(skip_serializing_if = "Option::is_none")]
    md5: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    sha512: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    xxh3: Option<String>,
}

fn cache_key(path: &Path, size: u64, mtime: Option<SystemTime>) -> String {
    let mtime_nanos = mtime
        .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{}|{}|{}", path.display(), size, mtime_nanos)
}

/// Thread-safe hash cache.
pub struct HashCache {
    inner: RwLock<CacheFile>,
    path: PathBuf,
}

impl HashCache {
    /// Load cache from directory. Creates dir if needed. Returns empty cache if file missing/invalid.
    pub fn load(cache_dir: &Path) -> Self {
        let path = cache_dir.join("cache.json");
        let (cache, loaded) = if path.exists() {
            match fs::read_to_string(&path).and_then(|s| {
                serde_json::from_str(&s)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
            }) {
                Ok(c) => (c, true),
                Err(_) => (CacheFile::default(), false),
            }
        } else {
            (CacheFile::default(), false)
        };
        if loaded {
            crate::log::log_debug(&format!(
                "Hash cache loaded from {} ({} entries)",
                path.display(),
                cache.entries.len()
            ));
        }
        Self {
            inner: RwLock::new(cache),
            path,
        }
    }

    /// Get cached hash if present and valid.
    pub fn get(
        &self,
        path: &Path,
        size: u64,
        mtime: Option<SystemTime>,
        algorithm: &str,
    ) -> Option<String> {
        let key = cache_key(path, size, mtime);
        let cache = self.inner.read().ok()?;
        let hashes = cache.entries.get(&key)?;
        match algorithm {
            "md5" => hashes.md5.clone(),
            "sha512" => hashes.sha512.clone(),
            "xxh3" => hashes.xxh3.clone(),
            _ => None,
        }
    }

    /// Store a computed hash.
    pub fn insert(
        &self,
        path: &Path,
        size: u64,
        mtime: Option<SystemTime>,
        algorithm: &str,
        hash: String,
    ) {
        let key = cache_key(path, size, mtime);
        if let Ok(mut cache) = self.inner.write() {
            let entry = cache.entries.entry(key).or_default();
            match algorithm {
                "md5" => entry.md5 = Some(hash),
                "sha512" => entry.sha512 = Some(hash),
                "xxh3" => entry.xxh3 = Some(hash),
                _ => {}
            }
        }
    }

    /// Persist cache to disk. Call after scan.
    pub fn save(&self) -> std::io::Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let cache = self
            .inner
            .read()
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::Other, "cache lock poisoned"))?;
        let json = serde_json::to_string_pretty(&*cache)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        fs::write(&self.path, json)
    }
}
