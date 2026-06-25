//! Configuration file support.
//!
//! Looks for `~/.config/vanity/config.toml` (XDG standard) or `~/.vanityrc`.
//! Can be overridden via `$VANITY_CONFIG` environment variable.
//!
//! CLI flags always win over config-file values.

use std::path::PathBuf;

use serde::Deserialize;

/// Top-level config structure.
#[derive(Debug, Default, Deserialize)]
pub struct Config {
    /// Default thread count (overridable by `-T` / `--threads`).
    pub threads: Option<usize>,
    /// Bark API key for iOS push notifications.
    pub bark_key: Option<String>,
    /// Default address type: legacy, p2sh, segwit, taproot.
    pub address_type: Option<String>,
}

impl Config {
    /// Load config from the first available location.
    pub fn load() -> Self {
        let paths = candidate_paths();
        for path in &paths {
            if path.exists() {
                let raw = match std::fs::read_to_string(path) {
                    Ok(s) => s,
                    Err(_) => continue,
                };
                match toml::from_str(&raw) {
                    Ok(cfg) => {
                        eprintln!("[config] loaded {}", path.display());
                        return cfg;
                    }
                    Err(e) => {
                        eprintln!("[config] parse error in {}: {e}", path.display());
                    }
                }
            }
        }
        // No valid config found → create default one.
        if let Some(path) = paths.first() {
            Self::create_default(path);
        }
        Config::default()
    }

    /// Write a commented default config file if it doesn't exist.
    fn create_default(path: &std::path::Path) {
        if path.exists() {
            return;
        }
        let content = r#"# vanity configuration
# see https://github.com/your-username/vanity for details

# Number of worker threads (default: all logical cores)
# threads = 8

# Bark API key for iOS push notifications
# bark_key = "YOUR_KEY_HERE"

# Default address type: legacy, p2sh, segwit, taproot
# address_type = "legacy"
"#;
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        match std::fs::write(path, content) {
            Ok(_) => eprintln!("[config] created default {}", path.display()),
            Err(e) => eprintln!("[config] failed to create {}: {e}", path.display()),
        }
    }
}

/// Return config file paths in priority order.
fn candidate_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    // 1. XDG standard: $XDG_CONFIG_HOME/vanity/config.toml
    let xdg = std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
            PathBuf::from(home).join(".config")
        });
    paths.push(xdg.join("vanitygen").join("config.toml"));

    paths
}
