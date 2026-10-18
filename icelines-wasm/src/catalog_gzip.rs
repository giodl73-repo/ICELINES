use std::io::Read;

use crate::{EngineError, MAX_PACKAGE_BYTES};

pub(crate) fn expand(bytes: &[u8], maximum: usize) -> Result<Vec<u8>, EngineError> {
    if maximum == 0 || maximum > MAX_PACKAGE_BYTES || bytes.len() > MAX_PACKAGE_BYTES {
        return Err(EngineError::InvalidPackage(
            "Invalid gzip byte limit".into(),
        ));
    }
    let invalid =
        |error: std::io::Error| EngineError::InvalidPackage(format!("Invalid gzip: {error}"));
    // Read through CRC/trailer and all members. The final one-byte probe detects
    // overflow without allocating/retaining an unbounded decoded stream.
    let mut decoder = flate2::bufread::MultiGzDecoder::new(bytes);
    let mut output = vec![0; maximum];
    let mut length = 0;
    while length < maximum {
        let read = decoder.read(&mut output[length..]).map_err(invalid)?;
        if read == 0 {
            output.truncate(length);
            return Ok(output);
        }
        length += read;
    }
    let mut extra = [0];
    if decoder.read(&mut extra).map_err(invalid)? != 0 {
        return Err(EngineError::InvalidPackage(
            "Expanded package exceeds its advertised byte limit".into(),
        ));
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn gzip(bytes: &[u8]) -> Vec<u8> {
        let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(bytes).unwrap();
        encoder.finish().unwrap()
    }

    #[test]
    fn exact_unicode_offset_and_larger_limits_preserve_bytes() {
        let raw = "Slafkovský: hockey".as_bytes();
        let compressed = gzip(raw);
        let mut padded = vec![255; 7];
        padded.extend_from_slice(&compressed);
        padded.extend_from_slice(&[255; 9]);
        let wire = &padded[7..7 + compressed.len()];
        assert_eq!(expand(wire, raw.len()).unwrap(), raw);
        assert_eq!(expand(wire, raw.len() + 9).unwrap(), raw);
        assert!(expand(wire, raw.len() - 1).is_err());
    }

    #[test]
    fn validates_crc_truncation_members_and_trailing_bytes() {
        let compressed = gzip(b"abc");
        let mut wrong_crc = compressed.clone();
        wrong_crc[compressed.len() - 8] ^= 1;
        assert!(expand(&wrong_crc, 3).is_err());
        assert!(expand(&compressed[..compressed.len() - 1], 3).is_err());
        let mut joined = compressed.clone();
        joined.extend(gzip(b"def"));
        assert_eq!(expand(&joined, 6).unwrap(), b"abcdef");
        assert!(expand(&joined, 3).is_err());
        let mut trailing = compressed;
        trailing.push(42);
        assert!(expand(&trailing, 3).is_err());
        assert!(expand(b"not gzip", 100).is_err());
    }

    #[test]
    fn zero_upper_limit_and_expansion_bomb_are_refused() {
        assert!(expand(&gzip(b"abc"), 0).is_err());
        assert!(expand(&gzip(b"abc"), MAX_PACKAGE_BYTES + 1).is_err());
        assert!(expand(&gzip(&vec![0; 1024 * 1024]), 128).is_err());
        assert_eq!(expand(&gzip(b""), 1).unwrap(), b"");
    }
}
