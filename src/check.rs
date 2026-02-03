use std::cmp::Ordering;
use std::fmt;
use std::path::PathBuf;
use std::time::SystemTime;

use chrono::{DateTime, Utc};
use md5; // Added
use serde::{Deserialize, Serialize};

use crate::config::SortOrder;

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct CheckOptions {
    pub name: Option<String>,
    pub size: Option<u64>,
    pub created: Option<SystemTime>,
    pub modified: Option<SystemTime>,
    pub md5: Option<String>,
    pub sha512: Option<String>,
    pub xxh3: Option<String>,
}

impl CheckOptions {
    pub fn new() -> Self {
        Self {
            name: None,
            size: None,
            created: None,
            modified: None,
            md5: None,
            sha512: None,
            xxh3: None,
        }
    }
}

/// Compare two CheckOptions based on multiple criteria
pub fn compare(
    cfg: &crate::config::ConfigFile,
    opt0: &CheckOptions,
    opt1: &CheckOptions,
) -> Ordering {
    for order in &cfg.sort_orders {
        match order {
            SortOrder::Name => {
                let cmp = opt0.name.cmp(&opt1.name);
                if cmp != Ordering::Equal {
                    return cmp;
                }
            }
            SortOrder::NameDesc => {
                let cmp = opt1.name.cmp(&opt0.name);
                if cmp != Ordering::Equal {
                    return cmp;
                }
            }
            SortOrder::Size => {
                let cmp = opt0.size.cmp(&opt1.size);
                if cmp != Ordering::Equal {
                    return cmp;
                }
            }
            SortOrder::SizeDesc => {
                let cmp = opt1.size.cmp(&opt0.size);
                if cmp != Ordering::Equal {
                    return cmp;
                }
            }
            SortOrder::Created => {
                let cmp = opt0.created.cmp(&opt1.created);
                if cmp != Ordering::Equal {
                    return cmp;
                }
            }
            SortOrder::CreatedDesc => {
                let cmp = opt1.created.cmp(&opt0.created);
                if cmp != Ordering::Equal {
                    return cmp;
                }
            }
            SortOrder::Modified => {
                let cmp = opt0.modified.cmp(&opt1.modified);
                if cmp != Ordering::Equal {
                    return cmp;
                }
            }
            SortOrder::ModifiedDesc => {
                let cmp = opt1.modified.cmp(&opt0.modified);
                if cmp != Ordering::Equal {
                    return cmp;
                }
            }
        }
    }

    Ordering::Equal
}

impl fmt::Display for CheckOptions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut parts = Vec::new();

        if let Some(name) = &self.name {
            parts.push(format!("Name: {}", name));
        }

        if let Some(size) = &self.size {
            parts.push(format!("Size: {} bytes", size));
        }

        if let Some(created) = &self.created {
            let datetime: DateTime<Utc> = (*created).into();
            parts.push(format!("Created: {}", datetime.format("%Y-%m-%d %H:%M:%S")));
        }

        if let Some(modified) = &self.modified {
            let datetime: DateTime<Utc> = (*modified).into();
            parts.push(format!(
                "Modified: {}",
                datetime.format("%Y-%m-%d %H:%M:%S")
            ));
        }

        if let Some(md5) = &self.md5 {
            parts.push(format!("MD5: {}", md5));
        }

        if let Some(sha512) = &self.sha512 {
            parts.push(format!("SHA512: {}", sha512));
        }

        if let Some(xxh3) = &self.xxh3 {
            parts.push(format!("XXH3: {}", xxh3));
        }

        write!(f, "{}", parts.join(" | "))
    }
}

/// Calculate file hash using specified algorithm
pub fn calculate_hash(
    path: &PathBuf,
    algorithm: &str,
    buffer_size: usize,
) -> std::io::Result<String> {
    use std::fs::File;
    use std::io::Read;

    let mut file = File::open(path)?;
    let mut buffer = vec![0; buffer_size];

    match algorithm {
        "md5" => {
            let mut context = md5::Context::new();
            loop {
                let count = file.read(&mut buffer)?;
                if count == 0 {
                    break;
                }
                context.consume(&buffer[..count]);
            }

            Ok(format!("{:x}", context.finalize()))
        }
        "sha512" => {
            use sha2::{Digest, Sha512};
            let mut hasher = Sha512::new();
            loop {
                let count = file.read(&mut buffer)?;
                if count == 0 {
                    break;
                }
                hasher.update(&buffer[..count]);
            }
            Ok(format!("{:x}", hasher.finalize()))
        }
        "xxh3" => {
            use xxhash_rust::xxh3::Xxh3;
            let mut hasher = Xxh3::new();
            loop {
                let count = file.read(&mut buffer)?;
                if count == 0 {
                    break;
                }
                hasher.update(&buffer[..count]);
            }
            Ok(format!("{:x}", hasher.digest()))
        }
        _ => Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("Unknown algorithm: {}", algorithm),
        )),
    }
}
