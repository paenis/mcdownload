use std::str::FromStr;
use std::sync::LazyLock;

use thiserror::Error;

type IdentifierValue = u64;

const ALPHABET: &str = "0123456789abcdefghjkmnpqrstvwxyz"; // crockford's base32 (lowercase)
const IDENTIFIER_LENGTH: usize = IdentifierValue::BITS.div_ceil(ALPHABET.len().ilog2()) as usize;

static ENCODER: LazyLock<data_encoding::Encoding> = LazyLock::new(|| {
    let mut spec = data_encoding::Specification::new();
    spec.symbols.push_str(ALPHABET);
    spec.padding = None;
    spec.translate.from.push_str("OoIiLlABCDEFGHJKMNPQRSTVWXYZ");
    spec.translate.to.push_str("001111abcdefghjkmnpqrstvwxyz");
    spec.encoding().unwrap()
});

/// Randomly generated identifier for a server instance
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GeneratedIdentifier {
    value: IdentifierValue,
}

impl GeneratedIdentifier {
    /// Create a new random identifier
    pub fn new() -> Self {
        let mut buf = [0u8; IdentifierValue::BITS as usize / 8];
        fastrand::fill(&mut buf);
        let value = IdentifierValue::from_le_bytes(buf);
        Self { value }
    }
    /// Get the string representation of this identifier
    pub fn as_str(&self) -> String {
        ENCODER.encode(&self.value.to_le_bytes())
    }
}

#[derive(Error, Debug)]
pub enum IdentifierParseError {
    #[error("invalid length. expected {IDENTIFIER_LENGTH}, found {0}")]
    InvalidLength(usize),
    #[error("invalid character: {0}. expected one of: {ALPHABET}")]
    InvalidCharacter(char),
    #[error("unexpected error")]
    Other(#[from] data_encoding::DecodeError),
}

impl FromStr for GeneratedIdentifier {
    type Err = IdentifierParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() != IDENTIFIER_LENGTH {
            return Err(IdentifierParseError::InvalidLength(s.len()));
        }

        let bytes = ENCODER.decode(s.as_bytes()).map_err(|e| match e.kind {
            data_encoding::DecodeKind::Symbol => {
                IdentifierParseError::InvalidCharacter(s.as_bytes()[e.position] as char)
            }
            _ => IdentifierParseError::Other(e),
        })?;

        let mut arr = [0u8; IdentifierValue::BITS as usize / 8];
        arr.copy_from_slice(&bytes);
        Ok(Self {
            value: IdentifierValue::from_le_bytes(arr),
        })
    }
}

impl std::fmt::Display for GeneratedIdentifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    proptest! {
        #[test]
        fn id_roundtrip(v in any::<IdentifierValue>()) {
            let id = GeneratedIdentifier { value: v };
            let s = id.as_str();
            let parsed = s.parse::<GeneratedIdentifier>().unwrap();
            prop_assert_eq!(id.value, parsed.value);
        }
    }

    #[test]
    fn id_parse_invalid_length() {
        let err = "abc".parse::<GeneratedIdentifier>().unwrap_err();
        match err {
            IdentifierParseError::InvalidLength(3) => {}
            _ => panic!("unexpected error: {err}"),
        }
    }

    #[test]
    fn id_parse_invalid_character() {
        // length is checked first
        let err = "abcdefghij".parse::<GeneratedIdentifier>().unwrap_err();
        match err {
            IdentifierParseError::InvalidLength(10) => {}
            _ => panic!("unexpected error: {err}"),
        }

        // make a valid-length string with an invalid character
        let s = ALPHABET
            .chars()
            .cycle()
            .take(IDENTIFIER_LENGTH - 1)
            .collect::<String>()
            + "!";

        let err = s.parse::<GeneratedIdentifier>().unwrap_err();
        match err {
            IdentifierParseError::InvalidCharacter('!') => {}
            _ => panic!("unexpected error: {err}"),
        }
    }
}
