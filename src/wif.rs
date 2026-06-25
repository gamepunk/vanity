//! WIF (Wallet Import Format) parsing and inspection.
//!
//! Uses `bitcoin::PrivateKey` under the hood, which handles all the
//! encoding / decoding, checksum verification, and network detection.

use bitcoin::Network;

use crate::error::Error;

/// Parsed information from a WIF string.
#[derive(Debug, Clone)]
pub struct WifInfo {
    pub private_key: bitcoin::PrivateKey,
    pub network: Network,
    pub compressed: bool,
}

/// Parse a WIF string and return structured information.
pub fn parse_wif(wif: &str) -> Result<WifInfo, Error> {
    let pk: bitcoin::PrivateKey = wif.parse().map_err(|e| {
        Error::InvalidWif(format!("failed to parse WIF: {e}"))
    })?;

    // PrivateKey stores network as NetworkKind internally; convert to Network.
    // WIF version byte 0x80 → Main (Bitcoin), 0xEF → Test (usually Testnet).
    let network = match pk.network {
        bitcoin::NetworkKind::Main => Network::Bitcoin,
        bitcoin::NetworkKind::Test => Network::Testnet,
    };

    Ok(WifInfo {
        network,
        compressed: pk.compressed,
        private_key: pk,
    })
}

/// Format a private key as WIF.
pub fn format_wif(
    secret_key: &bitcoin::secp256k1::SecretKey,
    compressed: bool,
    network: Network,
) -> String {
    let pk = if compressed {
        bitcoin::PrivateKey::new(*secret_key, network)
    } else {
        bitcoin::PrivateKey::new_uncompressed(*secret_key, network)
    };
    pk.to_wif()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roundtrip_wif() {
        use bitcoin::secp256k1::SecretKey;
        let mut bytes = [0u8; 32];
        bytes[31] = 1;
        let sk = SecretKey::from_slice(&bytes).unwrap();

        let wif = format_wif(&sk, true, Network::Bitcoin);
        let info = parse_wif(&wif).unwrap();
        assert_eq!(info.network, Network::Bitcoin);
        assert!(info.compressed);
        assert_eq!(info.private_key.inner, sk);
    }

    #[test]
    fn test_known_wif() {
        // Compressed WIF for private key = 1
        let wif = "Kz6K83ge1AeeDi7fvE7kxGkyYws47sucXUZZwMXVTFG9q7u4ey12";
        let info = parse_wif(wif).unwrap();
        assert_eq!(info.network, Network::Bitcoin);
        assert!(info.compressed);
        let expected_hex = "55b4e1da4b24e4606f9c395116f4d3620c6cec094faf15e99774e7eecd327283";
        assert_eq!(
            info.private_key.inner.secret_bytes()[..],
            hex_to_bytes(expected_hex)[..]
        );
    }

    #[test]
    fn test_uncompressed_wif() {
        // Uncompressed WIF for private key = 1
        let wif = "5HpHagT65TZzG1PH3CSu63k8DbpvD8s5ip4nEB3kEsreAnchuDf";
        let info = parse_wif(wif).unwrap();
        assert_eq!(info.network, Network::Bitcoin);
        assert!(!info.compressed); // uncompressed
    }

    #[test]
    fn test_invalid_wif() {
        // Too short, wrong checksum, etc.
        assert!(parse_wif("").is_err());
        assert!(parse_wif("abc").is_err());
        // Wrong network byte (0xEF = testnet, but must match)
        // Just check that garbage doesn't parse
        assert!(parse_wif("11111111111111111111111111111111111111111111").is_err());
    }

    #[test]
    fn test_format_then_parse_roundtrip() {
        use bitcoin::secp256k1::SecretKey;
        let mut bytes = [0u8; 32];
        bytes[31] = 42;
        let sk = SecretKey::from_slice(&bytes).unwrap();

        // Compressed
        let wif_c = format_wif(&sk, true, Network::Bitcoin);
        let info_c = parse_wif(&wif_c).unwrap();
        assert!(info_c.compressed);
        assert_eq!(info_c.private_key.inner, sk);

        // Uncompressed
        let wif_u = format_wif(&sk, false, Network::Bitcoin);
        let info_u = parse_wif(&wif_u).unwrap();
        assert!(!info_u.compressed);
        assert_eq!(info_u.private_key.inner, sk);
    }

    fn hex_to_bytes(s: &str) -> Vec<u8> {
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
            .collect()
    }
}
