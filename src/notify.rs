//! Bark push notification for iOS.
//!
//! Uses the system `curl` command (macOS / Linux have it) so we don't
//! need to bundle any TLS library.
//!
//! The user must provide their own Bark API key via `--bark` flag or
//! the `VANITY_BARK_KEY` environment variable.

use std::process::Command;

/// Send a Bark push notification via `curl`.
///
/// Bark API: `GET https://api.day.app/{key}/{title}/{body}`
pub fn send_bark(key: &str, title: &str, body: &str) -> Result<(), String> {
    let url = format!(
        "https://api.day.app/{}/{}/{}",
        key.trim(),
        urlencode(title),
        urlencode(body),
    );

    let output = Command::new("curl")
        .args(["-sS", "-o", "/dev/null", "-w", "%{http_code}", &url])
        .output()
        .map_err(|e| format!("cannot run curl: {e}"))?;

    let status = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if status.starts_with("2") || status == "0" {
        eprintln!("[Bark] notification sent ✓");
        Ok(())
    } else {
        let msg = format!("Bark failed (HTTP {status})");
        eprintln!("[Bark] {msg}");
        Err(msg)
    }
}

/// Resolve the Bark API key from CLI flag or config file.
pub fn resolve_key(cli_key: Option<&str>, cfg_key: Option<&str>) -> Option<String> {
    cli_key
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .or_else(|| cfg_key.map(|s| s.to_string()))
}

/// Minimal percent-encoding.
fn urlencode(s: &str) -> String {
    let mut encoded = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(b as char);
            }
            b' ' => encoded.push_str("%20"),
            _ => encoded.push_str(&format!("%{:02X}", b)),
        }
    }
    encoded
}
