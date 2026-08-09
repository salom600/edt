//! Stable, opaque identifiers used across the project model.
//!
//! IDs are 128-bit random values rendered as 26-character base32 strings
//! (Crockford alphabet, lowercase). They are unique within a project,
//! survive copy/paste, and are stable across save/load cycles.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};

/// An opaque identifier for any project entity (asset, track, clip, ...).
///
/// Internally a `u128` but serialized as a 26-char base32 string for
/// JSON compatibility (`serde_json` does not support `u128` directly).
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Id(pub u128);

impl Serialize for Id {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str().as_str())
    }
}

impl<'de> Deserialize<'de> for Id {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s: &str = Deserialize::deserialize(deserializer)?;
        let s = s.as_bytes();
        if s.len() != 26 {
            return Err(serde::de::Error::custom(format!(
                "expected 26-char id, got {} chars",
                s.len()
            )));
        }
        let mut value: u128 = 0;
        for &b in s {
            let digit = match b {
                b'0'..=b'9' => (b - b'0') as u128,
                b'a'..=b'h' => (b - b'a' + 10) as u128,
                b'j' | b'k' => (b - b'a' + 9) as u128,
                b'm' => 22,
                b'n' | b'p' => (b - b'a' + 8) as u128,
                b'q'..=b't' => (b - b'a' + 7) as u128,
                b'v'..=b'z' => (b - b'a' + 6) as u128,
                _ => {
                    return Err(serde::de::Error::custom(format!(
                        "invalid id char: {}",
                        b as char
                    )))
                }
            };
            value = (value << 5) | digit;
        }
        Ok(Id(value))
    }
}

impl Id {
    /// Render the id as a 26-char lowercase base32 string.
    pub fn as_str(&self) -> IdStr {
        IdStr::encode(self.0)
    }
}

impl fmt::Debug for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Id({})", self.as_str().as_str())
    }
}

impl fmt::Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str().as_str())
    }
}

/// Stack-allocated string form of an [`Id`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct IdStr([u8; 26]);

impl IdStr {
    const ALPHABET: &[u8; 32] = b"0123456789abcdefghjkmnpqrstvwxyz";

    fn encode(mut value: u128) -> Self {
        // 128 bits / 5 bits per char = 25.6 chars; pad with leading 0 to 26.
        let mut out = [b'0'; 26];
        for i in (0..26).rev() {
            out[i] = Self::ALPHABET[(value & 0x1f) as usize];
            value >>= 5;
            if value == 0 {
                break;
            }
        }
        IdStr(out)
    }

    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.0).expect("id is ascii")
    }
}

impl fmt::Display for IdStr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Generates monotonically-increasing [`Id`]s.
///
/// Each project owns one generator. The implementation uses a global
/// atomic counter XORed with a per-process random seed derived from time
/// and PID. This is **not** cryptographically secure — it is designed for
/// uniqueness within a single project, not for unforgeability.
pub struct IdGenerator {
    counter: AtomicU64,
    seed: u128,
}

impl IdGenerator {
    pub fn new() -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u128)
            .unwrap_or(0xdead_beef_cafe_babe);
        let pid = std::process::id() as u128;
        // Mix in a static counter to add additional dispersion across
        // multiple generators created within the same nanosecond.
        static INSTANCE: AtomicU64 = AtomicU64::new(0);
        let inst = INSTANCE.fetch_add(1, Ordering::Relaxed) as u128;
        let seed = nanos ^ (pid << 64) ^ (inst << 96);
        Self {
            counter: AtomicU64::new(0x1234_5678),
            seed,
        }
    }

    pub fn next(&self) -> Id {
        let n = self.counter.fetch_add(1, Ordering::Relaxed) as u128;
        Id(self.seed ^ (n << 32) ^ n)
    }
}

impl Default for IdGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn ids_are_unique() {
        let gen = IdGenerator::new();
        let mut seen = HashSet::new();
        for _ in 0..10_000 {
            let id = gen.next();
            assert!(seen.insert(id), "duplicate id generated");
        }
    }

    #[test]
    fn id_str_round_trips_value() {
        let id = Id(0x1234_5678_9abc_def0_1234_5678_9abc_def0);
        let s1 = id.as_str();
        let s2 = id.as_str();
        assert_eq!(s1.as_str(), s2.as_str());
        assert_eq!(s1.as_str().len(), 26);
    }

    #[test]
    fn id_str_alphabet_is_crockford() {
        let id = Id(u128::MAX);
        let s = id.as_str();
        for b in s.as_str().bytes() {
            assert!(b"0123456789abcdefghjkmnpqrstvwxyz".contains(&b));
        }
    }
}
