//! Delete log: records of deleted files and their kept duplicates for restore.
//! Format: pairs of lines "deleted:<path>" and "source:<path>" per record.
//! Files: logs_dir/del_logs/YYYYMMDDHHMMSSfundel.log

use std::fs::{self, File};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use chrono::Local;

const DEL_LOG_PREFIX: &str = "deleted:";
const SOURCE_PREFIX: &str = "source:";

/// Path to the del_logs directory inside logs_dir.
pub fn del_logs_dir(logs_dir: &Path) -> PathBuf {
    logs_dir.join("del_logs")
}

/// Create a new delete log file and return (path, writer).
pub fn create_del_log(logs_dir: &Path) -> std::io::Result<(PathBuf, File)> {
    let dir = del_logs_dir(logs_dir);
    fs::create_dir_all(&dir)?;
    let now = Local::now();
    let filename = format!("{}fundel.log", now.format("%Y%m%d%H%M%S"));
    let path = dir.join(filename);
    let file = File::create(&path)?;
    Ok((path, file))
}

/// Write a delete record: deleted_file and source_file (the kept duplicate).
pub fn write_record(file: &mut File, deleted: &Path, source: &Path) -> std::io::Result<()> {
    writeln!(file, "{}{}", DEL_LOG_PREFIX, deleted.display())?;
    writeln!(file, "{}{}", SOURCE_PREFIX, source.display())?;
    file.flush()
}

/// Parse a delete log file, return Vec<(deleted_path, source_path)>.
pub fn parse_del_log(path: &Path) -> std::io::Result<Vec<(PathBuf, PathBuf)>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut records = Vec::new();
    let mut deleted: Option<PathBuf> = None;

    for line in reader.lines() {
        let line = line?;
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(p) = line.strip_prefix(DEL_LOG_PREFIX) {
            deleted = Some(PathBuf::from(p.trim()));
        } else if let Some(p) = line.strip_prefix(SOURCE_PREFIX) {
            if let Some(d) = deleted.take() {
                records.push((d, PathBuf::from(p.trim())));
            }
        }
    }

    Ok(records)
}

/// Find the most recent fundel.log in del_logs_dir. Returns None if dir doesn't exist or is empty.
pub fn find_latest_del_log(logs_dir: &Path) -> std::io::Result<Option<PathBuf>> {
    let dir = del_logs_dir(logs_dir);
    if !dir.exists() {
        return Ok(None);
    }
    let mut latest: Option<(PathBuf, std::time::SystemTime)> = None;
    for entry in fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.ends_with("fundel.log") {
                    if let Ok(meta) = entry.metadata() {
                        let modified = meta.modified()?;
                        if latest.as_ref().map_or(true, |(_, t)| modified > *t) {
                            latest = Some((path, modified));
                        }
                    }
                }
            }
        }
    }
    Ok(latest.map(|(p, _)| p))
}
