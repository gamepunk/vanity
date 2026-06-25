//! Startup self-test using the well-known public test vector
//! (secret key = 1, the generator point G itself).
//!
//! If any step fails the program will refuse to continue, guaranteeing
//! that no output is produced when the derivation pipeline is broken.

use bitcoin::{
    Network,
    secp256k1::{Secp256k1, SecretKey},
};
use crate::address::derive_all;
use crate::error::Error;

/// Run the self-test.  Panics (or exits) on failure.
pub fn run() -> Result<(), Error> {
    let secp = Secp256k1::new();
    let mut sk_bytes = [0u8; 32];
    sk_bytes[31] = 1;
    let sk = SecretKey::from_slice(&sk_bytes)
        .expect("self-test: failed to construct secret key");

    let set_compressed = derive_all(&secp, &sk, true, Network::Bitcoin)?;
    let set_uncompressed = derive_all(&secp, &sk, false, Network::Bitcoin)?;

    // ── well-known public test vectors ──────────────────────────────
    const EXPECTED: &[(&str, &str, &str)] = &[
        // (compressed? , address-type  , expected address)
        ("compressed",   "Legacy",        "1BgGZ9tcN4rm9KBzDn7KprQz87SZ26SAMH"),
        ("uncompressed", "Legacy",        "1EHNa6Q4Jz2uvNExL497mE43ikXhwF6kZm"),
        ("compressed",   "P2SH-SegWit",   "3JvL6Ymt8MVWiCNHC7oWU6nLeHNJKLZGLN"),
        ("compressed",   "Native SegWit", "bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4"),
        ("compressed",   "Taproot",       "bc1pmfr3p9j00pfxjh0zmgp99y8zftmd3s5pmedqhyptwy6lm87hf5sspknck9"),
    ];

    let sets = [("compressed", &set_compressed), ("uncompressed", &set_uncompressed)];

    for &(compr_label, expected_label, expected_addr) in EXPECTED {
        let (_, set) = sets.iter().find(|(l, _)| *l == compr_label).unwrap();
        let addr = match expected_label {
            "Legacy"        => &set.legacy,
            "P2SH-SegWit"   => &set.p2sh_segwit,
            "Native SegWit" => &set.native_segwit,
            "Taproot"       => &set.taproot,
            _ => unreachable!(),
        };
        let got = addr.to_string();
        assert_eq!(
            got, expected_addr,
            "self-test FAILED for {compr_label} {expected_label}: got {got}, expected {expected_addr}",
        );
    }

    Ok(())
}
