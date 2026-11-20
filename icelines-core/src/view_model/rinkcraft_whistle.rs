//! Signed records describe counts; relative percentile qualification is separate.
use serde::{Deserialize, Serialize};
pub const RELATIVE_NAME: &str = "Relative Whistle Record";
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WhistleRecord {
    pub schema: String,
    pub drawn: u32,
    pub taken: u32,
    pub net: i64,
    pub sign: String,
    pub display: String,
    pub note: String,
}
pub fn record(drawn: Option<u32>, taken: Option<u32>) -> Option<WhistleRecord> {
    let (drawn, taken) = (drawn?, taken?);
    let net = i64::from(drawn) - i64::from(taken);
    Some(WhistleRecord{schema:"rinkcraft.whistle-record.v1".into(),drawn,taken,net,sign:if net>0{"positive"}else if net<0{"negative"}else{"zero"}.into(),display:format!("Drawn {drawn} · taken {taken} · net {net:+}"),note:"Relative peer ranks do not require positive net. Drawn-minus-taken counts do not establish an actual power play gained, coaching assignment or ability grade.".into()})
}
pub fn positive_gate(drawn: u32, taken: u32) -> bool {
    drawn > taken
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn zero_negative_positive_are_distinct() {
        for (d, t, net, sign) in [
            (0, 0, 0, "zero"),
            (3, 8, -5, "negative"),
            (8, 3, 5, "positive"),
        ] {
            let r = record(Some(d), Some(t)).unwrap();
            assert_eq!((r.net, r.sign.as_str()), (net, sign));
            assert_eq!(positive_gate(d, t), net > 0);
        }
    }
    #[test]
    fn unknown_is_not_zero() {
        assert!(record(None, Some(0)).is_none());
        assert!(record(Some(0), None).is_none());
        assert!(record(Some(0), Some(0)).is_some());
    }
    #[test]
    fn full_count_range_uses_signed_wide_net() {
        let r = record(Some(0), Some(u32::MAX)).unwrap();
        assert_eq!(r.net, -i64::from(u32::MAX));
        assert!(r.display.ends_with("-4294967295"));
    }
}
