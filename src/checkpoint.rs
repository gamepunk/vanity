//! Search progress checkpoint.
//!
//! Periodically saves the attempt counter and elapsed time so the user
//! can see how far a long-running search has progressed, even if the
//! process is interrupted.

use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::cli::AddressType;

const CHECKPOINT_FILE: &str = ".vanity_checkpoint";

/// Search parameters (used to detect stale checkpoints).
#[derive(Debug, Clone)]
pub struct SearchParams {
    pub prefix: String,
    pub address_type: AddressType,
    pub case_insensitive: bool,
    pub threads: usize,
}

/// Saved progress state.
#[derive(Debug)]
pub struct Checkpoint {
    pub params: SearchParams,
    pub attempts: u64,
    pub elapsed: Duration,
    pub timestamp: SystemTime,
}

/// Save a checkpoint to disk (overwrites any previous state).
pub fn save(params: &SearchParams, attempts: u64, elapsed: Duration) {
    let content = format!(
        "prefix={}\n\
         address_type={}\n\
         case_insensitive={}\n\
         threads={}\n\
         attempts={}\n\
         elapsed_secs={}\n\
         timestamp={}\n",
        params.prefix,
        match params.address_type {
            AddressType::Legacy => "legacy",
            AddressType::P2sh => "p2sh",
            AddressType::Segwit => "segwit",
            AddressType::Taproot => "taproot",
        },
        params.case_insensitive,
        params.threads,
        attempts,
        elapsed.as_secs_f64(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
    );
    let _ = fs::write(CHECKPOINT_FILE, &content);
}

/// Load a saved checkpoint (returns `None` if no checkpoint exists).
pub fn load() -> Option<Checkpoint> {
    let path = Path::new(CHECKPOINT_FILE);
    if !path.exists() {
        return None;
    }
    let file = fs::File::open(path).ok()?;
    let reader = BufReader::new(file);
    let mut kv = HashMap::new();
    for line in reader.lines() {
        let line = line.ok()?;
        if let Some((k, v)) = line.split_once('=') {
            kv.insert(k.to_string(), v.to_string());
        }
    }

    Some(Checkpoint {
        params: SearchParams {
            prefix: kv.get("prefix")?.clone(),
            address_type: match kv.get("address_type")?.as_str() {
                "p2sh" => AddressType::P2sh,
                "segwit" => AddressType::Segwit,
                "taproot" => AddressType::Taproot,
                _ => AddressType::Legacy,
            },
            case_insensitive: kv.get("case_insensitive")? == "true",
            threads: kv.get("threads")?.parse().ok()?,
        },
        attempts: kv.get("attempts")?.parse().ok()?,
        elapsed: Duration::from_secs_f64(kv.get("elapsed_secs")?.parse().ok()?),
        timestamp: UNIX_EPOCH + Duration::from_secs(kv.get("timestamp")?.parse().ok()?),
    })
}

/// Remove the checkpoint file (e.g. after a successful match).
pub fn clear() {
    let _ = fs::remove_file(CHECKPOINT_FILE);
}

/// Print checkpoint info to stderr and return whether the user wants to resume.
pub fn print_and_confirm(checkpoint: &Checkpoint) -> bool {
    let ago = SystemTime::now()
        .duration_since(checkpoint.timestamp)
        .unwrap_or_default();

    eprintln!(
        "─────────────────────────────────────────────────────────────"
    );
    eprintln!(
        "[检查点] 发现之前的搜索进度："
    );
    eprintln!(
        "  目标前缀:    {}",
        checkpoint.params.prefix
    );
    eprintln!(
        "  已尝试:      {} 次",
        checkpoint.attempts
    );
    eprintln!(
        "  已耗时:      {:.0} 秒 ({})",
        checkpoint.elapsed.as_secs_f64(),
        format_duration(checkpoint.elapsed)
    );
    eprintln!(
        "  距离上次:    {}",
        format_duration(ago)
    );
    eprintln!(
        "─────────────────────────────────────────────────────────────"
    );
    // Always resume — the checkpoint is informational only (search is
    // stateless, so we just continue with fresh random keys).
    true
}

fn format_duration(d: Duration) -> String {
    let secs = d.as_secs();
    if secs < 60 {
        format!("{secs}s")
    } else if secs < 3600 {
        format!("{}m {}s", secs / 60, secs % 60)
    } else {
        format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
    }
}
