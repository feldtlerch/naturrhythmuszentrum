// SPDX-License-Identifier: AGPL-3.0-only
//! An empty common phase-offset intersection excludes every tuple of the original joint question.

use super::phase::admit::bounds;
use super::{AT, LcCertainty, LcDeterminacy, LcRelationError, storage};
use crate::{Budget, LcQuantity, LcRelation};
use crate::{LcSharedClockPhase, LcSharedMeasurements};
use nrz_cas::{Rational, limits::BudgetUsage};
use std::cmp::Ordering;

/// Complete measurement-only emptiness, distinct from an original physical contradiction.
#[derive(Clone, Debug)]
pub struct LcMeasurementImpossible {
    /// Full original clock/calibration question, with every original physical input.
    source: Source,
    /// Required lower and upper bounds on the same offset, with lower strictly greater.
    separation: [Rational; 2],
    /// Original input positions attaining the bounds; `None` denotes the declared offset bound.
    constraints: [Option<usize>; 2],
    /// Cumulative exact checking snapshots.
    usage: [BudgetUsage; 2],
    /// Conservative new retained contradiction metadata, excluding shared source owners.
    bytes: u64,
}

/// Calibration declarations and original allocation identity remain part of the certificate.
#[derive(Clone, Debug)]
enum Source {
    /// Original shared clock and phase offset with every independent phase support.
    Phase(LcSharedClockPhase),
    /// Original clock/phase question plus all amplitude readings and their shared gain.
    Amplitude(LcSharedMeasurements),
}

impl Source {
    /// Every retained question contains the original shared clock and phase calibration.
    fn phase(&self) -> &LcSharedClockPhase {
        match self {
            Self::Phase(phase) => phase,
            Self::Amplitude(amplitude) => amplitude.phase(),
        }
    }

    /// Conservative original shared ownership, including all calibration declarations.
    fn bytes(&self) -> u64 {
        let phase = self.phase();
        let bytes = phase
            .retained_payload_bytes()
            .saturating_add(phase.clock().retained_payload_bytes())
            .saturating_add(phase.relation().retained_payload_bytes());
        match self {
            Self::Phase(_) => bytes,
            Self::Amplitude(amplitude) => bytes.saturating_add(amplitude.retained_payload_bytes()),
        }
    }
}

impl LcSharedClockPhase {
    /// Certify that no single phase offset satisfies every original physical and measured strip.
    ///
    /// For each original physical interval `P_i` and measured interval `M_i`, every tuple
    /// requires `s >= P_i.lower-M_i.upper` and `s <= P_i.upper-M_i.lower`. A strict gap
    /// in their common intersection excludes all physical parameters and cycle aliases.
    /// # Errors
    /// Refuses unsupported nonrational/unbounded strips, a nonempty offset intersection or exhausted Work.
    /// Refusal supplies neither a feasibility claim nor an impossibility certificate.
    pub fn impossible_measurements(
        &self,
        budget: &mut Budget,
    ) -> Result<LcMeasurementImpossible, LcRelationError> {
        check(Source::Phase(self.clone()), budget)
    }
}

impl LcSharedMeasurements {
    /// Certify a common phase-offset contradiction while retaining every amplitude declaration as well.
    /// # Errors
    /// Refuses unsupported strips, absent strict contradiction and exhausted cumulative resources.
    pub fn impossible_measurements(
        &self,
        budget: &mut Budget,
    ) -> Result<LcMeasurementImpossible, LcRelationError> {
        check(Source::Amplitude(self.clone()), budget)
    }
}

impl LcMeasurementImpossible {
    /// Complete original physical question; this certificate concerns its joint measurement subset.
    #[must_use]
    pub fn relation(&self) -> &LcRelation {
        self.source.phase().relation()
    }

    /// Exact original clock/phase question when no amplitude owner is attached.
    #[must_use]
    pub const fn clock_phase(&self) -> Option<&LcSharedClockPhase> {
        match &self.source {
            Source::Phase(phase) => Some(phase),
            Source::Amplitude(_) => None,
        }
    }

    /// Exact original clock/phase/amplitude question when attached.
    #[must_use]
    pub const fn shared_measurements(&self) -> Option<&LcSharedMeasurements> {
        match &self.source {
            Source::Phase(_) => None,
            Source::Amplitude(amplitude) => Some(amplitude),
        }
    }

    /// The joint measurement question is empty independently of the physical relation's feasibility.
    #[must_use]
    pub const fn determinacy(&self) -> LcDeterminacy {
        LcDeterminacy::Impossible
    }

    /// A strict exact offset contradiction excludes every original tuple and alias.
    #[must_use]
    pub const fn certainty(&self) -> LcCertainty {
        LcCertainty::Certified
    }

    /// Incompatible lower and upper requirements on the identical original phase offset.
    #[must_use]
    pub const fn separation(&self) -> &[Rational; 2] {
        &self.separation
    }

    /// Original physical input positions responsible for the lower/upper requirements.
    /// A missing position means the corresponding original phase-offset endpoint.
    #[must_use]
    pub const fn constraints(&self) -> [Option<usize>; 2] {
        self.constraints
    }

    /// Exact before/after construction counters on the caller's ledger.
    #[must_use]
    pub const fn usage(&self) -> [BudgetUsage; 2] {
        self.usage
    }

    /// Conservative new contradiction metadata, excluding shared original sources.
    #[must_use]
    pub const fn retained_payload_bytes(&self) -> u64 {
        self.bytes
    }
}

/// Only checked strict separation publishes impossibility; an intermediate solver error never does.
fn check(source: Source, budget: &mut Budget) -> Result<LcMeasurementImpossible, LcRelationError> {
    let before = budget.usage();
    budget.work(1, AT)?;
    let phase = source.phase();
    let retained = source
        .bytes()
        .saturating_add(storage::size(phase.samples().len())?.saturating_mul(2_048));
    storage::retained(retained, budget)?;
    let samples = phase
        .samples()
        .iter()
        .map(|sample| bounds(sample, budget))
        .collect::<Result<Vec<_>, _>>()?;
    let mut separation = phase.offset().clone();
    let mut constraints = [None, None];
    for (position, input) in phase.relation().inputs().iter().enumerate() {
        budget.work(1, AT)?;
        let LcQuantity::Phase(index) = input.quantity() else {
            continue;
        };
        let physical = bounds(input.interval(), budget)?;
        let lower = physical[0].sub(&samples[index][1], budget)?;
        let upper = physical[1].sub(&samples[index][0], budget)?;
        if lower.compare(&separation[0], budget)? == Ordering::Greater {
            separation[0] = lower;
            constraints[0] = Some(position);
        }
        if upper.compare(&separation[1], budget)? == Ordering::Less {
            separation[1] = upper;
            constraints[1] = Some(position);
        }
    }
    if separation[0].compare(&separation[1], budget)? != Ordering::Greater {
        return Err(LcRelationError::Projection);
    }
    let bytes = separation.iter().fold(1_024_u64, |sum, value| {
        sum.saturating_add(storage::rational(value))
    });
    storage::retained(retained.saturating_add(bytes), budget)?;
    Ok(LcMeasurementImpossible {
        source,
        separation,
        constraints,
        usage: [before, budget.usage()],
        bytes,
    })
}
