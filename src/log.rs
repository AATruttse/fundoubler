//! Logging to file with configurable level and directory.
//! Level 0 = off, 1 = error, 2 = error+info, 3+ = error+info+debug.
//! Log file: logs_dir/YYYYMMDDHHMMSSfun.log

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

use chrono::Local;

/// Log level: 0=off, 1=error, 2=info, 3=debug
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogLevel {
    Off = 0,
    Error = 1,
    Info = 2,
    Debug = 3,
}

impl LogLevel {
    pub fn from_u8(v: u8) -> Self {
        match v {
            0 => LogLevel::Off,
            1 => LogLevel::Error,
            2 => LogLevel::Info,
            _ => LogLevel::Debug,
        }
    }
}

struct LoggerState {
    level: LogLevel,
    file_path: Option<PathBuf>,
    file_handle: Option<fs::File>,
}

static LOGGER: std::sync::OnceLock<Mutex<LoggerState>> = std::sync::OnceLock::new();

/// Initialize the global logger. Call after config is loaded.
/// Creates logs_dir if it does not exist. Re-initializes if called again.
pub fn init(level: u8, logs_dir: &std::path::Path) -> std::io::Result<()> {
    let log_level = LogLevel::from_u8(level);
    let state = if log_level == LogLevel::Off {
        LoggerState {
            level: LogLevel::Off,
            file_path: None,
            file_handle: None,
        }
    } else {
        fs::create_dir_all(logs_dir)?;
        let now = Local::now();
        let filename = format!("{}fun.log", now.format("%Y%m%d%H%M%S"));
        let file_path = logs_dir.join(&filename);
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&file_path)?;
        LoggerState {
            level: log_level,
            file_path: Some(file_path),
            file_handle: Some(file),
        }
    };

    let m = LOGGER.get_or_init(|| Mutex::new(LoggerState {
        level: LogLevel::Off,
        file_path: None,
        file_handle: None,
    }));
    *m.lock().expect("logger mutex poisoned") = state;
    Ok(())
}

/// Return the current log file path if logging is active. For tests.
#[doc(hidden)]
pub fn current_log_path() -> Option<PathBuf> {
    LOGGER.get().and_then(|m| {
        m.lock().ok().and_then(|s| s.file_path.clone())
    })
}

fn write_log(level: LogLevel, msg: &str) {
    if let Some(m) = LOGGER.get() {
        if let Ok(mut state) = m.lock() {
            // Only write if our level is <= configured level (Error=1, Info=2, Debug=3)
            if state.level == LogLevel::Off || (level as u8) > (state.level as u8) {
                return;
            }
            if let Some(ref mut f) = state.file_handle {
                let now = Local::now();
                let level_str = match level {
                    LogLevel::Error => "ERROR",
                    LogLevel::Info => "INFO",
                    LogLevel::Debug => "DEBUG",
                    LogLevel::Off => return,
                };
                let _ = writeln!(f, "[{}] [{}] {}", now.format("%Y-%m-%d %H:%M:%S%.3f"), level_str, msg);
                let _ = f.flush();
            }
        }
    }
}

/// Log an error (level 1+).
pub fn log_error(msg: &str) {
    write_log(LogLevel::Error, msg);
}

/// Log info (level 2+).
pub fn log_info(msg: &str) {
    write_log(LogLevel::Info, msg);
}

/// Log debug (level 3+).
pub fn log_debug(msg: &str) {
    write_log(LogLevel::Debug, msg);
}

/// Log error with format!-like args.
#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => {
        $crate::log::log_error(&format!($($arg)*))
    };
}

/// Log info with format!-like args.
#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => {
        $crate::log::log_info(&format!($($arg)*))
    };
}

/// Log debug with format!-like args.
#[macro_export]
macro_rules! log_debug {
    ($($arg:tt)*) => {
        $crate::log::log_debug(&format!($($arg)*))
    };
}

/// Reset logger state (for tests between runs).
#[doc(hidden)]
pub fn reset() {
    if let Some(m) = LOGGER.get() {
        if let Ok(mut state) = m.lock() {
            state.level = LogLevel::Off;
            state.file_path = None;
            state.file_handle = None;
        }
    }
}
