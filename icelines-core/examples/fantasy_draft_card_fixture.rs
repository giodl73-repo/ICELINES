use std::fs;
use std::path::PathBuf;

use chrono::{TimeZone, Utc};
use icelines_core::season_stats::SeasonType;
use icelines_core::{
    build_fantasy_draft_board, build_fantasy_draft_card, import_fantasy_taken_players,
    FantasyActiveSlot, FantasyActiveSlotKind, FantasyDraftCandidateInput, FantasyDraftCardInput,
    FantasyDraftIdentityInput, Position, Season, ViewContext, ViewWindow,
};

fn candidate(
    key: &str,
    player: &str,
    team: &str,
    positions: Vec<Position>,
    quality: f64,
    replacement: f64,
    starts: f64,
    quiet: f64,
    collision: f64,
    playoff_starts: f64,
    playoff_value: f64,
    risk: f64,
) -> FantasyDraftCandidateInput {
    FantasyDraftCandidateInput {
        player_key: key.to_string(),
        player: player.to_string(),
        nhl_team: team.to_string(),
        platform_positions: positions,
        league_scored_quality: quality,
        replacement_level: replacement,
        incremental_usable_starts: starts,
        quiet_slate_games: quiet,
        schedule_collision_rate: collision,
        playoff_incremental_usable_starts: playoff_starts,
        playoff_usable_value_delta: playoff_value,
        risk_penalty: risk,
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("examples/fantasy-draft-card-dexters-dawgs-pick-7.json"));
    let candidates = vec![
        candidate(
            "connor-mcdavid",
            "Connor McDavid",
            "EDM",
            vec![Position::Center],
            96.0,
            51.0,
            22.0,
            8.0,
            0.41,
            3.0,
            9.0,
            1.0,
        ),
        candidate(
            "nathan-mackinnon",
            "Nathan MacKinnon",
            "COL",
            vec![Position::Center],
            92.0,
            51.0,
            20.0,
            7.0,
            0.46,
            2.0,
            8.0,
            1.0,
        ),
        candidate(
            "jason-robertson",
            "Jason Robertson",
            "DAL",
            vec![Position::LeftWing],
            78.0,
            49.0,
            24.0,
            10.0,
            0.31,
            3.0,
            8.0,
            1.0,
        ),
        candidate(
            "william-nylander",
            "William Nylander",
            "TOR",
            vec![Position::LeftWing, Position::RightWing],
            75.0,
            50.0,
            25.0,
            9.0,
            0.27,
            4.0,
            7.0,
            1.5,
        ),
        candidate(
            "cale-makar",
            "Cale Makar",
            "COL",
            vec![Position::Defense],
            81.0,
            54.0,
            19.0,
            6.0,
            0.46,
            2.0,
            9.0,
            2.0,
        ),
        candidate(
            "igor-shesterkin",
            "Igor Shesterkin",
            "NYR",
            vec![Position::Goalie],
            73.0,
            52.0,
            17.0,
            7.0,
            0.49,
            1.0,
            6.0,
            2.5,
        ),
        candidate(
            "macklin-celebrini",
            "Macklin Celebrini",
            "SJS",
            vec![Position::Center],
            76.0,
            51.0,
            23.0,
            11.0,
            0.25,
            4.0,
            8.0,
            2.0,
        ),
        candidate(
            "quinn-hughes",
            "Quinn Hughes",
            "VAN",
            vec![Position::Defense],
            77.0,
            54.0,
            21.0,
            8.0,
            0.38,
            3.0,
            7.0,
            1.5,
        ),
    ];
    let identities = candidates
        .iter()
        .map(|candidate| FantasyDraftIdentityInput {
            player_key: candidate.player_key.clone(),
            display_name: candidate.player.clone(),
            aliases: Vec::new(),
        })
        .collect::<Vec<_>>();
    let taken = import_fantasy_taken_players("Connor McDavid\nNathan MacKinnon", &identities)
        .map_err(std::io::Error::other)?;
    let mut board = build_fantasy_draft_board(
        "dexters-dawgs",
        "20252026",
        vec![
            FantasyActiveSlot {
                slot_id: "LW1".to_string(),
                kind: FantasyActiveSlotKind::LeftWing,
            },
            FantasyActiveSlot {
                slot_id: "RW1".to_string(),
                kind: FantasyActiveSlotKind::RightWing,
            },
            FantasyActiveSlot {
                slot_id: "D1".to_string(),
                kind: FantasyActiveSlotKind::Defense,
            },
            FantasyActiveSlot {
                slot_id: "G1".to_string(),
                kind: FantasyActiveSlotKind::Goalie,
            },
        ],
        candidates,
        taken,
        10,
    )
    .map_err(std::io::Error::other)?;
    board.warnings.push("Showcase candidate values, platform eligibility, injuries, and taken-player state are deterministic fixture inputs, not current claims.".to_string());
    let timestamp = Utc.with_ymd_and_hms(2026, 9, 30, 19, 0, 0).unwrap();
    let mut view = ViewContext::new(ViewWindow::new(Season(20262027), SeasonType::Regular));
    view.generated_at = Some(timestamp);
    let card = build_fantasy_draft_card(FantasyDraftCardInput {
        league_id: "dexters-dawgs-league".to_string(),
        league_name: "Dexter's 2026-27 League".to_string(),
        fantasy_team_id: "dexters-dawgs".to_string(),
        fantasy_team_name: "Dexter's Dawgs".to_string(),
        roster_snapshot_id: Some("draft-fixture-pick-7".to_string()),
        calendar_fingerprint: Some("deterministic-draft-calendar-v1".to_string()),
        board,
        view,
        evidence_at: Some(timestamp),
    })?;
    fs::write(&output, serde_json::to_string_pretty(&card)? + "\n")?;
    println!("{}\n{}", output.display(), card.fingerprint);
    Ok(())
}
