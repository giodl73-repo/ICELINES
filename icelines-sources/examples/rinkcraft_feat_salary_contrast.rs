//! Descriptive salary-banded contrast of fixed feat outputs, never salary-based scoring.
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
};
fn n(v: &Value, k: &str) -> Result<u64, String> {
    v[k].as_u64().ok_or_else(|| format!("missing integer {k}"))
}
fn read(spec: &Value) -> Result<(Value, String), String> {
    let b = fs::read(spec["path"].as_str().ok_or("path missing")?).map_err(|e| e.to_string())?;
    let h = format!("{:x}", Sha256::digest(&b));
    if spec["sha256"].as_str() != Some(h.as_str()) {
        return Err("input checksum mismatch".into());
    }
    Ok((serde_json::from_slice(&b).map_err(|e| e.to_string())?, h))
}
fn band(cap: u64) -> Option<usize> {
    match cap {
        1..=1_249_999 => Some(0),
        1_250_000..=2_999_999 => Some(1),
        3_000_000..=4_999_999 => Some(2),
        5_000_000..=6_999_999 => Some(3),
        _ => None,
    }
}
fn clock(seconds: f64) -> String {
    let s = seconds.round() as u64;
    format!("{}:{:02}", s / 60, s % 60)
}
fn summary(rows: &[&Value]) -> Value {
    let mut v = rows
        .iter()
        .map(|r| r["positive_total"].as_u64().unwrap())
        .collect::<Vec<_>>();
    v.sort_unstable();
    let count = v.len();
    json!({"players":count,"mean":if count==0{None}else{Some(v.iter().sum::<u64>() as f64/count as f64)},"median":if count==0{None}else{Some(if count%2==1{v[count/2] as f64}else{(v[count/2-1]+v[count/2]) as f64/2.})},"minimum":v.first(),"maximum":v.last(),"forwards":rows.iter().filter(|r|r["group"]=="Forwards").count(),"defense":rows.iter().filter(|r|r["group"]=="Defense").count()})
}
fn qualify(percentiles: &Value, stored: bool) -> Result<bool, String> {
    let p = percentiles
        .as_array()
        .filter(|a| a.len() == 2)
        .ok_or("two conditions required")?;
    let mut q = true;
    for v in p {
        let n = v
            .as_f64()
            .filter(|p| p.is_finite() && (0.0..=100.0).contains(p))
            .ok_or("invalid rank")?;
        q &= n >= 60.;
    }
    if q != stored {
        return Err("stored/rank decision mismatch".into());
    }
    Ok(q)
}
#[path = "support/rinkcraft_lower_tail.rs"]
mod lower_tail;
use lower_tail::low_callouts;

