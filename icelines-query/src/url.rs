use icelines_core::stats_catalog::{parse_filter_expr, FilterExpr, FilterParseError};

/// Pull every `filter=...` occurrence out of a raw URL query string,
/// preserving order. Empty values are dropped.
///
/// Repeated filters are a platform query contract, not a web-handler detail:
/// repeated keys compose as top-level AND constraints.
pub fn parse_filters_from_query(qs: &str) -> Vec<String> {
    qs.split('&')
        .filter_map(|pair| {
            let (key, value) = pair.split_once('=')?;
            if key != "filter" {
                return None;
            }
            let decoded = url_decode_form_value(value);
            if decoded.is_empty() {
                None
            } else {
                Some(decoded)
            }
        })
        .collect()
}

/// Combine multiple filter strings into one catalog `FilterExpr`.
///
/// Each string is parsed independently, then repeated filters are joined with
/// top-level AND. This mirrors repeated CLI `--filter` semantics.
pub fn combine_filter_exprs(raw: &[String]) -> Result<Option<FilterExpr>, FilterParseError> {
    let mut combined: Option<FilterExpr> = None;
    for raw_str in raw {
        let parsed = parse_filter_expr(raw_str)?;
        combined = Some(match combined {
            None => parsed,
            Some(existing) => FilterExpr::And(Box::new(existing), Box::new(parsed)),
        });
    }
    Ok(combined)
}

fn url_decode_form_value(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b'%' if i + 2 < bytes.len() => {
                let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or("");
                match u8::from_str_radix(hex, 16) {
                    Ok(byte) => {
                        out.push(byte);
                        i += 3;
                    }
                    Err(_) => {
                        out.push(bytes[i]);
                        i += 1;
                    }
                }
            }
            other => {
                out.push(other);
                i += 1;
            }
        }
    }
    String::from_utf8(out).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use icelines_core::stats_catalog::{FilterOp, StatId};

    #[test]
    fn repeated_filters_preserve_order_and_decode_form_values() {
        let got = parse_filters_from_query(
            "sort=points&filter=g%3E%3D30&filter=country%3DCAN+OR+country%3DUSA&filter=",
        );
        assert_eq!(got, vec!["g>=30", "country=CAN OR country=USA"]);
    }

    #[test]
    fn repeated_filters_compose_as_and() {
        let raw = vec!["g>=30".to_string(), "a>=40".to_string()];
        let combined = combine_filter_exprs(&raw)
            .expect("filters should parse")
            .expect("combined filter");

        match combined {
            FilterExpr::And(left, right) => {
                let left = left.as_atom().expect("left atom");
                let right = right.as_atom().expect("right atom");
                assert_eq!(left.stat, StatId::Goals);
                assert_eq!(left.op, FilterOp::Min);
                assert_eq!(right.stat, StatId::Assists);
                assert_eq!(right.op, FilterOp::Min);
            }
            other => panic!("expected AND composition, got {other:?}"),
        }
    }

    #[test]
    fn empty_repeated_filter_set_has_no_expression() {
        let combined = combine_filter_exprs(&[]).expect("empty filter set is valid");
        assert!(combined.is_none());
    }
}
