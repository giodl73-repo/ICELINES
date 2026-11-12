//! Official HTML shift-table adapter, mapped by team-side and sweater number.
//! Derived intervals retain their source kind; no tactical assignment is inferred.
use regex::Regex;
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
fn seconds(s: &str) -> Result<u32> {
    let (m, s) = s.trim().split_once(':').ok_or("Invalid report clock")?;
    let m: u32 = m.parse()?;
    let s: u32 = s.parse()?;
    if m > 120 || s >= 60 {
        return Err("Invalid report clock bound".into());
    }
    Ok(m * 60 + s)
}
fn period(s: &str) -> Result<u32> {
    let s = s.trim();
    if s == "OT" {
        return Ok(4);
    }
    if let Some(n) = s.strip_suffix("OT") {
        return Ok(3 + n.parse::<u32>()?);
    }
    Ok(s.parse()?)
}
fn text(s: &str, tags: &Regex) -> String {
    tags.replace_all(s, " ")
        .replace("&nbsp;", " ")
        .replace("&#160;", " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Converts both reports into canonical type-517 rows after roster and TOI checks.
pub fn parse_reports(boxscore: &Value, home_html: &str, away_html: &str) -> Result<Value> {
    let game = boxscore["id"].as_u64().ok_or("Missing game ID")?;
    let rows_re = Regex::new(r"(?is)<tr\b[^>]*>(.*?)</tr>")?;
    let cells_re = Regex::new(r"(?is)<td\b[^>]*>(.*?)</td>")?;
    let heading_re =
        Regex::new(r#"(?is)class\s*=\s*["'][^"']*\bplayerHeading\b[^"']*["'][^>]*>([^<]*)<"#)?;
    let tags = Regex::new(r"(?s)<[^>]*>")?;
    let mut records = Vec::new();
    let mut checks = Vec::new();
    for (side, html) in [("homeTeam", home_html), ("awayTeam", away_html)] {
        let tid = boxscore[side]["id"].as_u64().ok_or("Missing team ID")?;
        let plain = text(html, &tags);
        let game_label = Regex::new(&format!(r"\bGame\s+0*{}\b", game % 10000))?;
        if !game_label.is_match(&plain) {
            return Err("HTML report game label mismatch".into());
        }
        let mut roster = BTreeMap::<u64, &Value>::new();
        for group in ["forwards", "defense", "goalies"] {
            for p in boxscore["playerByGameStats"][side][group]
                .as_array()
                .ok_or("Missing roster")?
            {
                let sweater = p["sweaterNumber"].as_u64().ok_or("Missing sweater")?;
                if roster.insert(sweater, p).is_some() {
                    return Err("Duplicate roster sweater".into());
                }
            }
        }
        let mut current = None;
        let mut unique = BTreeSet::<(u64, u32, u32, u32)>::new();
        let mut totals = BTreeMap::<u64, u32>::new();
        for row in rows_re.captures_iter(html) {
            let body = &row[1];
            if let Some(h) = heading_re.captures(body) {
                let heading = text(&h[1], &tags);
                let n: u64 = heading
                    .split_whitespace()
                    .next()
                    .ok_or("Empty player heading")?
                    .parse()?;
                if !roster.contains_key(&n) {
                    return Err(format!("HTML sweater {n} absent from box roster").into());
                }
                current = Some(n);
                continue;
            }
            let cells: Vec<_> = cells_re
                .captures_iter(body)
                .map(|c| text(&c[1], &tags))
                .collect();
            if cells.len() < 5
                || !cells[2].contains('/')
                || !cells[3].contains('/')
                || cells[0].parse::<u32>().is_err()
            {
                continue;
            }
            let sweater = current.ok_or("Shift before player heading")?;
            let p = roster[&sweater];
            let pid = p["playerId"].as_u64().ok_or("Missing player ID")?;
            let per = period(&cells[1])?;
            let (start, remaining_start) = cells[2].split_once('/').unwrap();
            let (end, remaining_end) = cells[3].split_once('/').unwrap();
            let start = seconds(start)?;
            let end = seconds(end)?;
            let duration = seconds(&cells[4])?;
            let length = if per > 3 && boxscore["gameType"].as_u64() == Some(2) {
                300
            } else {
                1200
            };
            if end < start
                || end - start != duration
                || start + seconds(remaining_start)? != length
                || end + seconds(remaining_end)? != length
            {
                return Err("HTML elapsed/remaining/duration mismatch".into());
            }
            if unique.insert((pid, per, start, end)) {
                *totals.entry(pid).or_default() += duration;
            }
            records.push(json!({"gameId":game,"teamId":tid,"playerId":pid,"period":per,"typeCode":517,"startTime":format!("{:02}:{:02}",start/60,start%60),"endTime":format!("{:02}:{:02}",end/60,end%60)}));
        }
        for p in roster.values() {
            let pid = p["playerId"].as_u64().ok_or("Missing player ID")?;
            let expected = seconds(p["toi"].as_str().ok_or("Missing box TOI")?)?;
            let actual = totals.get(&pid).copied().unwrap_or(0);
            if actual.abs_diff(expected) > 1 {
                return Err(
                    format!("HTML/box TOI mismatch for {pid}: {actual} vs {expected}").into(),
                );
            }
            checks.push(json!({"player_id":pid,"box_seconds":expected,"report_seconds":actual}));
        }
    }
    if records.is_empty() {
        return Err("No HTML shift rows parsed".into());
    }
    Ok(
        json!({"adapter":"icelines.official-html-shifts.0.1-experimental","source_kind":"derived_from_official_home_and_away_HTML_reports","game_id":game,"total":records.len(),"data":records,"box_toi_validation":checks,"toi_tolerance_seconds":1}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (Value, String, String) {
        let b = json!({"id":2000020001u64,"gameType":2,"homeTeam":{"id":1},"awayTeam":{"id":2},"playerByGameStats":{"homeTeam":{"forwards":[{"playerId":1,"sweaterNumber":10,"toi":"01:00"}],"defense":[],"goalies":[]},"awayTeam":{"forwards":[{"playerId":2,"sweaterNumber":11,"toi":"01:00"}],"defense":[],"goalies":[]}}});
        let html = |n| {
            format!("Game 0001<table><tr><td class=\"playerHeading + border\">{n} NAME</td></tr><tr><td>1</td><td>1</td><td>0:00 / 20:00</td><td>1:00 / 19:00</td><td>01:00</td><td></td></tr></table>")
        };
        (b, html(10), html(11))
    }
    #[test]
    fn maps_actual_side_sweaters_and_checks_duration() {
        let (b, h, a) = fixture();
        let r = parse_reports(&b, &h, &a).unwrap();
        assert_eq!(r["total"], 2);
        assert_eq!(r["data"][0]["playerId"], 1);
        assert_eq!(r["data"][1]["teamId"], 2);
    }
    #[test]
    fn wrong_sweater_or_toi_rejected() {
        let (mut b, h, a) = fixture();
        assert!(parse_reports(&b, &h.replace("10 NAME", "99 NAME"), &a).is_err());
        b["playerByGameStats"]["homeTeam"]["forwards"][0]["toi"] = json!("01:10");
        assert!(parse_reports(&b, &h, &a).is_err());
    }
    #[test]
    fn contradictory_clock_rejected() {
        let (b, h, a) = fixture();
        assert!(parse_reports(&b, &h.replace("1:00 / 19:00", "1:00 / 18:59"), &a).is_err());
    }
}