fn actor(
    id: u64,
    reports: &Value,
    time: &Value,
    catalog: &Value,
    old: &Value,
    task: &Value,
) -> Result<Value, String> {
    let source = reports["skaters"]["data"]
        .as_array()
        .ok_or("skater rows missing")?
        .iter()
        .filter(|r| r["playerId"].as_u64() == Some(id))
        .collect::<Vec<_>>();
    if source.len() != 1 {
        return Err("skater identity absent/duplicate".into());
    }
    let source = source[0];
    let name = source["skaterFullName"].as_str().ok_or("name missing")?;
    let position = source["positionCode"]
        .as_str()
        .filter(|p| matches!(*p, "C" | "L" | "R" | "D"))
        .ok_or("not a supported skater")?;
    let group = if position == "D" {
        "Defense"
    } else {
        "Forwards"
    };
    let gp = n(source, "gamesPlayed")?;
    let a = catalog["profiles"]
        .as_array()
        .ok_or("catalog profiles missing")?
        .iter()
        .filter(|p| p["player_id"].as_u64() == Some(id))
        .collect::<Vec<_>>();
    if a.len() != 1 {
        return Err("catalog actor absent/duplicate".into());
    }
    let a = a[0];
    if a["season"] != 20242025
        || a["season_type"] != "regular"
        || a["group"] != group
        || a["games"].as_u64() != Some(gp)
    {
        return Err("catalog/source scope mismatch".into());
    }
    let t = time["data"]
        .as_array()
        .ok_or("TOI rows missing")?
        .iter()
        .filter(|r| r["playerId"].as_u64() == Some(id))
        .collect::<Vec<_>>();
    if t.len() != 1 {
        return Err("TOI actor absent/duplicate".into());
    }
    let t = t[0];
    if t["seasonId"] != 20242025
        || t["gamesPlayed"].as_u64() != Some(gp)
        || t["positionCode"] != position
        || gp < 40
    {
        return Err("TOI identity or availability inclusion mismatch".into());
    }
    let mut positive = Vec::new();
    let mut cautions = Vec::new();
    let mut seen = BTreeSet::new();
    let mut matched = 0;
    for f in a["feats"].as_array().ok_or("feat rows missing")? {
        let fid = f["id"].as_str().ok_or("feat id missing")?;
        if !seen.insert(fid) {
            return Err("duplicate feat definition".into());
        }
        if f["state"] != "earned" {
            continue;
        }
        matched += 1;
        let r = json!({"id":fid,"name":f["name"],"tier":f["tier"]});
        if f["placements"]
            .as_array()
            .ok_or("placements absent")?
            .iter()
            .any(|p| p == "risk_flags")
        {
            cautions.push(r);
        } else {
            positive.push(r);
        }
    }
    if a["earned"].as_u64() != Some(matched) {
        return Err("catalog earned counter differs".into());
    }
    let mut lows = low_callouts(a)?;
    let mut experimental = Vec::new();
    for (fid, name) in [
        ("accuracy-ledger", "Shooting Conversion"),
        ("pp-efficiency", "PP Production Rate"),
        ("penalty-balance", "Relative Whistle Record"),
    ] {
        let r = old["results"]
            .as_array()
            .ok_or("candidate results missing")?
            .iter()
            .filter(|r| r["definition"]["id"] == fid)
            .collect::<Vec<_>>();
        if r.len() != 1 {
            return Err("candidate missing/duplicate".into());
        }
        let primary = &r[0]["primary"];
        if primary["minimum_gp"] != 20
            || primary["threshold"] != 60.
            || primary["exposure_multiplier"] != 1.
        {
            return Err("candidate gates differ".into());
        }
        let g = primary["groups"]
            .as_array()
            .ok_or("candidate groups absent")?
            .iter()
            .find(|g| g["group"] == group)
            .ok_or("candidate group absent")?;
        let p = g["players"]
            .as_array()
            .ok_or("candidate players absent")?
            .iter()
            .find(|p| p["player_id"].as_u64() == Some(id));
        let mut q = false;
        let state = if let Some(p) = p {
            if p["games"].as_u64() != Some(gp) {
                return Err("candidate GP differs".into());
            }
            let conditions = p["conditions"]
                .as_array()
                .ok_or("candidate conditions absent")?
                .iter()
                .map(|c| c["percentile"].clone())
                .collect::<Vec<_>>();
            q = qualify(
                &json!(conditions),
                p["qualifies"]
                    .as_bool()
                    .ok_or("candidate qualification absent")?,
            )?;
            if n(g, "eligible")? < 20 {
                "insufficient_reference_population"
            } else if q {
                "earned"
            } else {
                "not_earned"
            }
        } else {
            "insufficient_exposure"
        };
        if let Some(p) = p.filter(|_| matches!(state, "earned" | "not_earned")) {
            let metric = match fid {
                "accuracy-ledger" => Some("conversion"),
                "pp-efficiency" => Some("pp_points_per60"),
                _ => None,
            };
            if let Some(c) = p["conditions"].as_array().unwrap().iter().find(|c| {
                Some(c["key"].as_str().unwrap_or("")) == metric
                    && c["percentile"]
                        .as_f64()
                        .is_some_and(|v| (0.0..=20.0).contains(&v))
            }) {
                lows.push(json!({"id":format!("low-{fid}"),"kind":"low_opportunity_conditioned_output","label":format!("Low {name}"),"condition":c,"percentile":c["percentile"],"reference_group":group,"eligible_peers":g["eligible"],"minimums":r[0]["definition"]["minimums"],"source":"old_screen","interpretation":"Low measured outcome after the existing candidate exposure gate; not latent ability or an automatic gameplay penalty."}));
            }
        }
        experimental
            .push(json!({"id":fid,"name":name,"state":state,"qualifies":q&&state=="earned"}));
    }
    for (fid, name) in [
        ("oz-draw-record", "O-zone Draws"),
        ("dz-draw-record", "D-zone Draws"),
        ("draw-specialist", "Faceoff Record"),
    ] {
        let r = task["results"]
            .as_array()
            .ok_or("task cases absent")?
            .iter()
            .find(|r| r["id"] == fid && r["group"] == group)
            .ok_or("task case absent")?;
        let s = r["scenarios"]
            .as_array()
            .ok_or("task scenarios absent")?
            .iter()
            .find(|s| s["exposure_multiplier"] == 1.)
            .ok_or("primary task absent")?;
        let m = r["measurements"]
            .as_array()
            .ok_or("task measurement missing")?
            .iter()
            .find(|m| m["player_id"].as_u64() == Some(id))
            .ok_or("task actor missing")?;
        if m["games"].as_u64() != Some(gp) {
            return Err("task GP mismatch".into());
        }
        let eligible = gp >= n(s, "minimum_gp")?
            && n(m, "opportunities")? as f64
                >= s["minimum_opportunities"]
                    .as_f64()
                    .ok_or("task gate missing")?;
        let conflict = !m["issues"]
            .as_array()
            .ok_or("task quality absent")?
            .is_empty();
        let p = s["players"]
            .as_array()
            .ok_or("task players absent")?
            .iter()
            .find(|p| p["player_id"].as_u64() == Some(id));
        let mut q = false;
        let state = if !eligible {
            "insufficient_exposure"
        } else if conflict {
            "source_conflict"
        } else if n(s, "measured")? < 20 {
            "insufficient_reference_population"
        } else {
            let p = p.ok_or("eligible task actor absent")?;
            q = qualify(
                &p["percentiles"],
                p["qualifies_p55_p60_p65"][1]
                    .as_bool()
                    .ok_or("task qualification absent")?,
            )?;
            if q {
                "earned"
            } else {
                "not_earned"
            }
        };
        experimental
            .push(json!({"id":fid,"name":name,"state":state,"qualifies":q&&state=="earned"}));
    }
    let experimental_awards = experimental
        .iter()
        .filter(|p| p["qualifies"] == true)
        .cloned()
        .collect::<Vec<_>>();
    let total = positive.len() + experimental_awards.len();
    let observation_count = total + cautions.len() + lows.len();
    let seconds = n(t, "timeOnIce")?;
    let pp = n(t, "ppTimeOnIce")?;
    let sh = n(t, "shTimeOnIce")?;
    let ev = n(t, "evTimeOnIce")?;
    if seconds != pp + sh + ev {
        return Err("TOI partition mismatch".into());
    }
    let mut row = json!({"player_id":id,"name":name,"position":position,"group":group,"games":gp,"teams":source["teamAbbrevs"],"positive_registry_count":positive.len(),"qualifying_experimental_count":experimental_awards.len(),"positive_total":total,"caution_count":cautions.len(),"positive_registry":positive,"experimental_awards":experimental_awards,"experimental_all":experimental,"cautions":cautions,"usage":{"all_seconds":seconds,"pp_seconds":pp,"sh_seconds":sh,"toi_per_game":seconds as f64/gp as f64,"toi_display":clock(seconds as f64/gp as f64),"pp_display":clock(pp as f64/gp as f64),"sh_display":clock(sh as f64/gp as f64)}});
    row["low_callouts"] = json!(lows);
    row["observation_count"] = json!(observation_count);
    row["five_observation_target_met"] = json!(observation_count >= 5);
    row["coverage_note"] = json!("Entries overlap and are not independent skills. Unmeasured here: entry denial, exits, retrievals, gap/lane protection, net-front coverage, rotations, pressure decisions and PK prevention. Missing assessment is not a limitation.");
    Ok(row)
}
fn run(input: &Value) -> Result<Value, String> {
    if input["schema"] != "rinkcraft.feat-salary-contrast-input.v1" || input["season"] != 20242025 {
        return Err("wrong study window/schema".into());
    }
    let mut hashes = BTreeMap::new();
    let mut data = BTreeMap::new();
    for k in [
        "cohort",
        "reports",
        "timeonice",
        "catalog",
        "old_screen",
        "task_screen",
    ] {
        let (v, h) = read(&input[k])?;
        hashes.insert(k, h);
        data.insert(k, v);
    }
    let (cohort, reports, time, catalog, old, task) = (
        &data["cohort"],
        &data["reports"],
        &data["timeonice"],
        &data["catalog"],
        &data["old_screen"],
        &data["task_screen"],
    );
    if cohort["schema"] != "puck.feat-salary-cohort.v1"
        || cohort["season"] != 20242025
        || reports["season"] != 20242025
        || catalog["protocol"] != "rinkcraft.stat-feats.0.1-experimental"
        || old["season"] != 20242025
        || task["season"] != 20242025
        || task["schema"] != "rinkcraft.task-batch-two-screen.v1"
    {
        return Err("source windows/protocol differ".into());
    }
    if old["protocol"] != "rinkcraft.feat-screen.0.1-exploratory" {
        return Err("old screen protocol differs".into());
    }
    for p in catalog["profiles"].as_array().ok_or("catalog absent")? {
        if p["source_sha256"].as_str() != Some(hashes["reports"].as_str()) {
            return Err("feat catalog/report lineage differs".into());
        }
    }
    let members = cohort["members"]
        .as_array()
        .filter(|a| a.len() == 12)
        .ok_or("twelve preselected skaters required")?;
    let mut rows = Vec::new();
    let mut seen = BTreeSet::new();
    let mut bands: Vec<Vec<usize>> = vec![Vec::new(); 4];
    for m in members {
        let id = n(m, "player_id")?;
        if !seen.insert(id) || m["season"] != 20242025 {
            return Err("duplicate or wrong-season price actor".into());
        }
        let cap = n(m, "reported_cap_hit_usd")?;
        let b = band(cap).ok_or("price outside fixed sample bands")?;
        let u = url::Url::parse(m["source_url"].as_str().ok_or("contract source missing")?)
            .map_err(|e| e.to_string())?;
        if u.scheme() != "https"
            || u.host_str() != Some("puckpedia.com")
            || !u.path().starts_with("/player/")
            || m["source_cells"]["season"] != "2024-25"
            || m["source_cells"]["field"] != "Cap Hit"
        {
            return Err("unmatched contract season/field/source".into());
        }
        let mut row = actor(id, reports, time, catalog, old, task)?;
        if row["name"] != m["name"] {
            return Err("name/actor match differs".into());
        }
        let idx = rows.len();
        row["reported_cap_hit_usd"] = json!(cap);
        row["cap_band"] = json!(b);
        row["contract_source"] = m.clone();
        bands[b].push(idx);
        rows.push(row);
    }
    let labels = [
        "Under $1.25M",
        "$1.25M–under $3M",
        "$3M–under $5M",
        "$5M–under $7M",
    ];
    let mut band_summary = Vec::new();
    for (i, idxs) in bands.iter().enumerate() {
        let refs = idxs.iter().map(|i| &rows[*i]).collect::<Vec<_>>();
        let s = summary(&refs);
        if refs.len() != 3 || s["forwards"] != 2 || s["defense"] != 1 {
            return Err("fixed 2F/1D quota differs".into());
        }
        band_summary.push(json!({"label":labels[i],"index":i,"summary":s}));
    }
    let ids = cohort["benchmark_ids"]
        .as_array()
        .filter(|a| a.len() == 4)
        .ok_or("four benchmark ids required")?;
    let mut benchmark = Vec::new();
    let mut distinct = BTreeSet::new();
    for id in ids {
        let id = id.as_u64().ok_or("invalid benchmark actor")?;
        if seen.contains(&id) || !distinct.insert(id) {
            return Err("benchmark overlaps sample or duplicates".into());
        }
        benchmark.push(actor(id, reports, time, catalog, old, task)?);
    }
    let sample_summary = summary(&rows.iter().collect::<Vec<_>>());
    let star_summary = summary(&benchmark.iter().collect::<Vec<_>>());
    Ok(
        json!({"schema":"rinkcraft.feat-salary-contrast.v1","season":20242025,"source_sha256":hashes,"salary_metric":"Reported historical full annual contract cap hit; not cash salary or retained team charge","reference_population":"Unchanged full NHL same-season F/D conditional references; salary never enters qualification","rows":rows,"bands":band_summary,"sample_summary":sample_summary,"benchmark":benchmark,"benchmark_summary":star_summary,"limits":"Small purposive skater contrast, not a league salary curve, contract value model, independent trait count, causal effect or latent ability. Related feat definitions share stats. GP inclusion40 is not injury assessment or a new feat gate. Goalies excluded from this count comparison because coverage differs. No thresholds or live rules changed."}),
    )
}
fn markdown(v: &Value) -> String {
    let mut s=String::from("# Everyday-player feat comparison —2024–25\n\nReported same-season cap hits. Positive counts include qualifying experimental patterns; caution flags separate. Same full-league references; salary does not score a feat.\n\n| Player | Pos | Cap hit | GP | TOI/GP | Positive feats | Cautions |\n|---|---|---:|---:|---:|---:|---:|\n");
    for r in v["rows"].as_array().unwrap() {
        s.push_str(&format!(
            "| [{}]({}) | {} | ${:.3}M | {} | {} | {} | {} |\n",
            r["name"].as_str().unwrap(),
            r["contract_source"]["source_url"].as_str().unwrap(),
            r["position"].as_str().unwrap(),
            r["reported_cap_hit_usd"].as_u64().unwrap() as f64 / 1e6,
            r["games"],
            r["usage"]["toi_display"].as_str().unwrap(),
            r["positive_total"],
            r["caution_count"]
        ));
    }
    s.push_str("\n## Band means\n\n");
    for b in v["bands"].as_array().unwrap() {
        s.push_str(&format!(
            "- {}: {:.2} positive feats (n=3;2F/1D).\n",
            b["label"].as_str().unwrap(),
            b["summary"]["mean"].as_f64().unwrap()
        ));
    }
    s.push_str(&format!("\nSample mean {:.2}; median {:.1}. Four-star benchmark mean {:.2}. Counts describe this selected sample, not contract value.\n",v["sample_summary"]["mean"].as_f64().unwrap(),v["sample_summary"]["median"].as_f64().unwrap(),v["benchmark_summary"]["mean"].as_f64().unwrap()));
    s.push_str("\n## Four-star benchmark\n\n| Player | Positive feats |\n|---|---:|\n");
    for r in v["benchmark"].as_array().unwrap() {
        s.push_str(&format!(
            "| {} | {} |\n",
            r["name"].as_str().unwrap(),
            r["positive_total"]
        ));
    }
    s.push_str("\nThis sample was selected before feat counts were inspected. Contract cells were producer-read from linked PuckPedia historical tables; they are reported figures, not independent NHL registry verification. TOI is all strengths, not a coaching line/pair assignment. Goalies are outside this count comparison because their feat coverage differs.\n\n## Balanced profile observations\n\nExperimental lower-tail policy: at or below P20, at least 20 games and 20 measured same-position-group peers. Per-game production and event volume are distinguished. Raw PP/SH counts, low penalties and low giveaways are excluded. PP-rate and conversion observations retain the existing candidate exposure gates and conditional reference populations. Positive feat thresholds are unchanged. Five observations is a coverage target, not five independent abilities; overlapping feats and cautions remain overlapping. No gameplay penalties are inferred.\n");
    for r in v["rows"].as_array().unwrap() {
        s.push_str(&format!("\n## {}\n\n", r["name"].as_str().unwrap()));
        let names = r["positive_registry"]
            .as_array()
            .unwrap()
            .iter()
            .chain(r["experimental_awards"].as_array().unwrap())
            .map(|f| f["name"].as_str().unwrap())
            .collect::<Vec<_>>();
        s.push_str(&format!("Qualifying records: {}.\n", names.join("; ")));
        let cautions = r["cautions"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| f["name"].as_str().unwrap())
            .collect::<Vec<_>>();
        s.push_str(&format!(
            "\nCaution flags: {}.\n",
            if cautions.is_empty() {
                "None qualified".into()
            } else {
                cautions.join("; ")
            }
        ));
        s.push_str("\nLower-tail observations:\n\n");
        let lows = r["low_callouts"].as_array().unwrap();
        if lows.is_empty() {
            s.push_str("- None met this annotation policy.\n");
        }
        for c in lows {
            s.push_str(&format!(
                "- **{}**: {:.4}, P{:.1} among {} peers. {}\n",
                c["label"].as_str().unwrap(),
                c["condition"]["value"].as_f64().unwrap(),
                c["percentile"].as_f64().unwrap(),
                r["group"].as_str().unwrap(),
                c["interpretation"].as_str().unwrap()
            ));
        }
        s.push_str(&format!("\nSupported entries: {} positive + {} caution + {} lower-tail = {}. Five-entry coverage target: {}.\n\n{}\n",r["positive_total"],r["caution_count"],lows.len(),r["observation_count"],if r["five_observation_target_met"]==true{"met"}else{"not met; additional assessment needed"},r["coverage_note"].as_str().unwrap()));
    }
    s
}
fn main() {
    let a: Vec<_> = env::args().collect();
    let f = || -> Result<(), String> {
        if a.len() != 4 {
            return Err("Usage: CONTRAST_INPUT OUTPUT_JSON OUTPUT_MD".into());
        }
        let input: Value = serde_json::from_slice(&fs::read(&a[1]).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        let p = fs::read(
            input["protocol"]["path"]
                .as_str()
                .ok_or("protocol missing")?,
        )
        .map_err(|e| e.to_string())?;
        if input["protocol"]["sha256"].as_str()
            != Some(format!("{:x}", Sha256::digest(&p)).as_str())
        {
            return Err("protocol checksum differs".into());
        }
        let result = run(&input)?;
        fs::write(
            &a[2],
            serde_json::to_vec(&result).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        fs::write(&a[3], markdown(&result)).map_err(|e| e.to_string())?;
        println!("Computed fixed-rule salary contrast with separate caution flags.");
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
    fn authored_band_edges() {
        for (cap, expected) in [
            (0, None),
            (1_249_999, Some(0)),
            (1_250_000, Some(1)),
            (2_999_999, Some(1)),
            (3_000_000, Some(2)),
            (4_999_999, Some(2)),
            (5_000_000, Some(3)),
            (6_999_999, Some(3)),
            (7_000_000, None),
        ] {
            assert_eq!(band(cap), expected);
        }
    }
    #[test]
    fn unique_record_summary_does_not_use_salary() {
        let a = json!({"positive_total":3,"group":"Forwards","reported_cap_hit_usd":1});
        let b = json!({"positive_total":9,"group":"Defense","reported_cap_hit_usd":999999});
        let s = summary(&[&a, &b]);
        assert_eq!(s["mean"], 6.);
        assert_eq!(s["median"], 6.);
    }
    #[test]
    fn all_conditions_must_match_source_award() {
        assert!(qualify(&json!([60, 80]), true).unwrap());
        assert!(qualify(&json!([59, 90]), true).is_err());
    }
    #[test]
    fn empty_group_is_not_zero_average() {
        assert!(summary(&[])["mean"].is_null());
        assert_eq!(clock(580.86), "9:41");
    }
    #[test]
    fn low_output_activity_and_favorable_lows_are_distinct() {
        let p = json!({"games":60,"group":"Defense","feats":[{"conditions":[
            {"key":"shots","label":"Shots","value":0.5,"peers":100,"percentile":20},
            {"key":"goals","label":"Goals","value":0.01,"peers":100,"percentile":10},
            {"key":"taken","value":0,"peers":100,"percentile":1},
            {"key":"giveaways","value":0,"peers":100,"percentile":1},
            {"key":"pp-goals","value":0,"peers":100,"percentile":1},
            {"key":"blocks","value":null,"peers":100,"percentile":null}
        ]}]});
        let lows = low_callouts(&p).unwrap();
        assert_eq!(lows.len(), 2);
        assert_eq!(lows[0]["kind"], "low_output");
        assert_eq!(lows[1]["kind"], "low_activity");
        let mut gated = p.clone();
        gated["games"] = json!(19);
        assert!(low_callouts(&gated).unwrap().is_empty());
        gated = p.clone();
        gated["feats"][0]["conditions"][0]["peers"] = json!(19);
        assert_eq!(low_callouts(&gated).unwrap().len(), 1);
    }
    #[test]
    fn repeated_stats_do_not_multiply_callouts_or_hide_conflicts() {
        let c = json!({"key":"blocks","value":0.1,"peers":100,"percentile":5});
        let mut p = json!({"games":60,"feats":[{"conditions":[c.clone()]},{"conditions":[c]}]});
        assert_eq!(low_callouts(&p).unwrap().len(), 1);
        p["feats"][1]["conditions"][0]["percentile"] = json!(6);
        assert!(low_callouts(&p).is_err());
    }
}
