use crate::playoffs_bundle::PlayoffsBundle;
use crate::schema::{GoalieStats, SkaterBio, SkaterStats};

#[derive(Debug, serde::Deserialize)]
pub struct Tier1ReportEnvelope<R> {
    pub data: Vec<R>,
    #[serde(default)]
    pub total: u32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TransactionsEnvelope {
    pub season: String,
    pub source: String,
    pub fetched_at: String,
    pub classifier_version: u16,
    pub rows: Vec<icelines_core::Transaction>,
}

pub fn parse_bios(bytes: &[u8]) -> Result<Vec<SkaterBio>, serde_json::Error> {
    serde_json::from_slice(bytes)
}

pub fn parse_stats(bytes: &[u8]) -> Result<Vec<SkaterStats>, serde_json::Error> {
    serde_json::from_slice(bytes)
}

pub fn parse_goalie_stats(bytes: &[u8]) -> Result<Vec<GoalieStats>, serde_json::Error> {
    serde_json::from_slice(bytes)
}

pub fn parse_transactions(bytes: &[u8]) -> Result<TransactionsEnvelope, serde_json::Error> {
    serde_json::from_slice(bytes)
}

pub fn parse_playoffs_bundle(bytes: &[u8]) -> Result<PlayoffsBundle, serde_json::Error> {
    serde_json::from_slice(bytes)
}

pub fn parse_tier1_report<R>(bytes: &[u8]) -> Result<Tier1ReportEnvelope<R>, serde_json::Error>
where
    R: serde::de::DeserializeOwned,
{
    serde_json::from_slice(bytes)
}

#[cfg(test)]
mod tests {
    use super::{parse_bios, parse_playoffs_bundle, parse_tier1_report, parse_transactions};

    #[test]
    fn rejects_wrong_bios_shape() {
        assert!(parse_bios(br#"{"data":[]}"#).is_err());
    }

    #[test]
    fn parses_transaction_envelope() {
        let envelope = parse_transactions(
            br#"{
                "season":"20252026",
                "source":"fixture",
                "fetched_at":"2026-07-01T00:00:00Z",
                "classifier_version":1,
                "rows":[]
            }"#,
        )
        .expect("valid envelope");
        assert_eq!(envelope.season, "20252026");
    }

    #[test]
    fn parses_playoffs_bundle_contract() {
        let bundle = parse_playoffs_bundle(
            br#"{"season":"19931994","champion":"NYR","conn_smythe":null,"rounds":[]}"#,
        )
        .expect("valid bundle");
        assert_eq!(bundle.champion.as_deref(), Some("NYR"));
    }

    #[test]
    fn tier1_report_accepts_missing_total_for_legacy_artifacts() {
        let report = parse_tier1_report::<serde_json::Value>(br#"{"data":[{"value":1}]}"#)
            .expect("valid legacy report");
        assert_eq!(report.data.len(), 1);
        assert_eq!(report.total, 0);
    }
}
