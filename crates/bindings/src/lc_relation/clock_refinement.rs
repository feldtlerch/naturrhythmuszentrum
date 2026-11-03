// SPDX-License-Identifier: AGPL-3.0-only
//! Independent and shared-clock refinement retain whole questions and previous evidence on exhausted solving.

mod attempt;
mod check;
mod inclusion;

pub use attempt::{LcClockRefinement, LcClockRefinementState, LcClockRefinementSuccess};
pub use inclusion::LcClockRefinementInputs;

use super::{AT, LcRelationError, storage};
use crate::{
    Budget, LcClockCover, LcHorizon, LcHorizonCoordinate, LcRelation, LcSampleTimes, LcSharedClock,
    LcSharedTiming, LcSharedTimingCover, LcTimingCover,
};
use nrz_cas::{Rational, pi_bounds::CheckedPiBounds};

/// One original measurement model remains unchanged while its declared supports narrow.
#[derive(Clone, Debug)]
enum Question {
    /// One common clock offset with exact nominal sample coordinates.
    Clock(LcSharedClock),
    /// One common offset and a separate error interval for every observation.
    SharedTiming(LcSharedTiming),
    /// Independent sample errors without a shared offset.
    Timing(LcSampleTimes),
}

impl Question {
    /// Complete original physical relation.
    fn relation(&self) -> &LcRelation {
        match self {
            Self::Clock(value) => value.relation(),
            Self::SharedTiming(value) => value.relation(),
            Self::Timing(value) => value.relation(),
        }
    }
    /// Conservative complete original measurement metadata.
    fn bytes(&self) -> u64 {
        match self {
            Self::Clock(value) => value.retained_payload_bytes(),
            Self::SharedTiming(value) => value.retained_payload_bytes(),
            Self::Timing(value) => value.retained_payload_bytes(),
        }
    }
    /// Original single-offset question, when applicable.
    const fn clock(&self) -> Option<&LcSharedClock> {
        match self {
            Self::Clock(value) => Some(value),
            Self::SharedTiming(_) | Self::Timing(_) => None,
        }
    }
    /// Original two-layer question, when applicable.
    const fn shared_timing(&self) -> Option<&LcSharedTiming> {
        match self {
            Self::SharedTiming(value) => Some(value),
            Self::Clock(_) | Self::Timing(_) => None,
        }
    }
    /// Original independent timing question, when applicable.
    const fn timing(&self) -> Option<&LcSampleTimes> {
        match self {
            Self::Timing(value) => Some(value),
            Self::Clock(_) | Self::SharedTiming(_) => None,
        }
    }
    /// Reuse the previous prepared circle evidence while solving the complete refined question.
    fn solve(
        &self,
        cycles: Vec<Rational>,
        pi: CheckedPiBounds,
        budget: &mut Budget,
    ) -> Result<Cover, LcRelationError> {
        match self {
            Self::Clock(value) => Ok(Cover::Clock(value.pairs_cover_checked(cycles, pi, budget)?)),
            Self::Timing(value) => Ok(Cover::Timing(
                value.pairs_cover_checked(cycles, pi, budget)?,
            )),
            Self::SharedTiming(value) => Ok(Cover::SharedTiming(
                value.pairs_cover_checked(cycles, pi, budget)?,
            )),
        }
    }
}

/// One certified complete source stays retained across successful and unresolved attempts.
#[derive(Clone, Debug)]
enum Cover {
    /// Complete original single-clock family.
    Clock(LcClockCover),
    /// Complete original two-layer timing family.
    SharedTiming(LcSharedTimingCover),
    /// Complete original independent timing family.
    Timing(LcTimingCover),
}

