//! `benchmark` subcommand: measure key-derivation throughput.

use std::time::Instant;

use bitcoin::secp256k1::{Secp256k1, SecretKey};
use bitcoin::Network;
use rand::rngs::OsRng;
use rand::RngCore;

use crate::address::derive_all;
use crate::error::Error;

/// Number of keys to derive per thread during the benchmark.
const ITER_PER_THREAD: usize = 50_000;

/// Run the benchmark and print results to stderr.
pub fn run() -> Result<(), Error> {
    let num_threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);
    let total_iters = num_threads * ITER_PER_THREAD;

    crate::style::header("Benchmark");
    crate::style::kv("threads", &num_threads.to_string());
    crate::style::kv(
        "iterations",
        &format!("{} ({} per thread)", total_iters, ITER_PER_THREAD),
    );
    eprintln!();

    // Generate random secret keys upfront so we don't measure RNG time.
    let mut all_keys: Vec<[u8; 32]> = Vec::with_capacity(total_iters);
    for _ in 0..total_iters {
        let mut b = [0u8; 32];
        OsRng.fill_bytes(&mut b);
        all_keys.push(b);
    }

    let start = Instant::now();

    // Parallel derivation using std::thread.
    let mut handles = Vec::with_capacity(num_threads);
    for chunk in all_keys.chunks(ITER_PER_THREAD) {
        let chunk = chunk.to_vec();
        handles.push(std::thread::spawn(move || {
            let secp = Secp256k1::new();
            let mut count = 0usize;
            for bytes in &chunk {
                if let Ok(sk) = SecretKey::from_slice(bytes) {
                    if derive_all(&secp, &sk, true, Network::Bitcoin).is_ok() {
                        count += 1;
                    }
                }
            }
            count
        }));
    }

    let total_success: usize = handles.into_iter().map(|h| h.join().unwrap_or(0)).sum();

    let elapsed = start.elapsed().as_secs_f64();
    let rate = total_success as f64 / elapsed;

    crate::style::header("Results");
    crate::style::kv("elapsed", &format!("{elapsed:.3}s"));
    crate::style::kv("keys derived", &total_success.to_string());
    crate::style::kv("speed", &format!("{:.2} Mkeys/s", rate / 1_000_000.0));
    crate::style::kv("threads", &num_threads.to_string());
    crate::style::kv(
        "per thread",
        &format!("{:.2} kkeys/s", (rate / num_threads as f64) / 1_000.0),
    );
    eprintln!();

    Ok(())
}
