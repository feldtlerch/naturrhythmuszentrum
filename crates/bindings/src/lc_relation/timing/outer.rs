// SPDX-License-Identifier: AGPL-3.0-only
//! Conservative nominal-time projection never promotes a relaxed tuple to original feasibility.

use super::{AT, Bounds, LcRelation, LcRelationError, LcSampleTimes, Rational, admit, storage};
use crate::{
    Budget, LcInput, LcInterval, LcPairsCover, LcPhaseGauge, LcQuantity as Q, LcValues, LcWitness,
};
use nrz_cas::{limits::BudgetUsage, pi_bounds::PiBoundCertificate};

/// Certified outer coverage only; rate/timing correlation was relaxed before projection.
///
/// Nonemptiness and determinacy of the relaxed nominal relation are not verdicts about
/// the original question. Only `check_witness` checks original feasibility.
#[derive(Clone, Debug)]
pub struct LcTimingOuterCover {
    /// Every original time box, nominal coordinate and retained positive rate bound.
    timing: LcSampleTimes,
    /// Complete nominal-time cover, kept private to prevent witness substitution.
    relaxed: LcPairsCover,
    /// Exact corner hull of `a*delta_i` for each original observation.
    advances: Vec<Bounds>,
    /// Cumulative admission and relaxation work on the caller's ledger.
    usage: [BudgetUsage; 2],
    /// Conservative complete retained exact ownership.
    bytes: u64,
}

impl LcSampleTimes {
    /// Widen each phase by the exact corner hull of its own rate-times-error product.
    ///
    /// Nominal phase is original phase minus `a*delta_i`. The existing complete
    /// nominal-time checker proves coverage of every original cycle, but a relaxed
    /// point remains only a candidate until the original equations and boxes are checked.
    /// # Errors
    /// Refuses unsupported physical inputs, incomplete cycle coverage and exhausted resources.
    pub fn outer_pairs_cover(
        &self,
        cycles: Vec<Rational>,
        pi: &PiBoundCertificate,
        budget: &mut Budget,
    ) -> Result<LcTimingOuterCover, LcRelationError> {
        let before = budget.usage();
        budget.work(1, AT)?;
        let mut advances = Vec::with_capacity(self.errors().len());
        for error in self.errors() {
            advances.push(admit::product(self.rate_support(), error, budget)?);
        }
        let effective = effective(self, &advances, budget)?;
        let relaxed = effective.pairs_cover(cycles, pi, budget)?;
        let bytes = advances.iter().flatten().fold(2_048_u64, |sum, value| {
            sum.saturating_add(storage::rational(value))
        });
        let bytes = bytes
            .saturating_add(self.retained_payload_bytes())
            .saturating_add(self.relation().retained_payload_bytes())
            .saturating_add(relaxed.retained_payload_bytes())
            .saturating_add(effective.retained_payload_bytes());
        storage::retained(bytes, budget)?;
        Ok(LcTimingOuterCover {
            timing: self.clone(),
            relaxed,
            advances,
            usage: [before, budget.usage()],
            bytes,
        })
    }
}

impl LcTimingOuterCover {
    /// Original independent timestamp question; no common offset is inferred.
    #[must_use]
    pub const fn timing(&self) -> &LcSampleTimes {
        &self.timing
    }

    /// Outer rate support, without an assertion that every rate is originally feasible.
    #[must_use]
    pub fn rate(&self) -> &Bounds {
        self.relaxed.rate()
    }

    /// Complete original cycle proposal, including the nominal checker's common-lift convention.
    #[must_use]
    pub fn cycles(&self) -> &[Rational] {
        self.relaxed.cycles()
    }

    /// Complete common integer gauge inherited from the original initial-phase declaration.
    #[must_use]
    pub fn phase_gauge(&self) -> LcPhaseGauge {
        self.relaxed.phase_gauge()
    }

    /// Exact corner hulls used in the conservative phase widening.
    #[must_use]
    pub fn timing_advances(&self) -> &[Bounds] {
        &self.advances
    }

    /// Outer initial-phase fibre at a retained rate, not an original existence certificate.
    /// # Errors
    /// Refuses rates outside the relaxed domain and exhausted resources.
    pub fn phase_at_rate(
        &self,
        rate: &Rational,
        budget: &mut Budget,
    ) -> Result<Bounds, LcRelationError> {
        self.relaxed.phase_at_rate(rate, budget)
    }

    /// Independently check every original equation and timestamp box for a proposed tuple.
    /// # Errors
    /// Refuses any original equation or input violation and exhausted resources.
    pub fn check_witness(
        &self,
        values: LcValues,
        budget: &mut Budget,
    ) -> Result<LcWitness, LcRelationError> {
        self.timing.relation().check_witness(values, budget)
    }

    /// Cumulative snapshots include widening and complete nominal-time projection.
    #[must_use]
    pub const fn usage(&self) -> [BudgetUsage; 2] {
        self.usage
    }

    /// Conservative complete exact metadata ownership for the relaxed answer.
    #[must_use]
    pub const fn retained_payload_bytes(&self) -> u64 {
        self.bytes
    }
}

/// Each original feasible tuple maps to these widened nominal constraints at the same rate.
fn effective(
    timing: &LcSampleTimes,
    advances: &[Bounds],
    budget: &mut Budget,
) -> Result<LcRelation, LcRelationError> {
    storage::retained(
        timing.relation().retained_payload_bytes().saturating_mul(2),
        budget,
    )?;
    let mut inputs = Vec::new();
    for original in timing.relation().inputs() {
        budget.work(1, AT)?;
        match original.quantity() {
            Q::Time(_) => (),
            Q::Phase(index) => {
                let advance = &advances[index];
                let interval = original.interval();
                let lower = interval.lower().ok_or(LcRelationError::Projection)?;
                let upper = interval.upper().ok_or(LcRelationError::Projection)?;
                let interval = LcInterval::new(
                    Some(lower.sub(&advance[1].clone().into(), budget)?),
                    Some(upper.sub(&advance[0].clone().into(), budget)?),
                    budget,
                )?;
                inputs.push(LcInput::new(
                    original.quantity(),
                    original.descriptor(),
                    interval,
                    budget,
                )?);
            }
            _ => inputs.push(original.clone()),
        }
    }
    for (index, nominal) in timing.nominal_times().iter().enumerate() {
        inputs.push(admit::input(
            Q::Time(index),
            [nominal.clone(), nominal.clone()],
            budget,
        )?);
    }
    LcRelation::new(
        timing.relation().epoch().to_owned(),
        timing.relation().observations(),
        inputs,
        budget,
    )
}
