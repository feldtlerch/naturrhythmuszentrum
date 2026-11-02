// SPDX-License-Identifier: AGPL-3.0-only
//! A checked positive homogeneity map reuses the original base witness across all aliases.

use super::super::AT;
use super::{LcAliasCover, LcPairParameters, LcRelationError, LcWitness, Rational, storage};
use crate::{Budget, ExactConstant, LcRelation, LcValues};
use nrz_cas::limits::BudgetUsage;

/// Exact existence evidence combines a checked original tuple with a total alias transformation.
#[derive(Clone, Debug)]
pub struct LcAliasWitness {
    /// Original unbounded-frequency relation; no derived representative bound is imposed here.
    relation: LcRelation,
    /// Complete transformed original coordinates and integer observation cycles.
    values: LcValues,
    /// Checked base tuple reused as the premise of the positive homogeneity map.
    base: LcWitness,
    /// Positive factor `base_rate / alias_rate` multiplying both elements and energy.
    factor: ExactConstant,
    /// Original exact nonnegative alias index.
    index: Rational,
    /// Cumulative before/after transformation and base-check counters.
    usage: [BudgetUsage; 2],
}

impl LcAliasWitness {
    /// Original complete unbounded relation whose equations the transformation preserves.
    #[must_use]
    pub const fn relation(&self) -> &LcRelation {
        &self.relation
    }

    /// Full original coordinates after the checked alias transformation.
    #[must_use]
    pub const fn values(&self) -> &LcValues {
        &self.values
    }

    /// Independently checked base equations reused by this existence certificate.
    #[must_use]
    pub const fn base_witness(&self) -> &LcWitness {
        &self.base
    }

    /// Positive shared element/energy factor; amplitudes and their ratio remain identical.
    #[must_use]
    pub const fn factor(&self) -> &ExactConstant {
        &self.factor
    }

    /// Exact retained frequency-alias identifier.
    #[must_use]
    pub const fn index(&self) -> &Rational {
        &self.index
    }

    /// Check a proposed full tuple against the retained checked preservation map.
    ///
    /// The base equations, positive common factor and integer timestamp lattice are
    /// retained premises. Every transformed coordinate must match their exact image.
    /// # Errors
    /// Refuses any changed coordinate, phase, timestamp or cycle and exhausted resources.
    pub fn check_transformed_values(
        &self,
        proposal: &LcValues,
        budget: &mut Budget,
    ) -> Result<(), LcRelationError> {
        for (expected, proposed) in [
            (&self.values.inductance, &proposal.inductance),
            (&self.values.capacitance, &proposal.capacitance),
            (&self.values.rate, &proposal.rate),
            (&self.values.voltage, &proposal.voltage),
            (&self.values.current, &proposal.current),
            (&self.values.energy, &proposal.energy),
        ] {
            equal(expected, proposed, budget)?;
        }
        let initial = proposal
            .initial_phase
            .as_ref()
            .ok_or(LcRelationError::Observation)?;
        let expected = self
            .values
            .initial_phase
            .as_ref()
            .ok_or(LcRelationError::Observation)?;
        equal(expected, initial, budget)?;
        if proposal.observations.len() != self.values.observations.len() {
            return Err(LcRelationError::Observation);
        }
        for (expected, proposed) in self.values.observations.iter().zip(&proposal.observations) {
            budget.work(1, AT)?;
            equal(&expected.time, &proposed.time, budget)?;
            let expected_phase = expected
                .phase
                .as_ref()
                .ok_or(LcRelationError::Observation)?;
            let proposed_phase = proposed
                .phase
                .as_ref()
                .ok_or(LcRelationError::Observation)?;
            equal(expected_phase, proposed_phase, budget)?;
            if expected.cycle != proposed.cycle {
                return Err(LcRelationError::Cycle);
            }
        }
        Ok(())
    }

    /// Before/after work and storage on the caller's existing ledger.
    #[must_use]
    pub const fn usage(&self) -> [BudgetUsage; 2] {
        self.usage
    }
}

/// Scaling both elements and energy by `a/a'` preserves all LC laws, with unchanged amplitudes.
/// The checked integer timestamp lattice preserves every original modulo phase and exact time.
pub(super) fn check(
    cover: &LcAliasCover,
    index: &Rational,
    parameters: &LcPairParameters,
    budget: &mut Budget,
) -> Result<LcAliasWitness, LcRelationError> {
    let before = budget.usage();
    let base = cover.0.fundamental.witness(parameters, budget)?;
    let mut values = base.values().clone();
    let shift = cover.0.step.mul(&index.clone().into(), budget)?;
    values.rate = values.rate.add(&shift, budget)?;
    let factor = base.values().rate.div(&values.rate, budget)?;
    if factor.positive().is_none() {
        return Err(LcRelationError::Positivity);
    }
    values.inductance = values.inductance.mul(&factor, budget)?;
    values.capacitance = values.capacitance.mul(&factor, budget)?;
    values.energy = values.energy.mul(&factor, budget)?;
    for (observation, tick) in values.observations.iter_mut().zip(&cover.0.ticks) {
        let cycle = observation
            .cycle
            .as_ref()
            .ok_or(LcRelationError::Observation)?;
        observation.cycle = Some(cycle.add(&tick.mul(index, budget)?, budget)?);
    }
    let mut bytes = base
        .retained_payload_bytes()
        .saturating_mul(2)
        .saturating_add(4_096);
    for value in [
        &values.rate,
        &values.inductance,
        &values.capacitance,
        &values.energy,
        &factor,
    ] {
        bytes = bytes.saturating_add(storage::constant(value));
    }
    storage::retained(bytes.saturating_add(cover.retained_payload_bytes()), budget)?;
    Ok(LcAliasWitness {
        relation: cover.0.relation.clone(),
        values,
        base,
        factor,
        index: index.clone(),
        usage: [before, budget.usage()],
    })
}

/// Exact structural equality authenticates every proposed coordinate of the preservation map.
fn equal(
    expected: &ExactConstant,
    proposed: &ExactConstant,
    budget: &mut Budget,
) -> Result<(), LcRelationError> {
    if !expected.structurally_equal(proposed, budget)? {
        return Err(LcRelationError::Equation);
    }
    Ok(())
}
