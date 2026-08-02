//! Provider-neutral fragments used by fetch-owned source-package assembly.

use crate::ahl::roster_stats::AhlRosterStatsOutput;
use crate::nhl::club_publication::ClubPublicationOutput;
use crate::nhl::contract_publication::ContractPublicationOutput;
use crate::nhl::draft_picks::DraftPicksOutput;
use crate::nhl::termination_publication::TerminationPublicationOutput;
use crate::nhl::trade_tracker::TradeTrackerOutput;
use icelines_core::source_facts::{
    FactAssertion, ProviderIdentityProposal, SourceExclusion, SourceFact, StagedPlayerAssertion,
};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct SourcePackageFragment {
    pub fact_assertions: Vec<FactAssertion<SourceFact>>,
    pub identity_proposals: Vec<ProviderIdentityProposal>,
    pub staged_player_assertions: Vec<StagedPlayerAssertion>,
    pub exclusions: Vec<SourceExclusion>,
}

impl SourcePackageFragment {
    pub fn from_facts(fact_assertions: Vec<FactAssertion<SourceFact>>) -> Self {
        Self {
            fact_assertions,
            ..Self::default()
        }
    }

    pub fn combine(mut self, mut other: Self) -> Self {
        self.fact_assertions.append(&mut other.fact_assertions);
        self.identity_proposals
            .append(&mut other.identity_proposals);
        self.staged_player_assertions
            .append(&mut other.staged_player_assertions);
        self.exclusions.append(&mut other.exclusions);
        self
    }
}

impl From<&ClubPublicationOutput> for SourcePackageFragment {
    fn from(output: &ClubPublicationOutput) -> Self {
        Self {
            identity_proposals: output.identity_proposals.clone(),
            staged_player_assertions: output.staged_assertions.clone(),
            ..Self::default()
        }
    }
}

impl From<&ContractPublicationOutput> for SourcePackageFragment {
    fn from(output: &ContractPublicationOutput) -> Self {
        Self {
            identity_proposals: vec![output.identity_proposal.clone()],
            staged_player_assertions: vec![output.staged_assertion.clone()],
            ..Self::default()
        }
    }
}

impl From<&TerminationPublicationOutput> for SourcePackageFragment {
    fn from(output: &TerminationPublicationOutput) -> Self {
        Self {
            identity_proposals: vec![output.identity_proposal.clone()],
            staged_player_assertions: vec![output.staged_assertion.clone()],
            ..Self::default()
        }
    }
}

impl From<&DraftPicksOutput> for SourcePackageFragment {
    fn from(output: &DraftPicksOutput) -> Self {
        Self {
            identity_proposals: output.identity_proposals.clone(),
            staged_player_assertions: output.staged_assertions.clone(),
            ..Self::default()
        }
    }
}

impl From<&AhlRosterStatsOutput> for SourcePackageFragment {
    fn from(output: &AhlRosterStatsOutput) -> Self {
        Self {
            identity_proposals: output.identity_proposals.clone(),
            staged_player_assertions: output.staged_assertions.clone(),
            ..Self::default()
        }
    }
}

impl From<&TradeTrackerOutput> for SourcePackageFragment {
    fn from(output: &TradeTrackerOutput) -> Self {
        let source_id = output.evidence.source_id().clone();
        let exclusions = output
            .ignored_assets
            .iter()
            .enumerate()
            .map(|(index, asset)| SourceExclusion {
                exclusion_id: format!(
                    "trade-asset:{}:{}:{}:{:03}",
                    asset.transaction_row,
                    asset.from.as_str(),
                    asset.to.as_str(),
                    index + 1
                ),
                stage: "source_normalization".to_owned(),
                subject: None,
                reason_code: "non_player_trade_asset".to_owned(),
                message: asset.description.clone(),
                source_ids: vec![source_id.clone()],
            })
            .collect();
        Self {
            identity_proposals: output.identity_proposals.clone(),
            staged_player_assertions: output.staged_assertions.clone(),
            exclusions,
            ..Self::default()
        }
    }
}
