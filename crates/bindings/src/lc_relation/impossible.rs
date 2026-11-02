// SPDX-License-Identifier: AGPL-3.0-only
//! An exact same-time phase gap excludes every cycle alias of the original relation.

use super::{AT, LcCertainty, LcDeterminacy, LcQuantity, LcRelation, LcRelationError, storage};
use crate::{Budget, ExactConstant, LcInterval};
use nrz_cas::{
    Rational,
    limits::BudgetUsage,
    pi_bounds::{CheckedPiBounds, PiBoundCertificate},
    syntax::Constant,
};
use std::cmp::Ordering;

/// Certified emptiness retains the original question and its complete all-alias contradiction.
#[derive(Clone, Debug)]
pub struct LcImpossible {
    /// Every original constraint remains present; a conflicting subset proves the whole set empty.
    relation: LcRelation,
    /// Original ordered pair indices with the same exact timestamp.
    observations: [usize; 2],
    /// Checked integer lifts used only to express a strict gap between consecutive full turns.
    cycles: [Rational; 2],
    /// Exact positive phase-difference support strictly below one full turn.
    gap: [Rational; 2],
    /// Independently checked pi evidence proving every other integer lift is excluded.
    pi: CheckedPiBounds,
    /// Before/after exact checking snapshots on the original caller ledger.
    usage: [BudgetUsage; 2],
    /// Conservative retained contradiction metadata, excluding the shared original relation.
    bytes: u64,
}

impl LcRelation {
    /// Check that two exact same-time phase supports cannot agree modulo any full turn.
    ///
    /// The proposed lifts must put the entire phase difference strictly between zero
    /// and `2*pi`; thus no integer difference of original cycle indices can satisfy it.
    /// # Errors
    /// Refuses malformed indices, absent/exactly unequal times, unresolved gaps,
    /// false pi evidence and exhausted resources. A failed proposal is not impossibility.
    pub fn impossible_pair(
        &self,
        observations: [usize; 2],
        cycles: [Rational; 2],
        pi: &PiBoundCertificate,
        budget: &mut Budget,
    ) -> Result<LcImpossible, LcRelationError> {
        let before = budget.usage();
        budget.work(1, AT)?;
        if observations[0] == observations[1]
            || observations.iter().any(|i| *i >= self.observations())
        {
            return Err(LcRelationError::Observation);
        }
        let left = exact_time(self, observations[0], budget)?;
        let right = exact_time(self, observations[1], budget)?;
        if left != right {
            return Err(LcRelationError::Projection);
        }
        let pi = pi.check(budget)?;
        let first = lift(self, observations[0], &cycles[0], budget)?;
        let second = lift(self, observations[1], &cycles[1], budget)?;
        let gap = [
            second[0].sub(&first[1], budget)?,
            second[1].sub(&first[0], budget)?,
        ];
        let turn_lower = pi
            .contract()
            .lower
            .mul(&Rational::from_ratio(2, 1, budget)?, budget)?;
        let positive = gap[0].compare(&Rational::zero(), budget)? == Ordering::Greater;
        let below_turn = gap[1].compare(&turn_lower, budget)? == Ordering::Less;
        if !positive || !below_turn {
            return Err(LcRelationError::CycleCover);
        }
        let bytes = cycles
            .iter()
            .chain(&gap)
            .fold(pi.retained_bytes().saturating_add(1_024), |bytes, value| {
                bytes.saturating_add(storage::rational(value))
            });
        storage::retained(bytes.saturating_add(self.retained_payload_bytes()), budget)?;
        Ok(LcImpossible {
            relation: self.clone(),
            observations,
            cycles,
            gap,
            pi,
            usage: [before, budget.usage()],
            bytes,
        })
    }
}

impl LcImpossible {
    /// Original complete relation to which the emptiness certificate applies.
    #[must_use]
    pub const fn relation(&self) -> &LcRelation {
        &self.relation
    }

    /// Structural impossibility is independent of numerical endpoint materialization.
    #[must_use]
    pub const fn determinacy(&self) -> LcDeterminacy {
        LcDeterminacy::Impossible
    }

    /// The exact strict gap excludes every cycle alias, with no unresolved feasibility branch.
    #[must_use]
    pub const fn certainty(&self) -> LcCertainty {
        LcCertainty::Certified
    }

    /// Conflicting original observation positions.
    #[must_use]
    pub const fn observations(&self) -> [usize; 2] {
        self.observations
    }

    /// Original proposed integer lifts, retained with the gap derivation.
    #[must_use]
    pub const fn cycles(&self) -> &[Rational; 2] {
        &self.cycles
    }

    /// Exact forbidden difference interval, strictly inside a full-turn lattice gap.
    #[must_use]
    pub const fn gap(&self) -> &[Rational; 2] {
        &self.gap
    }

    /// Independent exact pi evidence for the strict next-turn comparison.
    #[must_use]
    pub const fn pi_bounds(&self) -> &CheckedPiBounds {
        &self.pi
    }

    /// Cumulative before/after exact checking snapshots.
    #[must_use]
    pub const fn usage(&self) -> [BudgetUsage; 2] {
        self.usage
    }

    /// Conservative new exact metadata ownership.
    #[must_use]
    pub const fn retained_payload_bytes(&self) -> u64 {
        self.bytes
    }
}

/// Any original constraint in a contradictory subset suffices; all others stay in the relation.
fn input(
    relation: &LcRelation,
    quantity: LcQuantity,
    budget: &mut Budget,
) -> Result<LcInterval, LcRelationError> {
    for input in relation.inputs() {
        budget.work(1, AT)?;
        if input.quantity() == quantity {
            return Ok(input.interval().clone());
        }
    }
    Err(LcRelationError::Projection)
}

/// Equal exact timestamps are a premise, never inferred from overlapping time boxes.
fn exact_time(
    relation: &LcRelation,
    index: usize,
    budget: &mut Budget,
) -> Result<Rational, LcRelationError> {
    budget.work(1, AT)?;
    let interval = input(relation, LcQuantity::Time(index), budget)?;
    let lower = interval
        .lower()
        .and_then(ExactConstant::as_rational)
        .ok_or(LcRelationError::Projection)?;
    let upper = interval
        .upper()
        .and_then(ExactConstant::as_rational)
        .ok_or(LcRelationError::Projection)?;
    lower.check(budget, AT)?;
    upper.check(budget, AT)?;
    if lower != upper {
        return Err(LcRelationError::Projection);
    }
    Ok(lower.clone())
}

/// A proposed full-turn translation is checked integrally and applied to both exact endpoints.
fn lift(
    relation: &LcRelation,
    index: usize,
    cycle: &Rational,
    budget: &mut Budget,
) -> Result<[Rational; 2], LcRelationError> {
    budget.work(1, AT)?;
    cycle.check(budget, AT)?;
    if !cycle.is_integer() {
        return Err(LcRelationError::Cycle);
    }
    let turn = ExactConstant::named(Constant::Pi, budget)?
        .mul(&Rational::from_ratio(2, 1, budget)?.into(), budget)?;
    let shift = turn.mul(&cycle.clone().into(), budget)?;
    let interval = input(relation, LcQuantity::Phase(index), budget)?;
    let lower = interval
        .lower()
        .ok_or(LcRelationError::Projection)?
        .add(&shift, budget)?;
    let upper = interval
        .upper()
        .ok_or(LcRelationError::Projection)?
        .add(&shift, budget)?;
    Ok([
        lower
            .as_rational()
            .cloned()
            .ok_or(LcRelationError::Projection)?,
        upper
            .as_rational()
            .cloned()
            .ok_or(LcRelationError::Projection)?,
    ])
}
