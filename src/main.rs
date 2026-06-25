//! Bitcoin Vanity Address Generator — professional CPU multi-threaded edition.
//!
//! All cryptographic primitives are delegated to the audited
//! [`rust-bitcoin`](https://github.com/rust-bitcoin/rust-bitcoin) crate and its
//! underlying [`libsecp256k1`](https://github.com/bitcoin-core/secp256k1) C library.
//!
//! ## Usage
//!
//! ```text
//! vanity <prefix>                 # search (legacy syntax)
//! vanity search <prefix> [options]     # search (explicit)
//! vanity verify <WIF>                  # verify a private key
//! vanity address <WIF>                 # derive all 4 address types
//! vanity benchmark                     # throughput benchmark
//! ```

pub mod address;
pub mod benchmark;
pub mod checkpoint;
pub mod cli;
pub mod config;
pub mod error;
pub mod log;
pub mod mnemonic;
pub mod notify;
pub mod search;
pub mod self_test;
pub mod style;
pub mod verify;
pub mod wif;

use std::process;

use clap::Parser;
use bitcoin::Network;

use cli::{Cli, CliCommand, AddressType, validate_prefix};
use error::Error;

fn main() {
    // ── Load config ────────────────────────────────────────────────
    let cfg = config::Config::load();

    // ── Backward-compatible argument rewriting ─────────────────────
    let raw: Vec<String> = std::env::args().collect();
    let args: Vec<String> = if raw.len() >= 2 && !is_subcommand(&raw[1]) && !raw[1].starts_with('-') {
        let mut v = vec![raw[0].clone(), "search".to_string()];
        v.extend(raw[1..].iter().cloned());
        v
    } else {
        raw
    };

    let cli = Cli::parse_from(&args);

    let result = match &cli.command {
        CliCommand::Search { prefix, address_type, case_insensitive, mnemonic, uncompressed, threads, quiet, bark } => {
            run_search(&cfg, prefix, *address_type, *case_insensitive, *uncompressed, *mnemonic, *threads, *quiet, bark.as_deref())
        }
        CliCommand::Verify { wif } => verify::run(wif),
        CliCommand::Address { wif } => run_address(wif),
        CliCommand::Benchmark => benchmark::run(),
        CliCommand::Mnemonic => run_mnemonic(),
    };

    if let Err(e) = result {
        eprintln!("错误: {e}");
        process::exit(1);
    }
}

// ── Subcommand implementations ──────────────────────────────────────────

/// Search for a vanity address.
fn run_search(
    cfg: &config::Config,
    prefix: &str,
    addr_type: AddressType,
    case_insensitive: bool,
    uncompressed: bool,
    use_bip32: bool,
    threads: usize,
    quiet: bool,
    bark_key: Option<&str>,
) -> Result<(), Error> {
    // Validate the prefix for the chosen address type.
    if let Err(msg) = validate_prefix(prefix, addr_type) {
        return Err(Error::InvalidPrefix(msg));
    }

    let network = Network::Bitcoin;
    let compressed = !uncompressed;

    // ── Self-test ───────────────────────────────────────────────────
    self_test::run()?;
    if !quiet {
        style::success("Self-test passed");
    }

    // ── Checkpoint ──────────────────────────────────────────────────
    if let Some(ref cp) = checkpoint::load() {
        checkpoint::print_and_confirm(cp);
    }
    log::info(&format!(
        "开始搜索: prefix={prefix}, type={}, case_insensitive={case_insensitive}, threads={threads}",
        addr_type.label(),
    ));

    // ── Search info ─────────────────────────────────────────────────
    let _prefix_body = &prefix[addr_type.prefix_hint().len()..];

    if !quiet {
        style::header("Searching");
        style::kv("prefix", prefix);
        style::kv("type", addr_type.label());
        style::kv("threads", &threads.to_string());
        if use_bip32 {
            style::kv("source", "BIP39+BIP32");
        }
        eprintln!();
    }

    // ── Search ──────────────────────────────────────────────────────
    let (found, elapsed) = search::search(
        prefix, addr_type, case_insensitive, compressed, network, threads, use_bip32, quiet,
    )?;

    // ── Clear checkpoint + write log ───────────────────────────────
    checkpoint::clear();
    log::info(&format!(
        "找到! prefix={}, address={}, attempts={}, elapsed={:.2}s",
        prefix, found.address, found.total_attempts, elapsed.as_secs_f64(),
    ));

    let info = wif::parse_wif(&found.wif)?;
    let secp = bitcoin::secp256k1::Secp256k1::new();

    // ── Send notification (if Bark key provided) ──────────────────
    if let Some(bk) = notify::resolve_key(bark_key, cfg.bark_key.as_deref()) {
        let _ = notify::send_bark(
            &bk,
            "🎯 Vanity address found!",
            &format!("Address: {}\nElapsed: {:.1}s", found.address, elapsed.as_secs_f64()),
        );
    }

    if !quiet {
        style::header("Found vanity address");
    }

    style::result_line("Address", &found.address);
    style::result_line("WIF", &found.wif);

    if let Some(ref phrase) = found.mnemonic_phrase {
        println!();
        style::header("BIP39 Mnemonic");
        println!("  {}", phrase);
    }
    if let Some(ref path) = found.derivation_path {
        style::kv("derivation path", path);
    }

    println!();
    style::kv("attempts", &found.total_attempts.to_string());
    style::kv("elapsed", &format!("{:.2}s", elapsed.as_secs_f64()));
    println!();

    // ── Wallet addresses ───────────────────────────────────────────
    if let Some(ref phrase) = found.mnemonic_phrase {
        let wallet_addrs = derive_wallet_addresses(phrase, 0, network)?;
        style::header("Wallet addresses (index 0)");
        println!("{}", wallet_addrs);
        println!();
        style::warning("Import the mnemonic into any BIP39 wallet. The above addresses will match exactly.");
    } else {
        let all_addrs = address::derive_all(
            &secp, &info.private_key.inner, info.compressed, network,
        )?;
        style::header("Same-key addresses");
        style::result_line("P2PKH",  &all_addrs.legacy.to_string());
        style::result_line("P2SH",   &all_addrs.p2sh_segwit.to_string());
        style::result_line("P2WPKH", &all_addrs.native_segwit.to_string());
        style::result_line("P2TR",   &all_addrs.taproot.to_string());
    }

    println!();
    style::warning("Move funds immediately. Clear terminal history.");

    Ok(())
}

