use std::{env, fs, process};

use icelines_core::CardDocumentView;
use sha2::{Digest, Sha256};

fn main() {
    let Some(path) = env::args().nth(1) else {
        eprintln!("usage: reseal_card_document <card-document.json>");
        process::exit(2);
    };
    let raw = fs::read_to_string(&path).unwrap_or_else(|error| {
        eprintln!("read {path}: {error}");
        process::exit(1);
    });
    let mut value: serde_json::Value = serde_json::from_str(&raw).unwrap_or_else(|error| {
        eprintln!("parse {path}: {error}");
        process::exit(1);
    });
    if value.get("schema").and_then(serde_json::Value::as_str)
        == Some("player_line_matchup_forecast.v1")
    {
        value["fingerprint"] = serde_json::Value::String(String::new());
        let bytes = serde_json::to_vec(&value).expect("serialize matchup fingerprint material");
        value["fingerprint"] =
            serde_json::Value::String(format!("sha256:{:x}", Sha256::digest(bytes)));
        let mut output = serde_json::to_string_pretty(&value).expect("serialize matchup forecast");
        output.push('\n');
        fs::write(&path, output).unwrap_or_else(|error| {
            eprintln!("write {path}: {error}");
            process::exit(1);
        });
        return;
    }
    let mut card: CardDocumentView = serde_json::from_value(value).unwrap_or_else(|error| {
        eprintln!("parse card {path}: {error}");
        process::exit(1);
    });
    card.refresh_fingerprint().unwrap_or_else(|error| {
        eprintln!("fingerprint {path}: {error}");
        process::exit(1);
    });
    card.validate().unwrap_or_else(|error| {
        eprintln!("validate {path}: {error}");
        process::exit(1);
    });
    let mut output = serde_json::to_string_pretty(&card).expect("serialize card document");
    output.push('\n');
    fs::write(&path, output).unwrap_or_else(|error| {
        eprintln!("write {path}: {error}");
        process::exit(1);
    });
}
