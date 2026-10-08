//! Resource preflight for untrusted interchange bytes. Does not normalize hockey data.
use serde::de::{DeserializeSeed, Error, MapAccess, SeqAccess, Visitor};
use std::fmt;

const MAX_STRING_BYTES: usize = 1024;
const MAX_ENCODED_STRING_BYTES: usize = MAX_STRING_BYTES * 6;
const MAX_TEXT_BYTES: usize = 8 * 1024 * 1024;
const MAX_ARRAY_ITEMS: usize = 10_000;
const MAX_NODES: usize = 500_000;

pub(crate) fn validate(bytes: &[u8]) -> Result<(), String> {
    // serde_json decodes escaped strings into scratch storage. Bound the encoded
    // token first so that a rejected string cannot allocate arbitrarily large scratch.
    let (mut quoted, mut escaped, mut length) = (false, false, 0);
    for &byte in bytes {
        if !quoted {
            if byte == b'"' {
                quoted = true;
                length = 0;
            }
        } else if !escaped && byte == b'"' {
            quoted = false;
        } else {
            length += 1;
            if length > MAX_ENCODED_STRING_BYTES {
                return Err("package string exceeds 6144 encoded bytes".into());
            }
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            }
        }
    }
    let mut budget = Budget { text: 0, nodes: 0 };
    let mut decoder = serde_json::Deserializer::from_slice(bytes);
    Seed(&mut budget)
        .deserialize(&mut decoder)
        .map_err(|e| e.to_string())?;
    decoder.end().map_err(|e| e.to_string())
}

struct Budget {
    text: usize,
    nodes: usize,
}
struct Seed<'a>(&'a mut Budget);
struct Check<'a>(&'a mut Budget);
impl<'de> DeserializeSeed<'de> for Seed<'_> {
    type Value = ();
    fn deserialize<D: serde::Deserializer<'de>>(self, decoder: D) -> Result<(), D::Error> {
        self.0.nodes += 1;
        if self.0.nodes > MAX_NODES {
            return Err(D::Error::custom("package exceeds 500000 JSON nodes"));
        }
        decoder.deserialize_any(Check(self.0))
    }
}
impl<'de> Visitor<'de> for Check<'_> {
    type Value = ();
    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("resource-bounded JSON")
    }
    fn visit_str<E: Error>(self, value: &str) -> Result<(), E> {
        if value.len() > MAX_STRING_BYTES {
            return Err(E::custom("package string exceeds 1024 UTF-8 bytes"));
        }
        self.0.text += value.len();
        if self.0.text > MAX_TEXT_BYTES {
            return Err(E::custom("package aggregate text exceeds 8 MiB"));
        }
        Ok(())
    }
    fn visit_bool<E: Error>(self, _: bool) -> Result<(), E> {
        Ok(())
    }
    fn visit_i64<E: Error>(self, _: i64) -> Result<(), E> {
        Ok(())
    }
    fn visit_u64<E: Error>(self, _: u64) -> Result<(), E> {
        Ok(())
    }
    fn visit_f64<E: Error>(self, _: f64) -> Result<(), E> {
        Ok(())
    }
    fn visit_unit<E: Error>(self) -> Result<(), E> {
        Ok(())
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<(), A::Error> {
        let mut items = 0;
        while sequence.next_element_seed(Seed(self.0))?.is_some() {
            items += 1;
            if items > MAX_ARRAY_ITEMS {
                return Err(A::Error::custom("package array exceeds 10000 items"));
            }
        }
        Ok(())
    }
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<(), A::Error> {
        while map.next_key_seed(Seed(self.0))?.is_some() {
            map.next_value_seed(Seed(self.0))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn text_counts_decoded_utf8_and_accepts_escaped_quotes() {
        assert!(validate(br#"{"name":"Slafkovsk\u00fd", "text":"a\\\"b"}"#).is_ok());
        let boundary = serde_json::to_vec(&"x".repeat(MAX_STRING_BYTES)).unwrap();
        assert!(validate(&boundary).is_ok());
        let over = serde_json::to_vec(&"é".repeat(MAX_STRING_BYTES / 2 + 1)).unwrap();
        assert!(validate(&over).unwrap_err().contains("1024 UTF-8"));
        let encoded = format!("\"{}\"", "\\u0078".repeat(MAX_STRING_BYTES));
        assert!(validate(encoded.as_bytes()).is_ok());
        let encoded_over = format!("\"{}\"", "\\u0078".repeat(MAX_STRING_BYTES + 1));
        assert!(validate(encoded_over.as_bytes())
            .unwrap_err()
            .contains("6144 encoded"));
    }
    #[test]
    fn arrays_and_aggregate_text_are_bounded() {
        let rows = serde_json::to_vec(&vec![0; MAX_ARRAY_ITEMS]).unwrap();
        assert!(validate(&rows).is_ok());
        let rows = serde_json::to_vec(&vec![0; MAX_ARRAY_ITEMS + 1]).unwrap();
        assert!(validate(&rows).unwrap_err().contains("10000 items"));
        let text = serde_json::to_vec(&vec![
            "x".repeat(MAX_STRING_BYTES);
            MAX_TEXT_BYTES / MAX_STRING_BYTES + 1
        ])
        .unwrap();
        assert!(validate(&text).unwrap_err().contains("aggregate text"));
    }
    #[test]
    fn nested_small_arrays_cannot_bypass_total_node_budget() {
        let nested = serde_json::to_vec(&vec![vec![0; 5000]; 100]).unwrap();
        assert!(validate(&nested).unwrap_err().contains("500000 JSON nodes"));
    }
    #[test]
    fn malformed_and_trailing_json_remain_errors() {
        for input in [b"{} {}".as_slice(), b"{\"a\":\"unterminated", b"[1,]"] {
            assert!(validate(input).is_err());
        }
    }
}
