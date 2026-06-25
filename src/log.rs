//! Simple file logging.
//!
//! Writes timestamped log entries to `vanity.log` in the working
//! directory.  No external dependencies – uses raw `std::fs::OpenOptions`.

use std::fs::OpenOptions;
use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};

const LOG_FILE: &str = "vanity.log";

/// Log levels.
#[derive(Debug, Clone, Copy)]
pub enum Level {
    Info,
    Warn,
    Error,
}

impl Level {
    fn as_str(&self) -> &'static str {
        match self {
            Level::Info => "INFO",
            Level::Warn => "WARN",
            Level::Error => "ERROR",
        }
    }
}

/// Write a log entry.  Creates/appends to `vanity.log`.
pub fn log(level: Level, msg: &str) {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(LOG_FILE) {
        let _ = writeln!(file, "[{timestamp}] [{}] {}", level.as_str(), msg);
    }
}

/// Convenience wrappers.
pub fn info(msg: &str) {
    log(Level::Info, msg);
}
pub fn warn(msg: &str) {
    log(Level::Warn, msg);
}
pub fn error(msg: &str) {
    log(Level::Error, msg);
}