impl Cover {
    /// Original relation before any refinement attempt.
    fn relation(&self) -> &LcRelation {
        match self {
            Self::Clock(value) => value.relation(),
            Self::SharedTiming(value) => value.relation(),
            Self::Timing(value) => value.relation(),
        }
    }
    /// Exact complete positive rate projection.
    fn rate(&self) -> &[Rational; 2] {
        match self {
            Self::Clock(value) => value.rate(),
            Self::SharedTiming(value) => value.rate(),
            Self::Timing(value) => value.rate(),
        }
    }
    /// Original complete cycle association.
    fn cycles(&self) -> &[Rational] {
        match self {
            Self::Clock(value) => value.cycles(),
            Self::SharedTiming(value) => value.cycles(),
            Self::Timing(value) => value.cycles(),
        }
    }
    /// Conservative retained source ownership.
    fn bytes(&self) -> u64 {
        match self {
            Self::Clock(value) => value.retained_payload_bytes(),
            Self::SharedTiming(value) => value.retained_payload_bytes(),
            Self::Timing(value) => value.retained_payload_bytes(),
        }
    }
    /// Original single-clock cover, when applicable.
    const fn clock(&self) -> Option<&LcClockCover> {
        match self {
            Self::Clock(value) => Some(value),
            Self::SharedTiming(_) | Self::Timing(_) => None,
        }
    }
    /// Original two-layer cover, when applicable.
    const fn shared_timing(&self) -> Option<&LcSharedTimingCover> {
        match self {
            Self::SharedTiming(value) => Some(value),
            Self::Clock(_) | Self::Timing(_) => None,
        }
    }
    /// Original independent timing cover, when applicable.
    const fn timing(&self) -> Option<&LcTimingCover> {
        match self {
            Self::Timing(value) => Some(value),
            Self::Clock(_) | Self::SharedTiming(_) => None,
        }
    }
    /// Equal values never replace the exact original measurement-question authority.
    fn matches_question(&self, question: &Question) -> bool {
        match (self, question) {
            (Self::Clock(cover), Question::Clock(question)) => {
                cover.clock().shares_allocation(question)
            }
            (Self::SharedTiming(cover), Question::SharedTiming(question)) => {
                cover.timing().shares_allocation(question)
            }
            (Self::Timing(cover), Question::Timing(question)) => {
                cover.timing().shares_allocation(question)
            }
            _ => false,
        }
    }
    /// The old horizon must retain this exact certified cover allocation.
    fn matches_horizon(&self, horizon: &LcHorizon) -> bool {
        match self {
            Self::Timing(value) => horizon
                .timing_cover()
                .is_some_and(|source| source.shares_allocation(value)),
            Self::Clock(value) => horizon
                .clock_cover()
                .is_some_and(|source| source.shares_allocation(value)),
            Self::SharedTiming(value) => horizon
                .shared_timing_cover()
                .is_some_and(|source| source.shares_allocation(value)),
        }
    }
    /// Refined horizons use the identical coordinate and explicit common future reference.
    fn horizon(
        &self,
        coordinate: LcHorizonCoordinate,
        reference: &Rational,
        budget: &mut Budget,
    ) -> Result<LcHorizon, LcRelationError> {
        match (self, coordinate) {
            (Self::Timing(value), LcHorizonCoordinate::ActualTime) => {
                value.forward_horizon_from(reference, budget)
            }
            (Self::Timing(_), LcHorizonCoordinate::ReportedClock) => {
                Err(LcRelationError::Projection)
            }
            (Self::Clock(value), LcHorizonCoordinate::ActualTime) => {
                value.forward_horizon_from(reference, budget)
            }
            (Self::Clock(value), LcHorizonCoordinate::ReportedClock) => {
                value.forward_reported_horizon_from(reference, budget)
            }
            (Self::SharedTiming(value), LcHorizonCoordinate::ActualTime) => {
                value.forward_horizon_from(reference, budget)
            }
            (Self::SharedTiming(value), LcHorizonCoordinate::ReportedClock) => {
                value.forward_reported_horizon_from(reference, budget)
            }
        }
    }
}
