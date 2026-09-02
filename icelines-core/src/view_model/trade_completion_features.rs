use chrono::{DateTime, NaiveDate};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use thiserror::Error;

use super::trade_market::{
    TradeAvailabilityKind, TradeNegotiationPackageView, TradeNegotiationTier, TradeScoutView,
    TRADE_SCOUT_SCHEMA,
};

pub const TRADE_COMPLETION_FEATURE_SET_SCHEMA: &str = "trade_completion_feature_set.v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TradeCompletionFeatureRowView {
    pub proposal_id: String,
    pub issued_at: String,
    pub buyer: String,
    pub seller: String,
    pub target_id: String,
    pub target_label: String,
    pub negotiation_tier: TradeNegotiationTier,
    pub availability_kind: TradeAvailabilityKind,
    pub availability_probability: f64,
    pub availability_evidence_as_of: Option<String>,
    pub destination_allowed: Option<bool>,
    pub hockey_fit_score: f64,
    pub target_value_ratio: f64,
    pub fairness_score: f64,
    pub feasibility_probability: f64,
    pub buyer_utility_delta: f64,
    pub seller_utility_delta: f64,
    pub buyer_season_points_delta: f64,
    pub seller_season_points_delta: f64,
    pub assets_to_seller: usize,
    pub includes_draft_pick: bool,
    pub cap_compliant: Option<bool>,
    pub roster_compliant: Option<bool>,
    pub retention_compliant: Option<bool>,
    pub contract_authority_complete: bool,
    pub pick_ownership_confirmed: Option<bool>,
    pub transaction_ready: bool,
    pub mutually_beneficial: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TradeCompletionFeatureSetView {
    pub schema: String,
    pub as_of: String,
    pub source_schema: String,
    pub buyer: String,
    pub candidates: usize,
    pub rows: Vec<TradeCompletionFeatureRowView>,
    pub disclosures: Vec<String>,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum TradeCompletionFeatureError {
    #[error("invalid trade-completion feature input: {0}")]
    InvalidInput(String),
}

pub fn build_trade_completion_feature_set(
    scout: &TradeScoutView,
) -> Result<TradeCompletionFeatureSetView, TradeCompletionFeatureError> {
    if scout.schema != TRADE_SCOUT_SCHEMA
        || scout.buyer.trim().is_empty()
        || !valid_date_or_timestamp(&scout.as_of)
    {
        return Err(TradeCompletionFeatureError::InvalidInput(
            "source must be trade_scout.v1 with a buyer and valid as_of".to_owned(),
        ));
    }
    let mut candidate_ids = BTreeSet::new();
    let mut rows = Vec::with_capacity(scout.candidates.len() * 3);
    for candidate in &scout.candidates {
        if candidate.target_id.trim().is_empty()
            || candidate.seller.trim().is_empty()
            || !candidate_ids.insert(candidate.target_id.as_str())
            || !candidate.availability.probability.is_finite()
            || !(0.0..=1.0).contains(&candidate.availability.probability)
            || candidate
                .availability
                .observed_at
                .as_deref()
                .is_some_and(|value| !valid_date_or_timestamp(value))
        {
            return Err(TradeCompletionFeatureError::InvalidInput(format!(
                "candidate {} requires unique IDs, seller, valid availability, and dated evidence",
                candidate.target_id
            )));
        }
        for package in [
            &candidate.negotiation.opening_offer,
            &candidate.negotiation.fair_midpoint,
            &candidate.negotiation.maximum_acceptable,
        ] {
            validate_package(scout, candidate.seller.as_str(), package)?;
            let evaluation = &package.evaluation;
            rows.push(TradeCompletionFeatureRowView {
                proposal_id: format!(
                    "{}:{}:{}:{}",
                    scout.buyer,
                    candidate.seller,
                    candidate.target_id,
                    tier_label(package.tier)
                ),
                issued_at: scout.as_of.clone(),
                buyer: scout.buyer.clone(),
                seller: candidate.seller.clone(),
                target_id: candidate.target_id.clone(),
                target_label: candidate.label.clone(),
                negotiation_tier: package.tier,
                availability_kind: candidate.availability.kind,
                availability_probability: candidate.availability.probability,
                availability_evidence_as_of: candidate.availability.observed_at.clone(),
                destination_allowed: candidate.availability.destination_allowed,
                hockey_fit_score: candidate.hockey_fit_score,
                target_value_ratio: package.target_value_ratio,
                fairness_score: evaluation.fairness_score,
                feasibility_probability: evaluation.feasibility_probability,
                buyer_utility_delta: evaluation.buyer_utility_delta,
                seller_utility_delta: evaluation.seller_utility_delta,
                buyer_season_points_delta: evaluation.buyer_season_points_delta,
                seller_season_points_delta: evaluation.seller_season_points_delta,
                assets_to_seller: package.assets_to_seller.len(),
                includes_draft_pick: !evaluation.pick_roles.is_empty(),
                cap_compliant: evaluation.transaction_gates.cap_compliant,
                roster_compliant: evaluation.transaction_gates.roster_compliant,
                retention_compliant: evaluation.transaction_gates.retention_compliant,
                contract_authority_complete: evaluation
                    .transaction_gates
                    .contract_authority_complete,
                pick_ownership_confirmed: evaluation.transaction_gates.pick_ownership_confirmed,
                transaction_ready: evaluation.transaction_ready,
                mutually_beneficial: evaluation.mutually_beneficial,
            });
        }
    }
    Ok(TradeCompletionFeatureSetView {
        schema: TRADE_COMPLETION_FEATURE_SET_SCHEMA.to_owned(),
        as_of: scout.as_of.clone(),
        source_schema: scout.schema.clone(),
        buyer: scout.buyer.clone(),
        candidates: scout.candidates.len(),
        rows,
        disclosures: vec![
            "Every row is a point-in-time feature vector for one Trade Scout candidate and negotiation tier; it contains neither an outcome label nor a completion probability."
                .to_owned(),
            "Availability is preserved as its own sourced or speculative feature and is not relabeled as trade-completion likelihood."
                .to_owned(),
            "Future training must join reviewed outcomes by proposal_id and freeze the entire model before an evaluation window."
                .to_owned(),
        ],
    })
}

fn validate_package(
    scout: &TradeScoutView,
    seller: &str,
    package: &TradeNegotiationPackageView,
) -> Result<(), TradeCompletionFeatureError> {
    let evaluation = &package.evaluation;
    let finite = [
        package.market_value,
        package.target_value_ratio,
        evaluation.fairness_score,
        evaluation.feasibility_probability,
        evaluation.buyer_utility_delta,
        evaluation.seller_utility_delta,
        evaluation.buyer_season_points_delta,
        evaluation.seller_season_points_delta,
    ]
    .into_iter()
    .all(f64::is_finite);
    if evaluation.buyer != scout.buyer
        || evaluation.seller != seller
        || !finite
        || !(0.0..=1.0).contains(&evaluation.fairness_score)
        || !(0.0..=1.0).contains(&evaluation.feasibility_probability)
    {
        return Err(TradeCompletionFeatureError::InvalidInput(format!(
            "{} package must match buyer/seller and contain finite, bounded scores",
            tier_label(package.tier)
        )));
    }
    Ok(())
}

fn valid_date_or_timestamp(value: &str) -> bool {
    DateTime::parse_from_rfc3339(value).is_ok()
        || NaiveDate::parse_from_str(value, "%Y-%m-%d").is_ok()
}

const fn tier_label(tier: TradeNegotiationTier) -> &'static str {
    match tier {
        TradeNegotiationTier::OpeningOffer => "opening",
        TradeNegotiationTier::FairMidpoint => "fair",
        TradeNegotiationTier::MaximumAcceptable => "maximum",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn example_scout_expands_every_candidate_to_three_unlabeled_rows() {
        let scout: TradeScoutView = serde_json::from_str(include_str!(
            "../../../examples/icecast-brv-trade-scout-board-2026-27.json"
        ))
        .unwrap();
        let view = build_trade_completion_feature_set(&scout).unwrap();
        assert_eq!(view.schema, TRADE_COMPLETION_FEATURE_SET_SCHEMA);
        assert_eq!(view.candidates, 3);
        assert_eq!(view.rows.len(), 9);
        assert_eq!(view.rows[0].buyer, "SEA");
        assert!(view.rows[0].proposal_id.ends_with(":opening"));
        assert_ne!(
            view.rows[0].availability_kind,
            TradeAvailabilityKind::Unavailable
        );
    }

    #[test]
    fn feature_builder_rejects_duplicate_candidates_and_wrong_package_teams() {
        let scout: TradeScoutView = serde_json::from_str(include_str!(
            "../../../examples/icecast-brv-trade-scout-board-2026-27.json"
        ))
        .unwrap();
        let mut invalid = scout.clone();
        invalid.candidates[1].target_id = invalid.candidates[0].target_id.clone();
        assert!(build_trade_completion_feature_set(&invalid).is_err());

        let mut invalid = scout;
        invalid.candidates[0]
            .negotiation
            .opening_offer
            .evaluation
            .buyer = "NYR".to_owned();
        assert!(build_trade_completion_feature_set(&invalid).is_err());
    }
}