/// Derive wallet-compatible addresses for all 4 standard BIP32 paths
/// (BIP44 / BIP49 / BIP84 / BIP86) at a given address index.
fn derive_wallet_addresses(phrase: &str, index: u32, network: Network) -> Result<String, Error> {
    use bitcoin::bip32::{DerivationPath, Xpriv};
    use bip39::Mnemonic;

    let mnemonic = Mnemonic::parse(phrase)
        .map_err(|e| Error::InvalidWif(format!("mnemonic parse: {e}")))?;
    let seed = mnemonic.to_seed("");
    let secp = bitcoin::secp256k1::Secp256k1::new();
    let master = Xpriv::new_master(network, &seed)?;

    // BIP44 / BIP49 / BIP84 / BIP86 at the given index.
    let configs = [
        (44, "传统 Legacy (P2PKH)"),
        (49, "嵌套 SegWit (P2SH-P2WPKH)"),
        (84, "原生 SegWit (P2WPKH)"),
        (86, "Taproot (P2TR)"),
    ];

    let mut lines = Vec::new();
    for &(purpose, label) in &configs {
        let path_str = format!("m/{}'/0'/0'/0/{index}", purpose);
        let path: DerivationPath = path_str.parse()?;
        let child = master.derive_priv(&secp, &path)?;
        let addr = address::derive_single(&secp, &child.private_key, true, network,
            if purpose == 44 { cli::AddressType::Legacy }
            else if purpose == 49 { cli::AddressType::P2sh }
            else if purpose == 84 { cli::AddressType::Segwit }
            else { cli::AddressType::Taproot }
        )?;
        lines.push(format!("  {label:<24} {}  (path: {path_str})", addr));
    }

    Ok(lines.join("\n"))
}

/// Derive and display all four address types from a WIF.
fn run_address(wif_str: &str) -> Result<(), Error> {
    let info = wif::parse_wif(wif_str)?;
    let secp = bitcoin::secp256k1::Secp256k1::new();
    let set = address::derive_all(&secp, &info.private_key.inner, info.compressed, info.network)?;

    let net = match info.network {
        Network::Bitcoin => "Mainnet",
        Network::Testnet => "Testnet",
        Network::Signet => "Signet",
        Network::Regtest => "Regtest",
        _ => "Unknown",
    };

    style::header("Addresses from private key");
    style::kv("network", net);
    style::kv("compressed", &info.compressed.to_string());
    println!();
    style::header("Derived addresses");
    style::result_line("P2PKH",  &set.legacy.to_string());
    style::result_line("P2SH",   &set.p2sh_segwit.to_string());
    style::result_line("P2WPKH", &set.native_segwit.to_string());
    style::result_line("P2TR",   &set.taproot.to_string());
    println!();

    Ok(())
}

/// Generate a random BIP39 mnemonic and display all derived addresses.
fn run_mnemonic() -> Result<(), Error> {
    let result = mnemonic::generate_random()?;

    style::header("BIP39 Mnemonic (24 words, 256-bit)");
    println!("  {}", result.phrase);
    println!();

    for p in &result.paths {
        style::header(&format!("{}", p.label));
        style::kv("path", &p.path);
        style::kv("WIF", &p.wif);
        style::result_line("P2PKH",  &p.legacy);
        style::result_line("P2SH",   &p.p2sh);
        style::result_line("P2WPKH", &p.segwit);
        style::result_line("P2TR",   &p.taproot);
        println!();
    }

    style::warning("Write down these 24 words. Keep them offline. Anyone with this phrase can steal your funds.");
    style::warning("Test with a small amount before depositing significant funds.");

    Ok(())
}

/// Check whether a string is a known subcommand name.
fn is_subcommand(s: &str) -> bool {
    matches!(
        s,
        "search" | "s"
        | "verify" | "v"
        | "address" | "a" | "addr"
        | "benchmark" | "b" | "bench"
        | "mnemonic" | "m"
    )
}
