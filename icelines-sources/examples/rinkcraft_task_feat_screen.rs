//! Exploratory source-bound faceoff/EV-goalie task records; no live feat changes.
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
};
fn num(v: &Value, k: &str) -> Result<u64, String> {
    v[k].as_u64()
        .filter(|n| *n <= u32::MAX as u64)
        .ok_or_else(|| format!("missing integer {k}"))
}
fn rows<'a>(v: &'a Value, season: u64, route: &str) -> Result<BTreeMap<u64, &'a Value>, String> {
    if v["season"] != season || v["route"] != route {
        return Err("report window/route mismatch".into());
    }
    for source in v["source_urls"]
        .as_array()
        .filter(|s| !s.is_empty())
        .ok_or("missing source URLs")?
    {
        let u =
            url::Url::parse(source.as_str().ok_or("bad source URL")?).map_err(|e| e.to_string())?;
        let q: BTreeMap<_, _> = u.query_pairs().collect();
        let filter = q.get("cayenneExp").ok_or("missing filter")?;
        let f: Vec<_> = filter.split(" and ").collect();
        if u.scheme() != "https"
            || u.host_str() != Some("api.nhle.com")
            || u.path() != format!("/stats/rest/en/{route}")
            || !f.contains(&format!("seasonId={season}").as_str())
            || !f.contains(&"gameTypeId=2")
            || q.get("isGame").map(|v| v.as_ref()) != Some("false")
            || q.get("isAggregate").map(|v| v.as_ref()) != Some("false")
        {
            return Err("invalid source identity/window".into());
        }
    }
    let data = v["data"].as_array().ok_or("missing rows")?;
    if v["total"].as_u64() != Some(data.len() as u64) {
        return Err("partial report".into());
    }
    let mut out = BTreeMap::new();
    for r in data {
        let id = num(r, "playerId")?;
        if id == 0 || num(r, "seasonId")? != season || out.insert(id, r).is_some() {
            return Err("mixed/duplicate actor".into());
        }
    }
    Ok(out)
}
#[derive(Clone)]
struct Measurement {
    id: u64,
    gp: u64,
    opps: u64,
    successes: u64,
    failures: u64,
    issues: Vec<String>,
}
fn count_ok(success: u64, failure: u64, total: u64) -> bool {
    success.checked_add(failure) == Some(total)
}
fn rounding_ok(v: &Value, key: &str, n: u64, d: u64, tol: f64) -> bool {
    if d == 0 {
        return v[key].is_null();
    }
    v[key]
        .as_f64()
        .is_some_and(|x| x.is_finite() && (x - n as f64 / d as f64).abs() <= tol)
}
fn rank(v: f64, reference: &[f64]) -> f64 {
    100. * (reference.iter().filter(|x| **x < v).count() as f64
        + 0.5 * reference.iter().filter(|x| **x == v).count() as f64)
        / reference.len() as f64
}
fn faceoff(r: &Value, s: &Value, prefix: &str) -> Result<Measurement, String> {
    let mut issues = Vec::new();
    let keys = |p: &str| {
        if p == "total" {
            [
                "totalFaceoffs".into(),
                "totalFaceoffWins".into(),
                "totalFaceoffLosses".into(),
            ]
        } else {
            [
                format!("{p}Faceoffs"),
                format!("{p}FaceoffWins"),
                format!("{p}FaceoffLosses"),
            ]
        }
    };
    let k = keys(prefix);
    let (o, w, l) = (num(r, &k[0])?, num(r, &k[1])?, num(r, &k[2])?);
    if !count_ok(w, l, o) {
        issues.push("count_accounting_conflict".into());
    }
    for kind in ["attempts", "wins", "losses"] {
        let total = num(
            r,
            match kind {
                "attempts" => "totalFaceoffs",
                "wins" => "totalFaceoffWins",
                _ => "totalFaceoffLosses",
            },
        )?;
        let zone: [&str; 3] = match kind {
            "attempts" => [
                "offensiveZoneFaceoffs",
                "neutralZoneFaceoffs",
                "defensiveZoneFaceoffs",
            ],
            "wins" => [
                "offensiveZoneFaceoffWins",
                "neutralZoneFaceoffWins",
                "defensiveZoneFaceoffWins",
            ],
            _ => [
                "offensiveZoneFaceoffLosses",
                "neutralZoneFaceoffLosses",
                "defensiveZoneFaceoffLosses",
            ],
        };
        let strength: [&str; 3] = match kind {
            "attempts" => ["evFaceoffs", "ppFaceoffs", "shFaceoffs"],
            "wins" => ["evFaceoffsWon", "ppFaceoffsWon", "shFaceoffsWon"],
            _ => ["evFaceoffsLost", "ppFaceoffsLost", "shFaceoffsLost"],
        };
        for (label, part) in [("zone_partition", zone), ("strength_partition", strength)] {
            let sum = part.iter().try_fold(0_u64, |a, k| {
                num(r, k).and_then(|b| a.checked_add(b).ok_or("count overflow".into()))
            })?;
            if sum != total {
                issues.push(format!("{label}_{kind}_conflict"));
            }
        }
    }
    let (tw, tt) = (num(r, "totalFaceoffWins")?, num(r, "totalFaceoffs")?);
    if !rounding_ok(r, "faceoffWinPct", tw, tt, 1.1e-5)
        || !rounding_ok(s, "faceoffWinPct", tw, tt, 1.1e-5)
    {
        issues.push("published_faceoff_fraction_conflict".into());
    }
    Ok(Measurement {
        id: num(r, "playerId")?,
        gp: num(r, "gamesPlayed")?,
        opps: o,
        successes: w,
        failures: l,
        issues,
    })
}
fn goalie(r: &Value, s: &Value) -> Result<Measurement, String> {
    let mut issues = Vec::new();
    let (sv, sa, ga) = (
        num(r, "evSaves")?,
        num(r, "evShotsAgainst")?,
        num(r, "evGoalsAgainst")?,
    );
    if !count_ok(sv, ga, sa) {
        issues.push("ev_save_accounting_conflict".into());
    }
    if !rounding_ok(r, "evSavePct", sv, sa, 1.1e-6) {
        issues.push("published_ev_fraction_conflict".into());
    }
    for field in ["saves", "shotsAgainst", "goalsAgainst"] {
        if r[field] != s[field] {
            issues.push(format!("summary_{field}_conflict"));
        }
        let suffix = match field {
            "saves" => "Saves",
            "shotsAgainst" => "ShotsAgainst",
            _ => "GoalsAgainst",
        };
        let sum = ["ev", "pp", "sh"].iter().try_fold(0_u64, |a, p| {
            num(r, &format!("{p}{suffix}"))
                .and_then(|b| a.checked_add(b).ok_or("count overflow".into()))
        })?;
        if sum != num(r, field)? {
            issues.push(format!("strength_partition_{field}_conflict"));
        }
    }
    Ok(Measurement {
        id: num(r, "playerId")?,
        gp: num(r, "gamesPlayed")?,
        opps: sa,
        successes: sv,
        failures: ga,
        issues,
    })
}
fn scenario(measures: &[Measurement], minimum: u64, gp: u64, mult: f64) -> Value {
    let gate = minimum as f64 * mult;
    let exposed: Vec<_> = measures
        .iter()
        .filter(|p| p.gp >= gp && p.opps as f64 >= gate)
        .collect();
    let measured: Vec<_> = exposed
        .iter()
        .copied()
        .filter(|p| p.issues.is_empty() && p.opps > 0)
        .collect();
    let n = measured.len();
    let fraction: Vec<_> = measured
        .iter()
        .map(|p| p.successes as f64 / p.opps as f64)
        .collect();
    let workload: Vec<_> = measured
        .iter()
        .map(|p| p.opps as f64 / p.gp as f64)
        .collect();
    let mut awards = [0_usize; 3];
    let mut nonpositive = [0_usize; 3];
    let mut players = Vec::new();
    for p in measured {
        let values = [
            p.successes as f64 / p.opps as f64,
            p.opps as f64 / p.gp as f64,
        ];
        let ranks = if n >= 20 {
            Some([rank(values[0], &fraction), rank(values[1], &workload)])
        } else {
            None
        };
        let mut qs = Vec::new();
        for (i, t) in [55., 60., 65.].iter().enumerate() {
            let q = ranks.map(|r| r.iter().all(|p| *p >= *t));
            awards[i] += usize::from(q == Some(true));
            nonpositive[i] += usize::from(q == Some(true) && p.successes <= p.failures);
            qs.push(q);
        }
        players.push(json!({"player_id":p.id,"games":p.gp,"successes":p.successes,"failures":p.failures,"opportunities":p.opps,"signed_net":p.successes as i64-p.failures as i64,"values":values,"percentiles":ranks,"qualifies_p55_p60_p65":qs}));
    }
    json!({"exposure_multiplier":mult,"minimum_opportunities":gate,"minimum_gp":gp,"eligible_before_validation":exposed.len(),"measured":n,"minimum_peers_met":n>=20,"awards_p55_p60_p65":if n>=20{Some(awards)}else{None},"nonpositive_awards_p55_p60_p65":if n>=20{Some(nonpositive)}else{None},"conflicted_player_ids":exposed.iter().filter(|p|!p.issues.is_empty()).map(|p|p.id).collect::<Vec<_>>(),"players":players})
}
fn run(input: &Value) -> Result<Value, String> {
    if input["schema"] != "rinkcraft.task-batch-two-input.v1" || input["season"] != 20242025 {
        return Err("wrong study/schema".into());
    }
    let season = 20242025;
    let mut hashes = BTreeMap::new();
    let read = |key: &str| -> Result<(Value, String), String> {
        let b = fs::read(input[key]["path"].as_str().ok_or("missing path")?)
            .map_err(|e| e.to_string())?;
        let h = format!("{:x}", Sha256::digest(&b));
        if input[key]["sha256"].as_str() != Some(h.as_str()) {
            return Err("input checksum mismatch".into());
        }
        Ok((serde_json::from_slice(&b).map_err(|e| e.to_string())?, h))
    };
    let (rules, rh) = read("rules")?;
    hashes.insert("rules", rh);
    if rules["schema"] != "rinkcraft.task-batch-two-rules.v1"
        || rules["season"] != season
        || rules["minimum_peers"] != 20
        || rules["thresholds"] != json!([55, 60, 65])
        || rules["exposure_multipliers"] != json!([0.5, 1, 1.5])
    {
        return Err("unsupported screen policy".into());
    }
    let mut reports = BTreeMap::new();
    for k in ["skaters", "faceoffs", "goalies", "saves"] {
        let (v, h) = read(k)?;
        reports.insert(k, v);
        hashes.insert(k, h);
    }
    let sk = rows(&reports["skaters"], season, "skater/summary")?;
    let fo = rows(&reports["faceoffs"], season, "skater/faceoffwins")?;
    let go = rows(&reports["goalies"], season, "goalie/summary")?;
    let sv = rows(&reports["saves"], season, "goalie/savesByStrength")?;
    if sk.keys().ne(fo.keys()) || go.keys().ne(sv.keys()) {
        return Err("joined actor sets differ".into());
    }
    for (a, b) in [(&sk, &fo), (&go, &sv)] {
        for (id, r) in a {
            let p = b[id];
            if let Some(code) = r.get("positionCode") {
                if !matches!(code.as_str(), Some("C" | "L" | "R" | "D")) {
                    return Err("unknown skater position".into());
                }
            }
            if r["gamesPlayed"] != p["gamesPlayed"]
                || num(r, "gamesPlayed")? == 0
                || r.get("positionCode") != p.get("positionCode")
            {
                return Err("joined GP/position mismatch".into());
            }
        }
    }
    let definitions = rules["rules"]
        .as_array()
        .filter(|r| r.len() == 4)
        .ok_or("four rules required")?;
    let mut seen = BTreeSet::new();
    let mut results = Vec::new();
    for rule in definitions {
        let id = rule["id"].as_str().ok_or("missing rule id")?;
        if !seen.insert(id) {
            return Err("duplicate rule".into());
        }
        let (prefix, gp, opp, branch) = match id {
            "draw-specialist" => ("total", 20, 300, "skater"),
            "oz-draw-record" => ("offensiveZone", 20, 100, "skater"),
            "dz-draw-record" => ("defensiveZone", 20, 100, "skater"),
            "even-strength-saving" => ("ev", 15, 500, "goalie"),
            _ => return Err("unsupported rule".into()),
        };
        if rule["prefix"] != prefix
            || rule["minimum_gp"] != gp
            || rule["minimum_opportunities"] != opp
            || rule["branch"] != branch
        {
            return Err("frozen rule differs".into());
        }
        for group in if branch == "goalie" {
            vec!["Goalies"]
        } else {
            vec!["Forwards", "Defense"]
        } {
            let mut m = Vec::new();
            if branch == "goalie" {
                for (id, r) in &sv {
                    m.push(goalie(r, go[id])?);
                }
            } else {
                for (id, r) in &fo {
                    let actual = if r["positionCode"] == "D" {
                        "Defense"
                    } else {
                        "Forwards"
                    };
                    if actual == group {
                        m.push(faceoff(r, sk[id], prefix)?);
                    }
                }
            }
            let cases: Vec<_> = [0.5, 1., 1.5]
                .iter()
                .map(|mult| scenario(&m, opp, gp, *mult))
                .collect();
            results.push(json!({"id":id,"name":rule["name"],"group":group,"observed":m.len(),"measurements":m.iter().map(|p|json!({"player_id":p.id,"games":p.gp,"successes":p.successes,"failures":p.failures,"opportunities":p.opps,"issues":p.issues})).collect::<Vec<_>>(),"scenarios":cases}));
        }
    }
    // Supplementary overlap check, added after initial screen; no threshold tuning.
    let mut overlaps = Vec::new();
    for i in 0..results.len() {
        for j in i + 1..results.len() {
            let (a, b) = (&results[i], &results[j]);
            if a["group"] != b["group"] {
                continue;
            }
            let primary = |r: &Value| {
                r["scenarios"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|s| s["exposure_multiplier"] == 1.)
                    .unwrap()
                    .clone()
            };
            let (a_case, b_case) = (primary(a), primary(b));
            let ids = |v: &Value, awards: bool| {
                v["players"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|p| !awards || p["qualifies_p55_p60_p65"][1] == true)
                    .map(|p| p["player_id"].as_u64().unwrap())
                    .collect::<BTreeSet<_>>()
            };
            let common: BTreeSet<_> = ids(&a_case, false)
                .intersection(&ids(&b_case, false))
                .copied()
                .collect();
            let aa: BTreeSet<_> = ids(&a_case, true).intersection(&common).copied().collect();
            let bb: BTreeSet<_> = ids(&b_case, true).intersection(&common).copied().collect();
            let both = aa.intersection(&bb).count();
            let union = aa.union(&bb).count();
            overlaps.push(json!({"a":a["id"],"b":b["id"],"group":a["group"],"common_eligible":common.len(),"a_awards_in_common":aa.len(),"b_awards_in_common":bb.len(),"both":both,"jaccard":if union==0{None}else{Some(both as f64/union as f64)}}));
        }
    }
    Ok(
        json!({"posthoc_primary_overlaps":overlaps,"schema":"rinkcraft.task-batch-two-screen.v1","season":season,"status":"exploratory_no_promotion","input_sha256":hashes,"population":{"skaters":sk.len(),"goalies":go.len()},"results":results,"notice":"Same-season source-bound record conjunctions, not latent skill, assigned duties, possession after faceoffs, PK saving, prospective validation or simulator bonuses. EV is not restricted to5v5. Conflicted measurements excluded without correction; selection bias remains possible."}),
    )
}
fn main() {
    let a: Vec<_> = env::args().collect();
    let f = || -> Result<(), String> {
        if a.len() != 3 {
            return Err("Usage: TASK_BATCH_INPUT OUTPUT".into());
        }
        let i: Value = serde_json::from_slice(&fs::read(&a[1]).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        let protocol = fs::read(
            i["protocol"]["path"]
                .as_str()
                .ok_or("missing protocol path")?,
        )
        .map_err(|e| e.to_string())?;
        if i["protocol"]["sha256"].as_str()
            != Some(format!("{:x}", Sha256::digest(&protocol)).as_str())
        {
            return Err("protocol checksum mismatch".into());
        }
        fs::write(
            &a[2],
            serde_json::to_vec(&run(&i)?).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        println!("Faceoff-zone and EV goalie screen complete; no live feat changes.");
        Ok(())
    };
    if let Err(e) = f() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn missing_zero_and_count_conflicts_are_distinct() {
        assert!(num(&json!({}), "x").is_err());
        assert_eq!(num(&json!({"x":0}), "x").unwrap(), 0);
        assert!(count_ok(3, 2, 5));
        assert!(!count_ok(3, 2, 4));
    }
    #[test]
    fn zero_denominator_is_not_a_zero_rate() {
        assert!(rounding_ok(&json!({"pct":null}), "pct", 0, 0, 1e-6));
        assert!(!rounding_ok(&json!({"pct":0}), "pct", 0, 0, 1e-6));
    }
    #[test]
    fn minimum_reference_and_real_zero_are_kept() {
        let m = Measurement {
            id: 1,
            gp: 20,
            opps: 300,
            successes: 0,
            failures: 300,
            issues: vec![],
        };
        let s = scenario(&[m], 300, 20, 1.);
        assert_eq!(s["measured"], 1);
        assert!(s["players"][0]["percentiles"].is_null());
        assert_eq!(s["players"][0]["successes"], 0);
    }
    #[test]
    fn conflicted_row_does_not_enter_reference() {
        let m = Measurement {
            id: 1,
            gp: 20,
            opps: 500,
            successes: 450,
            failures: 51,
            issues: vec!["conflict".into()],
        };
        let s = scenario(&[m], 500, 15, 1.);
        assert_eq!(s["eligible_before_validation"], 1);
        assert_eq!(s["measured"], 0);
        assert_eq!(s["conflicted_player_ids"], json!([1]));
    }
    #[test]
    fn ties_get_midrank() {
        assert_eq!(rank(0., &[0., 0., 1.]), 100. / 3.);
    }
}
