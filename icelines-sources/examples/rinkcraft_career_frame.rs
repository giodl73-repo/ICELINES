//! Reconciled NHL regular-season career records. No career peer or ability scores.
use chrono::Datelike;
use icelines_core::{
    identity::PlayerId,
    model::Season,
    season_stats::SeasonType,
    view_model::rinkcraft_deployment::{self, UsageInput},
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, env, fs};
fn decode(b: &[u8]) -> Result<Value, String> {
    serde_json::from_slice(b).map_err(|e| e.to_string())
}
fn hash(b: &[u8]) -> String {
    format!("{:x}", Sha256::digest(b))
}
fn count(v: &Value, key: &str) -> Result<u64, String> {
    v[key]
        .as_u64()
        .ok_or_else(|| format!("missing/invalid count {key}"))
}
fn add(a: u64, b: u64) -> Result<u64, String> {
    a.checked_add(b).ok_or("count overflow".into())
}
fn clock_seconds(s: &str) -> Result<u64, String> {
    let parts: Vec<_> = s.split(':').collect();
    if parts.len() != 2 {
        return Err("expected minutes:seconds".into());
    }
    let m = parts[0].parse::<u64>().map_err(|_| "invalid minutes")?;
    let sec = parts[1].parse::<u64>().map_err(|_| "invalid seconds")?;
    if sec >= 60 {
        return Err("invalid clock seconds".into());
    }
    m.checked_mul(60)
        .and_then(|n| n.checked_add(sec))
        .ok_or("clock overflow".into())
}
fn index(report: &Value, actor: u64) -> Result<BTreeMap<u32, &Value>, String> {
    let rows = report["data"].as_array().ok_or("missing report rows")?;
    if report["total"].as_u64() != Some(rows.len() as u64) || rows.is_empty() {
        return Err("incomplete career report".into());
    }
    let mut map = BTreeMap::new();
    for row in rows {
        let season = count(row, "seasonId")? as u32;
        if count(row, "playerId")? != actor
            || season / 10000 + 1 != season % 10000
            || map.insert(season, row).is_some()
        {
            return Err("career actor/year/duplicate mismatch".into());
        }
    }
    Ok(map)
}
fn reconcile(
    ledger: &[&Value],
    rest: &BTreeMap<u32, &Value>,
    published: &Value,
    fields: &[(&str, &str, &str)],
) -> Result<(BTreeMap<String, u64>, Vec<Value>), String> {
    let mut sums = BTreeMap::new();
    let mut checks = Vec::new();
    let mut years: BTreeMap<u32, Vec<&Value>> = BTreeMap::new();
    for row in ledger {
        years
            .entry(count(row, "season")? as u32)
            .or_default()
            .push(row);
    }
    if years.keys().ne(rest.keys()) {
        return Err("landing and REST career years differ".into());
    }
    for (key, landing_key, rest_key) in fields {
        let mut ledger_sum = 0;
        let mut rest_sum = 0;
        for (year, stints) in &years {
            let mut total = 0;
            for stint in stints {
                total = add(total, count(stint, landing_key)?)?;
            }
            let other = count(rest[year], rest_key)?;
            if total != other {
                return Err(format!(
                    "career season {year} {key} mismatch (including possible doubled trade totals)"
                ));
            }
            ledger_sum = add(ledger_sum, total)?;
            rest_sum = add(rest_sum, other)?;
        }
        let career = count(published, landing_key)?;
        if ledger_sum != career || rest_sum != career {
            return Err(format!("career published {key} mismatch"));
        }
        sums.insert((*key).into(), career);
        checks.push(json!({"key":key,"published":career,"landing_seasons":ledger_sum,"rest_seasons":rest_sum,"state":"reconciled"}));
    }
    Ok((sums, checks))
}
fn value(key: &str, label: &str, unit: &str, num: u64, den: u64, scale: f64) -> Value {
    json!({"key":key,"label":label,"domain":"NHL career recorded outcomes","unit":unit,"value":if den==0{None}else{Some(num as f64/den as f64*scale)},"percentile":null,"numerator":num,"denominator":den,"scale":scale,"measured_peers":0,"reference_sha256":"","state":"unranked_no_career_reference","formula":format!("{num} / {den} * {scale}"),"interpretation":"Pooled NHL regular-season career record, not a capability, era-adjusted comparison or simulated game parameter."})
}
fn project(base: &Value, input: &Value, bytes: &[u8]) -> Result<Value, String> {
    if input["schema"] != "rinkcraft.career-capture.v1" {
        return Err("unsupported career capture schema".into());
    }
    let actors = input["players"].as_array().ok_or("missing career actors")?;
    let captures = input["captures"]
        .as_array()
        .ok_or("missing career captures")?;
    let mut output = serde_json::Map::new();
    for actor in actors {
        let id = count(actor, "player_id")?;
        let key = actor["key"].as_str().ok_or("missing fixture key")?;
        let old = base.get(key).ok_or("actor absent from baseline")?;
        if old["request"]["player_id"].as_u64() != Some(id)
            || actor["landing"]["playerId"].as_u64() != Some(id)
        {
            return Err("career identity mismatch".into());
        }
        let landing = &actor["landing"];
        let goalie = landing["position"] == "G";
        let main_kind = if goalie {
            "goalie-summary"
        } else {
            "skater-summary"
        };
        let rest = index(&actor["reports"][main_kind], id)?;
        let rows = landing["seasonTotals"]
            .as_array()
            .ok_or("missing landing season ledger")?;
        let ledger: Vec<_> = rows
            .iter()
            .filter(|r| r["leagueAbbrev"] == "NHL" && r["gameTypeId"] == 2)
            .collect();
        if ledger.is_empty() {
            return Err("no NHL regular-season career rows".into());
        }
        let published = &landing["careerTotals"]["regularSeason"];
        let skater_fields = [
            ("games", "gamesPlayed", "gamesPlayed"),
            ("goals", "goals", "goals"),
            ("assists", "assists", "assists"),
            ("points", "points", "points"),
            ("shots", "shots", "shots"),
            ("pim", "pim", "penaltyMinutes"),
            ("pp_goals", "powerPlayGoals", "ppGoals"),
            ("pp_points", "powerPlayPoints", "ppPoints"),
            ("sh_goals", "shorthandedGoals", "shGoals"),
            ("sh_points", "shorthandedPoints", "shPoints"),
            ("gwg", "gameWinningGoals", "gameWinningGoals"),
            ("ot_goals", "otGoals", "otGoals"),
        ];
        let goalie_fields = [
            ("games", "gamesPlayed", "gamesPlayed"),
            ("starts", "gamesStarted", "gamesStarted"),
            ("wins", "wins", "wins"),
            ("losses", "losses", "losses"),
            ("ot_losses", "otLosses", "otLosses"),
            ("shots_against", "shotsAgainst", "shotsAgainst"),
            ("goals_against", "goalsAgainst", "goalsAgainst"),
            ("shutouts", "shutouts", "shutouts"),
        ];
        let (mut sums, mut checks) = reconcile(
            &ledger,
            &rest,
            published,
            if goalie {
                &goalie_fields[..]
            } else {
                &skater_fields[..]
            },
        )?;
        if !goalie {
            for (kind, fields) in [
                (
                    "skater-realtime",
                    vec![
                        ("hits", "hits"),
                        ("blocks", "blockedShots"),
                        ("takeaways", "takeaways"),
                        ("giveaways", "giveaways"),
                    ],
                ),
                (
                    "skater-penalties",
                    vec![("drawn", "penaltiesDrawn"), ("taken", "penalties")],
                ),
            ] {
                let extra = index(&actor["reports"][kind], id)?;
                if extra.keys().ne(rest.keys()) {
                    return Err(format!("career {kind} year coverage differs"));
                }
                for (year, row) in &extra {
                    if row["gamesPlayed"] != rest[year]["gamesPlayed"] {
                        return Err(format!("career {kind} GP mismatch"));
                    }
                }
                for (key, source_key) in fields {
                    let missing: Vec<_> = extra
                        .iter()
                        .filter(|(_, r)| r[source_key].as_u64().is_none())
                        .map(|(y, _)| *y)
                        .collect();
                    if missing.is_empty() {
                        let total = extra
                            .values()
                            .try_fold(0_u64, |a, r| add(a, count(r, source_key)?))?;
                        sums.insert(key.into(), total);
                        checks.push(json!({"key":key,"rest_seasons":total,"state":"complete_report_windows","missing_seasons":[]}));
                    } else {
                        checks.push(json!({"key":key,"state":"incomplete_coverage","missing_seasons":missing}));
                    }
                }
            }
        }
        let first = *rest.keys().next().unwrap();
        let last = *rest.keys().next_back().unwrap();
        let group = if goalie {
            "Goalies"
        } else {
            old["numerical_evidence"]["group"]
                .as_str()
                .ok_or("missing skater group")?
        };
        let time = if goalie {
            rest.clone()
        } else {
            index(&actor["reports"]["skater-timeonice"], id)?
        };
        if rest.keys().ne(time.keys()) {
            return Err("career TOI season coverage incomplete".into());
        }
        let mut seconds = 0;
        let mut strengths = [0_u64; 3];
        let mut seasons = Vec::new();
        for (year, row) in &rest {
            let t = time[year];
            if count(row, "gamesPlayed")? != count(t, "gamesPlayed")? {
                return Err("career workload GP mismatch".into());
            }
            let total = count(t, "timeOnIce")?;
            seconds = add(seconds, total)?;
            if !goalie {
                let parts = [
                    count(t, "evTimeOnIce")?,
                    count(t, "ppTimeOnIce")?,
                    count(t, "shTimeOnIce")?,
                ];
                if parts.iter().sum::<u64>() != total {
                    return Err("career strength time accounting mismatch".into());
                }
                for (a, b) in strengths.iter_mut().zip(parts) {
                    *a = add(*a, b)?;
                }
            }
            seasons.push(json!({"season":year,"games":row["gamesPlayed"],"seconds":total,"teams":row["teamAbbrevs"],"state":"reconciled"}));
        }
        if goalie {
            let expected = clock_seconds(
                published["timeOnIce"]
                    .as_str()
                    .ok_or("missing published goalie TOI")?,
            )?;
            if expected != seconds {
                return Err("career goalie TOI does not reconcile".into());
            }
            let mut saves = 0;
            for r in rest.values() {
                saves = add(saves, count(r, "saves")?)?;
            }
            let derived = sums["shots_against"]
                .checked_sub(sums["goals_against"])
                .ok_or("career GA exceeds shots")?;
            sums.insert("reported_saves".into(), saves);
            sums.insert("derived_saves".into(), derived);
            checks.push(json!({"key":"saves","rest_seasons":saves,"derived_from_published":derived,"published_save_fraction":published["savePctg"],"state":if saves==derived{"reconciled"}else{"conflicting_sources"},"conflict_seasons":rest.iter().filter(|(_,r)|r["shotsAgainst"].as_u64().zip(r["goalsAgainst"].as_u64()).and_then(|(s,g)|s.checked_sub(g))!=r["saves"].as_u64()).map(|(year,r)|json!({"season":year,"reported_saves":r["saves"],"shots_against":r["shotsAgainst"],"goals_against":r["goalsAgainst"],"published_save_fraction":r["savePct"]})).collect::<Vec<_>>() }));
        }
        let mut dates = Vec::new();
        let mut sources = Vec::new();
        let mut raw_kinds = BTreeMap::<String, Vec<Value>>::new();
        let mut landing_seen = false;
        for cap in captures
            .iter()
            .filter(|c| c["player_id"].as_u64() == Some(id))
        {
            let url = cap["url"].as_str().ok_or("missing career source URL")?;
            let parsed = url::Url::parse(url).map_err(|e| e.to_string())?;
            let kind = cap["kind"].as_str().ok_or("missing capture kind")?;
            if parsed.scheme() != "https" {
                return Err("career source must be HTTPS".into());
            }
            if kind == "landing" {
                landing_seen = true;
                if parsed.host_str() != Some("api-web.nhle.com")
                    || parsed.path() != format!("/v1/player/{id}/landing")
                {
                    return Err("wrong landing source".into());
                }
            } else {
                let route = kind.replace('-', "/");
                let filter = parsed
                    .query_pairs()
                    .find(|(k, _)| k == "cayenneExp")
                    .map(|(_, v)| v.into_owned())
                    .ok_or("missing career source filter")?;
                let filters: Vec<_> = filter.split(" and ").collect();
                if parsed.host_str() != Some("api.nhle.com")
                    || parsed.path() != format!("/stats/rest/en/{route}")
                    || filters.len() != 2
                    || !filters.contains(&format!("playerId={id}").as_str())
                    || !filters.contains(&"gameTypeId=2")
                {
                    return Err("wrong career report actor/window".into());
                }
            }
            let raw = fs::read(cap["path"].as_str().ok_or("missing raw path")?)
                .map_err(|e| e.to_string())?;
            let digest = hash(&raw);
            if cap["sha256"].as_str() != Some(digest.as_str()) {
                return Err("career raw hash mismatch".into());
            }
            let body = decode(&raw)?;
            if kind == "landing" {
                if &body != landing {
                    return Err("landing assembly differs from raw".into());
                }
            } else {
                raw_kinds.entry(kind.into()).or_default().extend(
                    body["data"]
                        .as_array()
                        .ok_or("raw career rows missing")?
                        .clone(),
                );
            }
            let date = chrono::DateTime::parse_from_rfc3339(
                cap["accessed_utc"]
                    .as_str()
                    .ok_or("missing career capture date")?,
            )
            .map_err(|e| e.to_string())?;
            dates.push(date);
            sources.push(json!({"url":url,"sha256":digest,"bytes":raw.len()}));
        }
        if dates.is_empty() {
            return Err("no career capture provenance".into());
        }
        let expected_kinds = if goalie {
            vec!["goalie-summary"]
        } else {
            vec![
                "skater-summary",
                "skater-timeonice",
                "skater-realtime",
                "skater-penalties",
            ]
        };
        if !landing_seen || expected_kinds.iter().any(|k| !raw_kinds.contains_key(*k)) {
            return Err("required raw career surface missing".into());
        }
        dates.sort();
        if dates[0].year() < (last / 10000) as i32 {
            return Err("career capture predates the last included season".into());
        }
        let start = dates[0].to_rfc3339();
        let end = dates.last().unwrap().to_rfc3339();
        for (kind, rows) in &raw_kinds {
            let report = &actor["reports"][kind];
            if report["total"].as_u64() != Some(rows.len() as u64) {
                return Err("raw career pages incomplete".into());
            }
            let assembled = index(report, id)?;
            let mut seen = BTreeMap::new();
            for raw in rows {
                let year = count(raw, "seasonId")? as u32;
                if seen.insert(year, ()).is_some() {
                    return Err("duplicate raw career season".into());
                }
                let a = assembled
                    .get(&year)
                    .ok_or("raw career season absent from assembly")?;
                if a.as_object().map(|v| v.len()) != raw.as_object().map(|v| v.len()) {
                    return Err("career raw/assembled fields differ".into());
                }
                for (k, v) in raw.as_object().ok_or("bad raw row")? {
                    let other = &a[k];
                    let agrees = match (v.as_f64(), other.as_f64()) {
                        (Some(x), Some(y)) => (x - y).abs() <= 1e-12 * x.abs().max(y.abs()).max(1.),
                        _ => v == other,
                    };
                    if !agrees {
                        return Err(format!("career raw/assembled {kind}.{k} mismatch"));
                    }
                }
            }
        }
        let u32n = |n: u64| {
            u32::try_from(n).map_err(|_| "career value exceeds supported integer range".to_owned())
        };
        let mut deployment = rinkcraft_deployment::build(UsageInput {
            player_id: PlayerId(u32n(id)?),
            season: Season(last),
            season_type: SeasonType::Regular,
            games: u32n(sums["games"])?,
            total: u32n(seconds)?,
            skater_strengths: if goalie {
                None
            } else {
                Some([
                    u32n(strengths[0])?,
                    u32n(strengths[1])?,
                    u32n(strengths[2])?,
                ])
            },
            goalie_starts: if goalie {
                Some(u32n(sums["starts"])?)
            } else {
                None
            },
        })?;
        deployment.source_label = "NHL pooled career workload".into();
        deployment.source_sha256 = Some(hash(bytes));
        deployment.notes.push("Exact seasonal seconds pooled across reconciled NHL regular seasons. Hash identifies the supplied career capture bundle; raw page hashes are separate. No rounded average-TOI multiplication.".into());
        let label = format!(
            "NHL career · {}–{:02} to {}–{:02}",
            first / 10000,
            first % 100,
            last / 10000,
            last % 100
        );
        let mut measurements = if goalie {
            vec![
                value("career-starts", "Starts", "Count", sums["starts"], 1, 1.),
                value("career-wins", "Wins", "Count", sums["wins"], 1, 1.),
                value(
                    "career-shutouts",
                    "Shutouts",
                    "Count",
                    sums["shutouts"],
                    1,
                    1.,
                ),
                value(
                    "career-goals-against",
                    "Goals against",
                    "Count",
                    sums["goals_against"],
                    1,
                    1.,
                ),
                value(
                    "career-save-fraction",
                    "Save percentage",
                    "Pct",
                    sums["reported_saves"],
                    sums["shots_against"],
                    1.,
                ),
                value(
                    "career-gaa",
                    "GA / 60",
                    "Per60",
                    sums["goals_against"],
                    seconds,
                    3600.,
                ),
            ]
        } else {
            vec![
                value("career-goals", "Goals", "Count", sums["goals"], 1, 1.),
                value("career-assists", "Assists", "Count", sums["assists"], 1, 1.),
                value("career-points", "Points", "Count", sums["points"], 1, 1.),
                value(
                    "career-points-per-game",
                    "Points / GP",
                    "PerGame",
                    sums["points"],
                    sums["games"],
                    1.,
                ),
                value("career-shots", "Shots", "Count", sums["shots"], 1, 1.),
                value(
                    "career-conversion",
                    "Shooting percentage",
                    "Pct",
                    sums["goals"],
                    sums["shots"],
                    1.,
                ),
            ]
        };
        if goalie && sums["reported_saves"] != sums["derived_saves"] {
            let m = measurements
                .iter_mut()
                .find(|m| m["key"] == "career-save-fraction")
                .unwrap();
            m["value"] = Value::Null;
            m["numerator"] = Value::Null;
            m["state"] = json!("conflicting_sources");
            m["formula"] =
                json!("Withheld: reported saves conflict with shots against minus goals against.");
            m["interpretation"]=json!(format!("Career save percentage withheld: seasonal reported saves {} versus {} from reconciled shots/goals. See career audit; neither value is silently chosen.",sums["reported_saves"],sums["derived_saves"]));
        }
        let numerical = json!({"schema":"icelines.rinkcraft-numerical.v1","scope_kind":"career","scope_label":label,"season":last,"first_season":first,"player_id":id,"name":old["player_name"],"group":group,"situation":"all strengths","games":sums["games"],"minutes":seconds as f64/60.,"eligible_peers":0,"facets":measurements.iter().map(|m|json!({"label":m["label"],"measurement":m,"explanation":null})).collect::<Vec<_>>(),"leading":[],"metrics":measurements,"source_label":"NHL reconciled career records","source_credit":"NHL landing career totals and season ledger reconciled to official stats reports. These are two surfaces of one provider, not independent corroboration.","peer_rules":"No career peer cohort has been validated. All career percentiles and seasonal feat thresholds are withheld. Rates pool counts and opportunities; seasonal percentiles/rates are never averaged.","sources":sources,"career_audit":{"seasons":seasons,"fields":checks,"same_provider":true},"unknowns":["Career-adjusted peer ranks and feats are not released.","Historical dossier does not establish a uniform lifetime role or capability."],"feature_context":[],"style_evidence":null,"context_evidence":null});
        let metrics:Vec<_>=sums.iter().map(|(k,v)|json!({"key":k,"label":k.replace('_'," "),"unit":"Count","value":v,"status":"available","aggregation":"reconciled_career_sum","observed_seasons":rest.len(),"missing_seasons":[],"report_source":"NHL career reconciliation","ratio":null})).collect();
        let position = if goalie {
            "Goalie"
        } else if group == "Defense" {
            "Defense"
        } else {
            "Forward"
        };
        let record = json!({"scope_kind":"career","season":last,"games":sums["games"],"position":position,"teams":[],"traded":false,"metrics":metrics});
        let mut deployment = serde_json::to_value(deployment).map_err(|e| e.to_string())?;
        deployment["scope_label"] = json!(label);
        deployment["scope_kind"] = json!("career");
        let historical = old["profile_window"]["season"]
            .as_u64()
            .ok_or("historical season option missing")?;
        let current = old["league_current_season"]
            .as_u64()
            .ok_or("current context missing")?;
        output.insert(key.into(),json!({"schema":"rinkcraft.career-profile-preview.v1","request":{"player_id":id,"season_type":"regular","first_season":first,"last_season":last},"scope":"nhl_regular_career","included_seasons":rest.keys().collect::<Vec<_>>(),"latest_available":record,"summary":metrics,"timeline":[],"changes":[],"attribute_seasons":[],"numerical_evidence":numerical,"numerical_history":null,"numerical_comparison_choices":[],"numerical_snapshot":{"selected_season":last,"newest_supplied_season":last,"requested_season":null,"state":"career_reconciled","note":"Pooled NHL regular-season career records; no career peer ranks or seasonal feat conversion."},"deployment_evidence":deployment,"achievements":null,"experimental_patterns":{"schema":"rinkcraft.scoped-patterns.v1","player_id":id,"season":last,"season_type":"regular","group":group,"status":"career_reference_unavailable","patterns":[],"scope_note":"Career counts are reconciled; career feat reference is unavailable."},"profile_window":{"player_id":id,"kind":"career","first_season":first,"season":last,"season_type":"regular","as_of":end,"label":label,"coverage":"career_reconciled","choices":[{"kind":"season","label":format!("Season · {}–{:02}",historical/10000,historical%100),"available":true},{"kind":"career","label":label,"available":true},{"kind":"current","label":format!("Current · {}–{:02}",current/10000,current%100),"available":true}],"note":"NHL regular-season career through this capture, including a partial current season. Core career counts reconcile across NHL landing/season ledger/stats summary. No independent game-by-game audit or validated career peer baseline; historical dossier retains its own dates."},"career_reconciliation":{"state":"core_counts_reconciled","seasons":seasons,"fields":checks,"excluded_non_nhl_or_playoff_rows":rows.len()-ledger.len(),"observed_nhl_regular_seasons":rest.len(),"first_season":first,"last_season":last,"counts":sums,"time_seconds":seconds,"sources_same_provider":true},"capture_period":{"from":start,"through":end},"current_season_available":last as u64==current,"source_windows":[],"absent_seasons_in_requested_range":[],"unmeasured_dimensions":["Career peer reference","Lifetime task effectiveness","Uniform career role"],"limitations":["Reconciled modern NHL regular-season records only, not all hockey lifetime.","Historical class/method dossier is not a lifetime capability assessment."]}));
    }
    Ok(Value::Object(output))
}
fn main() {
    let a: Vec<_> = env::args().collect();
    let run = || -> Result<(), String> {
        if a.len() != 4 {
            return Err(
                "Usage: HISTORICAL_FIXTURES CAREER_CAPTURE_INPUT OUTPUT_CAREER_PATCHES".into(),
            );
        }
        let base = decode(&fs::read(&a[1]).map_err(|e| e.to_string())?)?;
        let bytes = fs::read(&a[2]).map_err(|e| e.to_string())?;
        let out = project(&base, &decode(&bytes)?, &bytes)?;
        fs::write(&a[3], serde_json::to_vec(&out).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        println!("Career counts reconciled; pooled rates produced without career ranks or seasonal feats.");
        Ok(())
    };
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pooled_rates_do_not_average_percentages() {
        let v = value("x", "x", "Pct", 110, 1010, 1.);
        assert!((v["value"].as_f64().unwrap() - 110. / 1010.).abs() < 1e-12);
        assert_ne!(v["value"], json!(0.55));
        assert!(v["percentile"].is_null());
    }
    #[test]
    fn long_goalie_minutes_are_not_hours() {
        assert_eq!(clock_seconds("19224:02").unwrap(), 1153442);
        assert!(clock_seconds("1:60").is_err());
    }
    #[test]
    fn duplicate_season_is_not_a_trade_total() {
        assert!(index(&json!({"total":2,"data":[{"playerId":1,"seasonId":20242025},{"playerId":1,"seasonId":20242025}]}),1).is_err());
    }
    #[test]
    fn missing_count_is_not_zero() {
        assert!(count(&json!({"x":null}), "x").is_err());
        assert_eq!(count(&json!({"x":0}), "x").unwrap(), 0);
    }
    #[test]
    fn split_trade_stints_reconcile_but_total_plus_stints_does_not() {
        let a = json!({"season":20242025,"gamesPlayed":10});
        let b = json!({"season":20242025,"gamesPlayed":15});
        let total = json!({"season":20242025,"gamesPlayed":25});
        let report =
            json!({"total":1,"data":[{"playerId":1,"seasonId":20242025,"gamesPlayed":25}]});
        let rest = index(&report, 1).unwrap();
        let published = json!({"gamesPlayed":25});
        assert!(reconcile(
            &[&a, &b],
            &rest,
            &published,
            &[("games", "gamesPlayed", "gamesPlayed")]
        )
        .is_ok());
        assert!(reconcile(
            &[&a, &b, &total],
            &rest,
            &published,
            &[("games", "gamesPlayed", "gamesPlayed")]
        )
        .is_err());
    }
}
