//! Source/window-bound faceoff records and fixed-reference boundary displays.
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
};
fn n(v: &Value, k: &str) -> Result<u64, String> {
    v[k].as_u64()
        .filter(|n| *n <= u32::MAX as u64)
        .ok_or_else(|| format!("invalid count {k}"))
}
fn read(spec: &Value) -> Result<(Value, String), String> {
    let b =
        fs::read(spec["path"].as_str().ok_or("missing source path")?).map_err(|e| e.to_string())?;
    let h = format!("{:x}", Sha256::digest(&b));
    if spec["sha256"].as_str() != Some(h.as_str()) {
        return Err("source checksum mismatch".into());
    }
    Ok((serde_json::from_slice(&b).map_err(|e| e.to_string())?, h))
}
struct Window {
    season: u64,
    screen: Value,
    report: Value,
    rules: Value,
    screen_hash: String,
    report_hash: String,
    rules_hash: String,
    captured: String,
}
fn load(e: &Value) -> Result<Window, String> {
    let year = n(e, "season")?;
    let (screen, sh) = read(&e["screen"])?;
    let (report, rh) = read(&e["faceoffs"])?;
    let (rules, dh) = read(&e["rules"])?;
    let (capture, _) = read(&e["capture"])?;
    if screen["season"] != year
        || report["season"] != year
        || rules["season"] != year
        || capture["season"] != year
        || screen["schema"] != "rinkcraft.task-batch-two-screen.v1"
        || report["route"] != "skater/faceoffwins"
        || screen["input_sha256"]["faceoffs"].as_str() != Some(rh.as_str())
        || screen["input_sha256"]["rules"].as_str() != Some(dh.as_str())
    {
        return Err("screen/report/rules binding mismatch".into());
    }
    let rows = report["data"].as_array().ok_or("missing source rows")?;
    let mut ids = BTreeSet::new();
    for r in rows {
        if n(r, "seasonId")? != year || !ids.insert(n(r, "playerId")?) {
            return Err("mixed/duplicate source actor".into());
        }
    }
    if report["total"].as_u64() != Some(rows.len() as u64) {
        return Err("partial faceoff report".into());
    }
    for v in report["source_urls"]
        .as_array()
        .filter(|v| !v.is_empty())
        .ok_or("missing URLs")?
    {
        let u = url::Url::parse(v.as_str().ok_or("bad URL")?).map_err(|e| e.to_string())?;
        let q: BTreeMap<_, _> = u.query_pairs().collect();
        let f = q.get("cayenneExp").ok_or("missing source filter")?;
        if u.scheme() != "https"
            || u.host_str() != Some("api.nhle.com")
            || u.path() != "/stats/rest/en/skater/faceoffwins"
            || !f.split(" and ").any(|v| v == format!("seasonId={year}"))
            || !f.split(" and ").any(|v| v == "gameTypeId=2")
        {
            return Err("wrong source route/window".into());
        }
    }
    let captured = capture["pages"]
        .as_array()
        .ok_or("capture pages missing")?
        .iter()
        .filter(|p| p["kind"] == "faceoffs")
        .filter_map(|p| p["accessed_utc"].as_str())
        .max()
        .ok_or("faceoff capture date absent")?
        .to_owned();
    Ok(Window {
        season: year,
        screen,
        report,
        rules,
        screen_hash: sh,
        report_hash: rh,
        rules_hash: dh,
        captured,
    })
}
fn rank(value: f64, reference: &[f64]) -> f64 {
    100. * (reference.iter().filter(|v| **v < value).count() as f64
        + 0.5 * reference.iter().filter(|v| **v == value).count() as f64)
        / reference.len() as f64
}
fn boundary(
    w: u64,
    l: u64,
    reference: &[f64],
    work_rank: f64,
    original: bool,
    assessed: bool,
) -> Value {
    let total = w + l;
    let notice="Fixed original peer reference; one draw result reclassified, total attempts unchanged. Not a confidence interval, error probability or alternative game simulation.";
    if !assessed || total == 0 || reference.len() < 20 {
        return json!({"schema":"rinkcraft.faceoff-boundary.v1","state":"unassessed","variants":[],"notice":notice});
    }
    let mut variants = Vec::new();
    for (label, ww, ll) in [
        ("win_to_loss", w.checked_sub(1), l.checked_add(1)),
        ("loss_to_win", w.checked_add(1), l.checked_sub(1)),
    ] {
        if let (Some(ww), Some(ll)) = (ww, ll) {
            let frac = ww as f64 / total as f64;
            let p = rank(frac, reference);
            variants.push(json!({"change":label,"wins":ww,"losses":ll,"fraction":frac,"fraction_percentile":p,"workload_percentile":work_rank,"qualifies":p>=60.&&work_rank>=60.}));
        }
    }
    let sensitive = variants
        .iter()
        .any(|v| v["qualifies"].as_bool() != Some(original));
    json!({"schema":"rinkcraft.faceoff-boundary.v1","state":if sensitive{"one_draw_sensitive"}else{"unchanged_in_test"},"variants":variants,"notice":notice})
}
fn observation(
    w: &Window,
    actor: u64,
    group: &str,
    expected_gp: Option<u64>,
    id: &str,
) -> Result<Value, String> {
    let (prefix, min, name, context) = match id {
        "oz-draw-record" => ("offensiveZone", 100, "O-zone Draws", "Offensive zone"),
        "dz-draw-record" => ("defensiveZone", 100, "D-zone Draws", "Defensive zone"),
        "draw-specialist" => ("total", 300, "Faceoff Record", "All zones"),
        _ => return Err("unsupported task id".into()),
    };
    let rule = w.rules["rules"]
        .as_array()
        .ok_or("rules missing")?
        .iter()
        .find(|r| r["id"] == id)
        .ok_or("rule absent")?;
    if rule["minimum_gp"] != 20 || rule["minimum_opportunities"] != min || rule["prefix"] != prefix
    {
        return Err("task rule differs".into());
    }
    let evidence = json!({"schema":"rinkcraft.task-evidence.v1","scope_kind":"season","player_id":actor,"season":w.season,"season_type":"regular","group":group,"source_urls":w.report["source_urls"],"source_sha256":w.report_hash,"screen_sha256":w.screen_hash,"rules_sha256":w.rules_hash,"captured_through":w.captured});
    let source = w.report["data"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["playerId"].as_u64() == Some(actor));
    let Some(source) = source else {
        if expected_gp.is_some() {
            return Err("selected actor absent".into());
        }
        return Ok(
            json!({"season":w.season,"state":"not_observed","conditions":[],"eligible_peers":0,"evidence":evidence}),
        );
    };
    let actual = if source["positionCode"] == "D" {
        "Defense"
    } else {
        "Forwards"
    };
    if actual != group {
        if expected_gp.is_some() {
            return Err("selected actor group differs".into());
        }
        return Ok(
            json!({"season":w.season,"state":"group_changed","conditions":[],"eligible_peers":0,"evidence":evidence}),
        );
    }
    let gp = n(source, "gamesPlayed")?;
    if expected_gp.is_some_and(|v| v != gp) {
        return Err("display/source GP mismatch".into());
    }
    let wins = n(source, &format!("{prefix}FaceoffWins"))?;
    let losses = n(source, &format!("{prefix}FaceoffLosses"))?;
    let attempts = n(source, &format!("{prefix}Faceoffs"))?;
    if wins + losses != attempts || gp == 0 {
        return Err("draw accounting invalid".into());
    }
    let case = w.screen["results"]
        .as_array()
        .ok_or("screen cases absent")?
        .iter()
        .find(|r| r["id"] == id && r["group"] == group)
        .ok_or("group case absent")?;
    let primary = case["scenarios"]
        .as_array()
        .ok_or("scenarios absent")?
        .iter()
        .find(|s| s["exposure_multiplier"] == 1.)
        .ok_or("primary case absent")?;
    if primary["minimum_gp"] != 20 || primary["minimum_opportunities"].as_f64() != Some(min as f64)
    {
        return Err("primary gates differ".into());
    }
    let measurement = case["measurements"]
        .as_array()
        .ok_or("measurements absent")?
        .iter()
        .find(|m| m["player_id"].as_u64() == Some(actor))
        .ok_or("actor measurement absent")?;
    if n(measurement, "games")? != gp
        || n(measurement, "successes")? != wins
        || n(measurement, "failures")? != losses
        || n(measurement, "opportunities")? != attempts
    {
        return Err("source/screen record mismatch".into());
    }
    let measured = n(primary, "measured")?;
    let has_conflict = !measurement["issues"]
        .as_array()
        .ok_or("quality absent")?
        .is_empty();
    let assessed = !has_conflict && gp >= 20 && attempts >= min && measured >= 20;
    let player = primary["players"]
        .as_array()
        .ok_or("players absent")?
        .iter()
        .find(|p| p["player_id"].as_u64() == Some(actor));
    let state = if has_conflict {
        "source_conflict"
    } else if gp < 20 || attempts < min {
        "insufficient_exposure"
    } else if measured < 20 {
        "insufficient_reference_population"
    } else {
        let p = player.ok_or("eligible player missing")?;
        if p["qualifies_p55_p60_p65"][1] == true {
            "earned"
        } else {
            "not_earned"
        }
    };
    let mut conditions = Vec::new();
    let mut work_rank = 0.;
    if assessed {
        let p = player.unwrap();
        let values = p["values"]
            .as_array()
            .filter(|v| v.len() == 2)
            .ok_or("two task values required")?;
        let ranks = p["percentiles"]
            .as_array()
            .filter(|v| v.len() == 2)
            .ok_or("two ranks required")?;
        for (i, key) in ["faceoff_fraction", "faceoffs_pg"].iter().enumerate() {
            let value = values[i].as_f64().ok_or("task value missing")?;
            let pct = ranks[i]
                .as_f64()
                .filter(|p| p.is_finite() && (0.0..=100.0).contains(p))
                .ok_or("rank missing")?;
            let expected = if i == 0 {
                wins as f64 / attempts as f64
            } else {
                attempts as f64 / gp as f64
            };
            if (value - expected).abs() > 1e-12 {
                return Err("source/task rate differs".into());
            }
            if i == 1 {
                work_rank = pct;
            }
            conditions.push(json!({"key":key,"value":value,"percentile":pct,"direction":"higher"}));
        }
        if (state == "earned")
            != conditions
                .iter()
                .all(|c| c["percentile"].as_f64().unwrap() >= 60.)
        {
            return Err("rank/award mismatch".into());
        }
    }
    let fraction = if attempts == 0 {
        None
    } else {
        Some(wins as f64 / attempts as f64)
    };
    let display = match fraction {
        Some(f) => format!("{wins} W · {losses} L · {:.2}%", 100. * f),
        None => "0 draws · no observed rate".into(),
    };
    let precision = if attempts == 0 {
        "One-draw sensitivity unavailable without attempts.".into()
    } else {
        format!(
            "One result changes the observed win fraction by {:.3} percentage points.",
            100. / attempts as f64
        )
    };
    let reference = primary["players"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|p| p["values"][0].as_f64())
        .collect::<Vec<_>>();
    if reference.len() != measured as usize {
        return Err("reference size differs".into());
    }
    let boundary = boundary(
        wins,
        losses,
        &reference,
        work_rank,
        state == "earned",
        assessed,
    );
    Ok(
        json!({"id":id,"name":name,"season":w.season,"placement":"task_skills","state":state,"reference_mode":"same_season","reference_label":format!("{}–{:02} measured eligible {} peers",w.season/10000,w.season%100,group),"eligible_peers":measured,"threshold":60,"conditions":conditions,"opportunities":{"games":gp,"wins":wins,"losses":losses,"draws":attempts,"minimum_draws":min},"minimums":{"games":20,"draws":min},"task_record":{"schema":"rinkcraft.faceoff-record.v1","context":context,"wins":wins,"losses":losses,"attempts":attempts,"games":gp,"fraction":fraction,"attempts_per_game":attempts as f64/gp as f64,"signed_net":wins as i64-losses as i64,"display":display,"precision_display":precision},"task_boundary":boundary,"evidence":evidence,"note":"Observed draw win fraction AND task workload. Zone context does not establish assigned duty, possession secured after the draw, pressure response or latent skill. Experimental thresholds, no ability grade or simulator effect."}),
    )
}
fn run(profiles: &Value, profile_hash: &str, input: &Value) -> Result<Value, String> {
    if input["schema"] != "rinkcraft.faceoff-profile-input.v1" {
        return Err("wrong input schema".into());
    }
    let protocol = fs::read(input["protocol"]["path"].as_str().ok_or("method absent")?)
        .map_err(|e| e.to_string())?;
    let method_hash = format!("{:x}", Sha256::digest(&protocol));
    if input["protocol"]["sha256"].as_str() != Some(method_hash.as_str()) {
        return Err("method checksum mismatch".into());
    }
    let map = profiles
        .as_object()
        .ok_or("explicit profile map required")?;
    let max = map
        .values()
        .map(|p| n(&p["profile_window"], "season"))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .max()
        .ok_or("empty profile map")?;
    let mut windows = BTreeMap::new();
    for e in input["windows"].as_array().ok_or("window sources absent")? {
        let season = n(e, "season")?;
        if season > max {
            continue;
        }
        if windows.insert(season, load(e)?).is_some() {
            return Err("duplicate source window".into());
        }
    }
    let mut patches = BTreeMap::new();
    for (key, p) in map {
        let w = &p["profile_window"];
        let actor = n(&p["request"], "player_id")?;
        let season = n(w, "season")?;
        let group = p["numerical_evidence"]["group"]
            .as_str()
            .ok_or("group absent")?;
        if w["coverage"] != "selected_season"
            || p["experimental_patterns"]["schema"] != "rinkcraft.scoped-patterns.v1"
            || p["experimental_patterns"]["season_type"] != "regular"
            || w["kind"] != "season"
            || w["season_type"] != "regular"
            || w["player_id"].as_u64() != Some(actor)
            || p["experimental_patterns"]["player_id"].as_u64() != Some(actor)
            || p["experimental_patterns"]["season"] != season
            || p["experimental_patterns"]["group"] != group
            || p["numerical_evidence"]["season"] != season
        {
            return Err("profile scope mismatch".into());
        }
        let mut patterns = Vec::new();
        if group != "Goalies" {
            if !matches!(group, "Forwards" | "Defense") {
                return Err("unknown task group".into());
            }
            let selected = windows
                .get(&season)
                .ok_or("selected source window absent")?;
            let gp = n(&p["latest_available"], "games")?;
            for id in ["oz-draw-record", "dz-draw-record", "draw-specialist"] {
                let mut pattern = observation(selected, actor, group, Some(gp), id)?;
                let history = windows
                    .iter()
                    .filter(|(year, _)| **year < season)
                    .map(|(_, src)| observation(src, actor, group, None, id))
                    .collect::<Result<Vec<_>, _>>()?;
                pattern["history"] = Value::Array(history);
                patterns.push(pattern);
            }
        }
        patches.insert(key.clone(),json!({"player_id":actor,"season":season,"season_type":"regular","group":group,"patterns":patterns}));
    }
    Ok(
        json!({"schema":"rinkcraft.faceoff-profile-patches.v1","profile_input_sha256":profile_hash,"method_sha256":method_hash,"patches":patches}),
    )
}
fn main() {
    let a: Vec<_> = env::args().collect();
    let f = || -> Result<(), String> {
        if a.len() != 4 {
            return Err("Usage: THREE_SCOPE_PROFILES TASK_INPUT OUTPUT_PATCHES".into());
        }
        let b = fs::read(&a[1]).map_err(|e| e.to_string())?;
        let p: Value = serde_json::from_slice(&b).map_err(|e| e.to_string())?;
        let i: Value = serde_json::from_slice(&fs::read(&a[2]).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        fs::write(
            &a[3],
            serde_json::to_vec(&run(&p, &format!("{:x}", Sha256::digest(&b)), &i)?)
                .map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        println!("Projected bounded-history faceoff records and fixed-reference count boundaries.");
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
    fn one_draw_reclassification_couples_counts() {
        let refs = (0..100).map(|i| i as f64 / 100.).collect::<Vec<_>>();
        let b = boundary(60, 40, &refs, 80., true, true);
        for v in b["variants"].as_array().unwrap() {
            assert_eq!(
                v["wins"].as_u64().unwrap() + v["losses"].as_u64().unwrap(),
                100
            );
        }
        assert_eq!(b["variants"][0]["wins"], 59);
        assert_eq!(b["variants"][0]["losses"], 41);
    }
    #[test]
    fn unsupported_boundary_is_not_robustness() {
        assert_eq!(boundary(0, 0, &[], 0., false, false)["state"], "unassessed");
        assert_eq!(
            boundary(10, 10, &[0.5], 60., true, true)["state"],
            "unassessed"
        );
    }
    #[test]
    fn impossible_negative_draw_is_excluded() {
        let refs = vec![0.5; 20];
        let b = boundary(0, 10, &refs, 80., false, true);
        assert_eq!(b["variants"].as_array().unwrap().len(), 1);
        assert_eq!(b["variants"][0]["change"], "loss_to_win");
    }
    #[test]
    fn fixed_midrank_is_deterministic() {
        assert_eq!(rank(0.5, &[0.5, 0.5]), 50.);
    }
}
