//! IceLines signal metrics.
//!
//! Signals are descriptive composite metrics built from existing catalog fields.
//! They are intentionally separate from `StatId` until their methodology and
//! consumer fit are proven across product surfaces.

use crate::model::MIN_GP;
use crate::stats_repository::PlayerView;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SignalMetricId {
    PhysicalEngagementRate,
    PuckManagementDifferential,
    PenaltyDragRate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SignalMetricUnit {
    Per60,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SignalPolarity {
    HigherIsBetter,
    LowerIsBetter,
    Neutral,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SignalInput {
    SampleSize,
    Realtime,
    IceTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignalMetricDescriptor {
    pub id: SignalMetricId,
    pub label: &'static str,
    pub short_label: &'static str,
    pub cli_key: &'static str,
    pub unit: SignalMetricUnit,
    pub polarity: SignalPolarity,
    pub required_inputs: &'static [SignalInput],
    pub methodology: &'static str,
    pub limitations: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SignalEvidenceTier {
    Missing,
    Partial,
    Full,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignalEvidence {
    pub tier: SignalEvidenceTier,
    pub missing_inputs: Vec<SignalInput>,
}

const PHYSICAL_ENGAGEMENT_INPUTS: &[SignalInput] = &[
    SignalInput::SampleSize,
    SignalInput::Realtime,
    SignalInput::IceTime,
];
const PUCK_MANAGEMENT_INPUTS: &[SignalInput] = &[
    SignalInput::SampleSize,
    SignalInput::Realtime,
    SignalInput::IceTime,
];
const PENALTY_DRAG_INPUTS: &[SignalInput] = &[SignalInput::SampleSize, SignalInput::IceTime];

impl SignalMetricId {
    pub fn all() -> &'static [SignalMetricId] {
        use SignalMetricId::*;
        &[
            PhysicalEngagementRate,
            PuckManagementDifferential,
            PenaltyDragRate,
        ]
    }

    pub fn descriptor(self) -> SignalMetricDescriptor {
        match self {
            Self::PhysicalEngagementRate => SignalMetricDescriptor {
                id: self,
                label: "Physical Engagement Rate",
                short_label: "Phys/60",
                cli_key: "physical-engagement-rate",
                unit: SignalMetricUnit::Per60,
                polarity: SignalPolarity::Neutral,
                required_inputs: PHYSICAL_ENGAGEMENT_INPUTS,
                methodology: "(hits + blocked shots) per 60 minutes",
                limitations: "Descriptive only; hit and block recording carries rink scorer bias and does not prove puck recovery, possession value, or player quality.",
            },
            Self::PuckManagementDifferential => SignalMetricDescriptor {
                id: self,
                label: "Puck Management Differential",
                short_label: "PMD/60",
                cli_key: "puck-management-differential",
                unit: SignalMetricUnit::Per60,
                polarity: SignalPolarity::HigherIsBetter,
                required_inputs: PUCK_MANAGEMENT_INPUTS,
                methodology: "(takeaways - giveaways) per 60 minutes",
                limitations: "Descriptive only; takeaway/giveaway recording is scorer-dependent and does not isolate teammates, zone, deployment, or puck-recovery context.",
            },
            Self::PenaltyDragRate => SignalMetricDescriptor {
                id: self,
                label: "Penalty Drag Rate",
                short_label: "PIM/60",
                cli_key: "penalty-drag-rate",
                unit: SignalMetricUnit::Per60,
                polarity: SignalPolarity::LowerIsBetter,
                required_inputs: PENALTY_DRAG_INPUTS,
                methodology: "penalty minutes per 60 minutes",
                limitations: "Descriptive only; penalty minutes mix minors, majors, misconducts, and coincidental penalties and do not by themselves prove avoidable team harm.",
            },
        }
    }

    pub fn evidence(self, view: &PlayerView<'_>) -> SignalEvidence {
        let descriptor = self.descriptor();
        let mut missing_inputs = Vec::new();
        for input in descriptor.required_inputs {
            if !input.is_available(view) {
                missing_inputs.push(*input);
            }
        }
        let tier = if missing_inputs.is_empty() {
            SignalEvidenceTier::Full
        } else if missing_inputs.len() == descriptor.required_inputs.len() {
            SignalEvidenceTier::Missing
        } else {
            SignalEvidenceTier::Partial
        };
        SignalEvidence {
            tier,
            missing_inputs,
        }
    }

    pub fn read(self, view: &PlayerView<'_>) -> Option<f64> {
        if self.evidence(view).tier != SignalEvidenceTier::Full {
            return None;
        }
        match self {
            Self::PhysicalEngagementRate => {
                let realtime = view.stats.realtime.as_ref()?;
                per_60(view, realtime.hits.saturating_add(realtime.blocked_shots))
            }
            Self::PuckManagementDifferential => {
                let realtime = view.stats.realtime.as_ref()?;
                let takeaways = per_60(view, realtime.takeaways)?;
                let giveaways = per_60(view, realtime.giveaways)?;
                Some(takeaways - giveaways)
            }
            Self::PenaltyDragRate => per_60(view, view.stats.totals.pim),
        }
    }
}

impl SignalInput {
    fn is_available(self, view: &PlayerView<'_>) -> bool {
        match self {
            Self::SampleSize => view.gp() >= MIN_GP,
            Self::Realtime => view.stats.realtime.is_some(),
            Self::IceTime => total_toi_sec(view).is_some(),
        }
    }
}

fn total_toi_sec(view: &PlayerView<'_>) -> Option<u32> {
    let toi_sec = view
        .stats
        .time_on_ice
        .as_ref()
        .map(|t| t.time_on_ice_sec)
        .or_else(|| {
            view.stats
                .totals
                .toi_per_game_sec
                .map(|per_g| per_g.saturating_mul(view.gp()))
        })?;
    (toi_sec >= 300).then_some(toi_sec)
}

fn per_60(view: &PlayerView<'_>, count: u32) -> Option<f64> {
    Some(count as f64 * 3600.0 / total_toi_sec(view)? as f64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures::{identity, stats};
    use crate::stats_repository::PlayerView;

    fn signal_view() -> (
        crate::identity::PlayerIdentity,
        crate::season_stats::SeasonStats,
    ) {
        let identity = identity(8478402).build();
        let stats = stats(8478402, 20252026, "EDM")
            .realtime(140, 70, 35, 21)
            .build();
        (identity, stats)
    }

    fn view<'a>(
        identity: &'a crate::identity::PlayerIdentity,
        stats: &'a crate::season_stats::SeasonStats,
    ) -> PlayerView<'a> {
        PlayerView {
            identity,
            stats,
            contract: None,
        }
    }

    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1e-9,
            "actual={actual} expected={expected}"
        );
    }

    #[test]
    fn l0_signal_metrics_have_stable_descriptors_and_sort_polarity() {
        let all = SignalMetricId::all();
        assert_eq!(
            all,
            &[
                SignalMetricId::PhysicalEngagementRate,
                SignalMetricId::PuckManagementDifferential,
                SignalMetricId::PenaltyDragRate
            ]
        );

        let physical = SignalMetricId::PhysicalEngagementRate.descriptor();
        assert_eq!(physical.cli_key, "physical-engagement-rate");
        assert_eq!(physical.polarity, SignalPolarity::Neutral);
        assert!(physical.limitations.contains("scorer bias"));

        let puck_management = SignalMetricId::PuckManagementDifferential.descriptor();
        assert_eq!(puck_management.polarity, SignalPolarity::HigherIsBetter);
        assert!(puck_management.limitations.contains("scorer-dependent"));

        let penalty_drag = SignalMetricId::PenaltyDragRate.descriptor();
        assert_eq!(penalty_drag.polarity, SignalPolarity::LowerIsBetter);
        assert!(penalty_drag.limitations.contains("coincidental penalties"));
    }

    #[test]
    fn l0_signal_metrics_compute_from_existing_realtime_and_toi_inputs() {
        let (identity, stats) = signal_view();
        let view = view(&identity, &stats);

        assert_close(
            SignalMetricId::PhysicalEngagementRate.read(&view).unwrap(),
            9.0,
        );
        assert_close(
            SignalMetricId::PuckManagementDifferential
                .read(&view)
                .unwrap(),
            0.6,
        );
        assert_close(
            SignalMetricId::PenaltyDragRate.read(&view).unwrap(),
            20.0 * 3600.0 / (70.0 * 20.0 * 60.0),
        );
    }

    #[test]
    fn l0_signal_metrics_do_not_zero_fill_missing_realtime() {
        let identity = identity(8478402).build();
        let stats = stats(8478402, 20252026, "EDM").build();
        let view = view(&identity, &stats);

        assert_eq!(SignalMetricId::PhysicalEngagementRate.read(&view), None);
        assert_eq!(SignalMetricId::PuckManagementDifferential.read(&view), None);
        assert!(SignalMetricId::PenaltyDragRate.read(&view).is_some());

        let evidence = SignalMetricId::PhysicalEngagementRate.evidence(&view);
        assert_eq!(evidence.tier, SignalEvidenceTier::Partial);
        assert_eq!(evidence.missing_inputs, vec![SignalInput::Realtime]);
    }

    #[test]
    fn l0_signal_metrics_refuse_missing_ice_time_and_small_samples() {
        let (identity, mut stats) = signal_view();
        stats.totals.toi_per_game_sec = None;
        let missing_toi_view = view(&identity, &stats);
        assert_eq!(
            SignalMetricId::PenaltyDragRate.read(&missing_toi_view),
            None
        );
        assert_eq!(
            SignalMetricId::PenaltyDragRate
                .evidence(&missing_toi_view)
                .missing_inputs,
            vec![SignalInput::IceTime]
        );

        let (identity, mut stats) = signal_view();
        stats.totals.gp = MIN_GP - 1;
        let small_sample_view = view(&identity, &stats);
        assert_eq!(
            SignalMetricId::PhysicalEngagementRate.read(&small_sample_view),
            None
        );
        assert_eq!(
            SignalMetricId::PhysicalEngagementRate
                .evidence(&small_sample_view)
                .missing_inputs,
            vec![SignalInput::SampleSize]
        );
    }

    #[test]
    fn l0_signal_metrics_preserve_ordering_contracts() {
        let (identity, mut stats) = signal_view();
        let baseline = SignalMetricId::PuckManagementDifferential
            .read(&view(&identity, &stats))
            .unwrap();

        stats.realtime.as_mut().unwrap().takeaways += 14;
        let improved = SignalMetricId::PuckManagementDifferential
            .read(&view(&identity, &stats))
            .unwrap();
        assert!(improved > baseline);

        let (identity, mut stats) = signal_view();
        let baseline = SignalMetricId::PenaltyDragRate
            .read(&view(&identity, &stats))
            .unwrap();
        stats.totals.pim += 20;
        let worse = SignalMetricId::PenaltyDragRate
            .read(&view(&identity, &stats))
            .unwrap();
        assert!(worse > baseline);
        assert_eq!(
            SignalMetricId::PenaltyDragRate.descriptor().polarity,
            SignalPolarity::LowerIsBetter
        );
    }
}
