//! Local verified capture ingestion; no live requests and no class inference.
use icelines_core::view_model::{
    rinkcraft_numbers::{NumericalCatalog, Source},
    rinkcraft_style::{self, MovementSnapshot},
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::path::Path;

#[derive(Deserialize)]
struct Manifest {
    schema: String,
    season: u32,
    accessed: String,
    records: Vec<Record>,
}
#[derive(Deserialize)]
struct Record {
    player_id: u32,
    url: String,
    sha256: String,
    bytes: usize,
}
fn bounded(path: &Path) -> anyhow::Result<Vec<u8>> {
    anyhow::ensure!(
        std::fs::metadata(path)?.len() <= 1024 * 1024,
        "capture exceeds 1 MiB"
    );
    let bytes = std::fs::read(path)?;
    anyhow::ensure!(bytes.len() <= 1024 * 1024, "capture exceeds 1 MiB");
    Ok(bytes)
}
pub fn enrich_from_capture_dir(catalog: &mut NumericalCatalog, dir: &Path) -> anyhow::Result<()> {
    let manifest: Manifest = serde_json::from_slice(&bounded(&dir.join("manifest.json"))?)?;
    anyhow::ensure!(
        manifest.schema == "icelines.edge-capture.v1" && manifest.season == catalog.season,
        "incompatible capture manifest"
    );
    let mut snapshots = Vec::new();
    for r in manifest.records {
        let bytes = bounded(&dir.join(format!("{}.json", r.player_id)))?;
        anyhow::ensure!(
            bytes.len() == r.bytes && format!("{:x}", Sha256::digest(&bytes)) == r.sha256,
            "capture hash/length mismatch"
        );
        let v: serde_json::Value = serde_json::from_slice(&bytes)?;
        anyhow::ensure!(
            v["player"]["id"].as_u64() == Some(r.player_id as u64),
            "capture player mismatch"
        );
        anyhow::ensure!(
            v["player"]["position"]
                .as_str()
                .is_some_and(|p| matches!(p, "C" | "L" | "R" | "D")),
            "not a skater capture"
        );
        let games = v["player"]["gamesPlayed"]
            .as_u64()
            .and_then(|g| u32::try_from(g).ok())
            .ok_or_else(|| anyhow::anyhow!("missing games"))?;
        let get = |path: &str| -> anyhow::Result<Option<f64>> {
            match v.pointer(path) {
                None | Some(serde_json::Value::Null) => Ok(None),
                Some(n) => {
                    Ok(Some(n.as_f64().ok_or_else(|| {
                        anyhow::anyhow!("non-numeric tracking field")
                    })?))
                }
            }
        };
        snapshots.push(MovementSnapshot {
            player_id: r.player_id,
            season: manifest.season,
            games,
            max_mph: get("/skatingSpeed/speedMax/imperial")?,
            bursts_over_20: get("/skatingSpeed/burstsOver20/value")?,
            distance_miles: get("/totalDistanceSkated/imperial")?,
            source: Source {
                url: r.url,
                sha256: r.sha256,
                bytes: r.bytes,
            },
            accessed: manifest.accessed.clone(),
        });
    }
    rinkcraft_style::enrich(catalog, &snapshots).map_err(anyhow::Error::msg)?;
    catalog.validate().map_err(anyhow::Error::msg)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn verified_capture_rejects_changed_bytes_and_incompatible_season() {
        use icelines_core::view_model::rinkcraft_numbers::{self, InputRow};
        let dir = tempfile::tempdir().unwrap();
        let raw=br#"{"player":{"id":2,"position":"D","gamesPlayed":20},"skatingSpeed":{"speedMax":{"imperial":22.0},"burstsOver20":{"value":40}},"totalDistanceSkated":{"imperial":60.0}}"#;
        std::fs::write(dir.path().join("2.json"), raw).unwrap();
        let manifest = serde_json::json!({"schema":"icelines.edge-capture.v1","season":20242025,"accessed":"2026-11-08","records":[{"player_id":2,"url":"https://api-web.nhle.com/v1/edge/skater-detail/2/20242025/2","bytes":raw.len(),"sha256":format!("{:x}",Sha256::digest(raw))}]});
        std::fs::write(
            dir.path().join("manifest.json"),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();
        let row = InputRow {
            player_id: 2,
            name: "Synthetic".into(),
            group: "Defense".into(),
            games: 20,
            seconds: 30000.,
            values: Default::default(),
        };
        let mut c = rinkcraft_numbers::build(&[row], 20242025, &[], vec![]).unwrap();
        enrich_from_capture_dir(&mut c, dir.path()).unwrap();
        assert_eq!(
            c.profiles[0].style_evidence.as_ref().unwrap().measured[1].value,
            2.
        );
        c.season = 20252026;
        assert!(enrich_from_capture_dir(&mut c, dir.path()).is_err());
        c.season = 20242025;
        std::fs::write(dir.path().join("2.json"), b"{}").unwrap();
        assert!(enrich_from_capture_dir(&mut c, dir.path()).is_err());
    }
}
