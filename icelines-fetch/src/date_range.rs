//! Bounded, paginated official NHL date-range captures shared by CLI and web.
use crate::nhl_api::NhlApiClient;
use anyhow::{anyhow, bail, ensure, Context, Result};
use chrono::{Datelike, NaiveDate, Utc};
use icelines_core::date_range::{scoring_rows, GameScoring};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

pub const METHOD: &str = "Inclusive NHL game dates; only games played for the selected team. Regular season and playoffs remain separate. EV points are official NHL evPoints (all even-strength situations, including 3-on-3 and 4-on-4), not 5-on-5 only. Competition ranks share ties and are recalculated among selected players. P/G and EVP/G use each player's games in this window. Roster filters use the separately dated official current-roster capture; players outside it may have departed or been assigned elsewhere.";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DateRangeQuery {
    pub team: String,
    pub date_from: NaiveDate,
    pub date_to: NaiveDate,
    pub game_type: u8,
}
impl DateRangeQuery {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.team.len() == 3 && self.team.bytes().all(|c| c.is_ascii_uppercase()),
            "team must be a three-letter NHL abbreviation"
        );
        ensure!(
            self.date_from <= self.date_to,
            "start date must be on or before end date"
        );
        ensure!(
            (self.date_to - self.date_from).num_days() <= 1096,
            "date range is limited to three years per capture"
        );
        ensure!(
            self.date_to < Utc::now().date_naive(),
            "end date must be a completed NHL game date before today (UTC)"
        );
        ensure!(
            self.date_from.year() >= 1917,
            "NHL game dates begin in 1917"
        );
        ensure!(
            matches!(self.game_type, 2 | 3),
            "game type must be regular or playoff"
        );
        Ok(())
    }
    pub fn path(&self, root: &Path) -> PathBuf {
        root.join("date-ranges").join(format!(
            "{}-{}-{}-{}.json",
            self.team, self.date_from, self.date_to, self.game_type
        ))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapturedSource {
    pub url: String,
    pub sha256: String,
    /// Exact captured bytes as UTF-8, preserving reproducibility without a live API.
    pub body: String,
}

/// Membership at capture time, independently dated from the historical query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentRosterCapture {
    pub schema_version: u32,
    pub team: String,
    pub fetched_at: String,
    pub player_ids: BTreeSet<u32>,
    pub source: CapturedSource,
}
fn validate_team(team: &str) -> Result<()> {
    ensure!(
        team.len() == 3 && team.bytes().all(|c| c.is_ascii_uppercase()),
        "team must be a three-letter NHL abbreviation"
    );
    Ok(())
}
pub fn current_roster_path(root: &Path, team: &str) -> Result<PathBuf> {
    validate_team(team)?;
    Ok(root.join("current-rosters").join(format!("{team}.json")))
}
fn roster_ids(body: &str) -> Result<BTreeSet<u32>> {
    let roster: serde_json::Value = serde_json::from_str(body)?;
    let mut ids = BTreeSet::new();
    for group in ["forwards", "defensemen", "goalies"] {
        for player in roster[group]
            .as_array()
            .with_context(|| format!("current roster is missing {group}"))?
        {
            let id = u32::try_from(
                player["id"]
                    .as_u64()
                    .context("roster player is missing an ID")?,
            )?;
            ensure!(ids.insert(id), "duplicate player in current roster");
        }
    }
    ensure!(!ids.is_empty(), "current roster is empty");
    Ok(ids)
}
impl CurrentRosterCapture {
    pub fn validate(&self, team: &str) -> Result<()> {
        validate_team(team)?;
        ensure!(
            self.schema_version == 1 && self.team == team,
            "current-roster capture context mismatch"
        );
        ensure!(
            self.source.url == format!("https://api-web.nhle.com/v1/roster/{team}/current"),
            "current-roster source mismatch"
        );
        ensure!(
            self.source.sha256 == format!("{:x}", Sha256::digest(self.source.body.as_bytes())),
            "current-roster source digest mismatch"
        );
        ensure!(
            self.player_ids == roster_ids(&self.source.body)?,
            "current-roster membership does not match source"
        );
        Ok(())
    }
}
pub fn load_current_roster(root: &Path, team: &str) -> Result<CurrentRosterCapture> {
    let bytes = std::fs::read(current_roster_path(root, team)?).context(
        "current-roster evidence unavailable; use Refresh data & run or --refresh to capture it",
    )?;
    let capture: CurrentRosterCapture = serde_json::from_slice(&bytes)?;
    capture.validate(team)?;
    Ok(capture)
}
pub async fn refresh_current_roster(root: &Path, team: &str) -> Result<CurrentRosterCapture> {
    let path = current_roster_path(root, team)?;
    let lock_root = root.parent().unwrap_or(root).to_owned();
    let _guard = tokio::task::spawn_blocking(move || {
        crate::fetch_lock::acquire(&lock_root, std::time::Duration::from_secs(1))
    })
    .await??;
    let client = NhlApiClient::production();
    let mut sources = Vec::new();
    tokio::time::timeout(
        std::time::Duration::from_secs(90),
        source(
            &client,
            format!("https://api-web.nhle.com/v1/roster/{team}/current"),
            &mut sources,
        ),
    )
    .await
    .context("current-roster refresh exceeded its 90-second deadline")??;
    let source = sources.pop().context("current-roster source missing")?;
    let capture = CurrentRosterCapture {
        schema_version: 1,
        team: team.into(),
        fetched_at: Utc::now().to_rfc3339(),
        player_ids: roster_ids(&source.body)?,
        source,
    };
    capture.validate(team)?;
    crate::atomic_write::write_json_atomic(&path, &capture)?;
    Ok(capture)
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DateRangeCapture {
    pub schema_version: u32,
    pub query: DateRangeQuery,
    pub fetched_at: String,
    pub game_ids: BTreeSet<u64>,
    pub games: Vec<GameScoring>,
    pub sources: Vec<CapturedSource>,
}
impl DateRangeCapture {
    pub fn validate(&self, query: &DateRangeQuery) -> Result<()> {
        ensure!(
            self.schema_version == 1 && self.query == *query,
            "capture schema or query mismatch"
        );
        scoring_rows(
            &self.games,
            &query.team,
            query.date_from,
            query.date_to,
            query.game_type,
            "points",
        )
        .map_err(|e| anyhow!(e))?;
        let present: BTreeSet<_> = self.games.iter().map(|g| g.game_id).collect();
        ensure!(
            present == self.game_ids,
            "NHL scoring coverage does not match the completed schedule"
        );
        ensure!(!self.sources.is_empty(), "capture has no source evidence");
        for s in &self.sources {
            ensure!(
                format!("{:x}", Sha256::digest(s.body.as_bytes())) == s.sha256,
                "source digest mismatch"
            );
        }
        Ok(())
    }
}
pub fn load(root: &Path, query: &DateRangeQuery) -> Result<DateRangeCapture> {
    query.validate()?;
    let bytes = std::fs::read(query.path(root)).context(
        "date range is not cached; use Refresh data or icelines query date-range --refresh",
    )?;
    let capture: DateRangeCapture = serde_json::from_slice(&bytes)?;
    capture.validate(query)?;
    Ok(capture)
}
fn encode(s: &str) -> String {
    s.bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || b"-._~".contains(&b) {
                char::from(b).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect()
}
async fn source(
    client: &NhlApiClient,
    url: String,
    sources: &mut Vec<CapturedSource>,
) -> Result<serde_json::Value> {
    let bytes = client.fetch_source_bytes(&url).await?;
    let value = serde_json::from_slice(&bytes)?;
    sources.push(CapturedSource {
        url,
        sha256: format!("{:x}", Sha256::digest(&bytes)),
        body: String::from_utf8(bytes)?,
    });
    Ok(value)
}
fn season_year(date: NaiveDate) -> i32 {
    date.year() - i32::from(date.month() < 7)
}

pub async fn refresh(root: &Path, query: &DateRangeQuery) -> Result<DateRangeCapture> {
    query.validate()?;
    let lock_root = root.parent().unwrap_or(root).to_owned();
    let _guard = tokio::task::spawn_blocking(move || {
        crate::fetch_lock::acquire(&lock_root, std::time::Duration::from_secs(1))
    })
    .await??;
    let capture = tokio::time::timeout(std::time::Duration::from_secs(180), acquire(query))
        .await
        .context("date-range capture exceeded its three-minute deadline")??;
    // Only a fully validated capture can replace a previous successful one.
    capture.validate(query)?;
    let path = query.path(root);
    std::fs::create_dir_all(path.parent().unwrap())?;
    let temp = path.with_extension(format!("{}.tmp", std::process::id()));
    std::fs::write(&temp, serde_json::to_vec_pretty(&capture)?)?;
    std::fs::rename(temp, path)?;
    Ok(capture)
}
async fn acquire(query: &DateRangeQuery) -> Result<DateRangeCapture> {
    let client = NhlApiClient::production();
    acquire_from(
        &client,
        query,
        "https://api.nhle.com/stats/rest/en",
        "https://api-web.nhle.com/v1",
    )
    .await
}
async fn acquire_from(
    client: &NhlApiClient,
    query: &DateRangeQuery,
    stats_base: &str,
    web_base: &str,
) -> Result<DateRangeCapture> {
    let mut sources = Vec::new();
    let mut expected = BTreeSet::new();
    let mut team_id = None;
    for year in season_year(query.date_from)..=season_year(query.date_to) {
        let schedule = source(
            client,
            format!(
                "{web_base}/club-schedule-season/{}/{}{}",
                query.team,
                year,
                year + 1
            ),
            &mut sources,
        )
        .await?;
        let games = schedule["games"]
            .as_array()
            .context("missing schedule games")?;
        for g in games {
            for side in ["homeTeam", "awayTeam"] {
                if g[side]["abbrev"].as_str() == Some(&query.team) {
                    let id = g[side]["id"].as_u64().context("missing team ID")?;
                    ensure!(
                        team_id.is_none() || team_id == Some(id),
                        "team ID changed across seasons; query each identity separately"
                    );
                    team_id = Some(id);
                }
            }
            let date: NaiveDate = g["gameDate"]
                .as_str()
                .context("missing game date")?
                .parse()?;
            if date >= query.date_from
                && date <= query.date_to
                && g["gameType"].as_u64() == Some(u64::from(query.game_type))
            {
                ensure!(
                    matches!(g["gameState"].as_str(), Some("OFF" | "FINAL")),
                    "game {} on {date} is not final; retry when official stats are complete",
                    g["id"]
                );
                expected.insert(g["id"].as_u64().context("missing game ID")?);
            }
        }
    }
    let id = team_id.context("team not found in official season schedules")?;
    let expression = format!(
        "gameTypeId={} and teamId={} and gameDate>=\"{}\" and gameDate<=\"{}\"",
        query.game_type, id, query.date_from, query.date_to
    );
    let sort = encode(
        r#"[{"property":"gameId","direction":"ASC"},{"property":"playerId","direction":"ASC"}]"#,
    );
    let mut games = Vec::new();
    let mut total = None;
    loop {
        ensure!(games.len() < 100_000, "date-range row limit exceeded");
        let url = format!("{stats_base}/skater/summary?isAggregate=false&isGame=true&start={}&limit=100&sort={sort}&cayenneExp={}",games.len(),encode(&expression));
        let page = source(client, url, &mut sources).await?;
        let count = page["total"]
            .as_u64()
            .context("missing NHL pagination total")? as usize;
        ensure!(
            total.is_none() || total == Some(count),
            "NHL total changed during pagination; retry capture"
        );
        total = Some(count);
        let rows: Vec<GameScoring> = serde_json::from_value(page["data"].clone())?;
        if rows.is_empty() && games.len() != count {
            bail!("NHL returned an incomplete page");
        }
        games.extend(rows);
        ensure!(
            games.len() <= count,
            "NHL returned more records than reported"
        );
        if games.len() == count {
            break;
        }
    }
    let capture = DateRangeCapture {
        schema_version: 1,
        query: query.clone(),
        fetched_at: Utc::now().to_rfc3339(),
        game_ids: expected,
        games,
        sources,
    };
    capture.validate(query)?;
    Ok(capture)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn date_range_current_roster_replay_rejects_incomplete_and_tampered_membership() {
        let body =
            serde_json::json!({"forwards":[{"id":1}],"defensemen":[{"id":2}],"goalies":[{"id":3}]})
                .to_string();
        let mut c = CurrentRosterCapture {
            schema_version: 1,
            team: "AAA".into(),
            fetched_at: "2026-11-12T15:00:00Z".into(),
            player_ids: BTreeSet::from([1, 2, 3]),
            source: CapturedSource {
                url: "https://api-web.nhle.com/v1/roster/AAA/current".into(),
                sha256: format!("{:x}", Sha256::digest(body.as_bytes())),
                body,
            },
        };
        let root = tempfile::tempdir().unwrap();
        crate::atomic_write::write_json_atomic(
            &current_roster_path(root.path(), "AAA").unwrap(),
            &c,
        )
        .unwrap();
        assert_eq!(
            load_current_roster(root.path(), "AAA")
                .unwrap()
                .player_ids
                .len(),
            3
        );
        c.player_ids.remove(&1);
        assert!(c.validate("AAA").is_err());
        assert!(roster_ids(r#"{"forwards":[{"id":1}]}"#).is_err());
        assert!(current_roster_path(root.path(), "../").is_err());
    }
    #[tokio::test]
    async fn date_range_paginates_reconciles_and_replays_capture() {
        use httpmock::{Method::GET, MockServer};
        use serde_json::json;
        let server = MockServer::start_async().await;
        let q = DateRangeQuery {
            team: "AAA".into(),
            date_from: "2026-10-01".parse().unwrap(),
            date_to: "2026-11-01".parse().unwrap(),
            game_type: 2,
        };
        let schedule = server.mock_async(|when,then| {
            when.method(GET).path("/club-schedule-season/AAA/20262027");
            then.json_body(json!({"games":[{"id":2026020001_u64,"gameDate":"2026-10-05","gameType":2,"gameState":"OFF","homeTeam":{"abbrev":"AAA","id":9001},"awayTeam":{"abbrev":"BBB","id":9002}}]}));
        }).await;
        let row = |id: u32| json!({"playerId":id,"skaterFullName":format!("Player {id}"),"positionCode":"C","teamAbbrev":"AAA","gameId":2026020001_u64,"gameDate":"2026-10-05","gamesPlayed":1,"goals":1,"assists":1,"points":2,"evPoints":1,"ppPoints":1,"shPoints":0});
        let first = server
            .mock_async(|when, then| {
                when.method(GET)
                    .path("/skater/summary")
                    .query_param("start", "0")
                    .query_param("isGame", "true");
                then.json_body(json!({"total":2,"data":[row(1)]}));
            })
            .await;
        let second = server
            .mock_async(|when, then| {
                when.method(GET)
                    .path("/skater/summary")
                    .query_param("start", "1");
                then.json_body(json!({"total":2,"data":[row(2)]}));
            })
            .await;
        let c = acquire_from(
            &NhlApiClient::new(server.base_url(), server.base_url()),
            &q,
            &server.base_url(),
            &server.base_url(),
        )
        .await
        .unwrap();
        assert_eq!(c.games.len(), 2);
        assert_eq!(c.sources.len(), 3);
        assert_eq!(c.game_ids.len(), 1);
        schedule.assert_async().await;
        first.assert_async().await;
        second.assert_async().await;
        let dir = tempfile::tempdir().unwrap();
        crate::atomic_write::write_json_atomic(&q.path(dir.path()), &c).unwrap();
        assert_eq!(load(dir.path(), &q).unwrap().games.len(), 2);
        let mut bad = c.clone();
        bad.game_ids.insert(2026020002);
        assert!(bad.validate(&q).is_err());
        let mut bad = c;
        bad.sources[0].body.push(' ');
        assert!(bad.validate(&q).is_err());
    }
    #[test]
    fn date_range_query_validation_and_season_boundary() {
        let mut q = DateRangeQuery {
            team: "BOS".into(),
            date_from: "2026-02-28".parse().unwrap(),
            date_to: "2026-11-11".parse().unwrap(),
            game_type: 2,
        };
        assert!(q.validate().is_ok());
        assert_eq!(season_year(q.date_from), 2025);
        assert_eq!(season_year(q.date_to), 2026);
        q.team = "../".into();
        assert!(q.validate().is_err());
        q.team = "BOS".into();
        q.date_from = q.date_to.succ_opt().unwrap();
        assert!(q.validate().is_err());
    }
}
