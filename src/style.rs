//! Output styling helpers — clean, Rails-inspired formatting.
//!
//! Uses ANSI escape codes for color when output is a terminal.
//! No external dependencies.

use std::io::IsTerminal;

// ── ANSI codes ───────────────────────────────────────────────────────
const RESET: &str = "\x1b[0m";
const GREEN: &str = "\x1b[32m";
const CYAN: &str = "\x1b[36m";
const YELLOW: &str = "\x1b[33m";
const RED: &str = "\x1b[31m";
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";

/// Whether stderr is a terminal (we color stderr output).
fn is_tty() -> bool {
    std::io::stderr().is_terminal()
}

/// Helper: apply ANSI only when stderr is a TTY.
macro_rules! ansi {
    ($code:expr) => {{
        if is_tty() {
            $code
        } else {
            ""
        }
    }};
}

/// Green checkmark prefix for success messages.
pub fn success(msg: &str) {
    let check = ansi!(GREEN);
    let reset = ansi!(RESET);
    eprintln!("{check}>>{reset} {msg}");
}

/// Section header (like Rails `==` but with >>).
pub fn header(msg: &str) {
    let bold = ansi!(BOLD);
    let reset = ansi!(RESET);
    eprintln!("\n{bold}>> {msg}{reset}");
}

/// Info label: value pair, indented nicely.
pub fn kv(label: &str, value: &str) {
    let dim = ansi!(DIM);
    let reset = ansi!(RESET);
    eprintln!("{dim}  {label}:{reset} {value}");
}

/// Warning message with yellow.
pub fn warning(msg: &str) {
    let yellow = ansi!(YELLOW);
    let reset = ansi!(RESET);
    eprintln!("{yellow}⚠ {msg}{reset}");
}

/// Error message with red.
pub fn error(msg: &str) {
    let red = ansi!(RED);
    let reset = ansi!(RESET);
    eprintln!("{red}✖ {msg}{reset}");
}

/// Progress line (uses carriage return, shown on stderr).
pub fn progress_line(attempts: u64, rate: f64, elapsed: f64) {
    if is_tty() {
        let dim = ansi!(DIM);
        let reset = ansi!(RESET);
        eprint!(
            "\r{dim}  {attempts:>12} attempts  |  {rate:>8.2} Mkeys/s  |  {elapsed:>7.1}s{reset}  ",
        );
    } else {
        // Plain newline for non-TTY (piped output, logs).
        eprintln!("  {attempts:>12} attempts  |  {rate:>8.2} Mkeys/s  |  {elapsed:>7.1}s",);
    }
}

/// Print the final result line for "Found".
pub fn result_line(key: &str, value: &str) {
    let cyan = ansi!(CYAN);
    let reset = ansi!(RESET);
    println!("{cyan}  {key}:{reset} {value}");
}
