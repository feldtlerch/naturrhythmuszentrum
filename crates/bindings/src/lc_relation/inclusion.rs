// SPDX-License-Identifier: AGPL-3.0-only
//! Checked input-family inclusion preserves original clocks, coordinates and constraints.

use super::{AT, LcInput, LcRelation, LcRelationError, ordered, storage};
use crate::Budget;
use nrz_cas::limits::BudgetUsage;
use std::rc::Rc;

/// A sufficient exact inclusion certificate, independent of any subsequent solver budget.
#[derive(Clone, Debug)]
pub struct LcInputRefinement(Rc<Data>);

/// Every old constraint has a contained new counterpart with the same physical meaning.
#[derive(Debug)]
struct Data {
    /// Original question; allocation identity prevents an equal-value replacement.
    previous: LcRelation,
    /// Complete refined original question, including all added constraints and observations.
    refined: LcRelation,
    /// One checked new input position for each original input, in original order.
    counterparts: Vec<usize>,
    /// Exact inclusion-check snapshots on the caller's cumulative ledger.
    usage: [BudgetUsage; 2],
    /// Conservative new evidence ownership, excluding the shared original declarations.
    bytes: u64,
}

impl LcRelation {
    /// Check a compatible tightening or added-pair family without solving it.
    ///
    /// Each old input must have one same-quantity new interval contained in it. This
    /// sufficient closed test refuses more general implication rather than guessing it.
    /// Existing observation positions keep their identity and the common epoch must match.
    /// # Errors
    /// Refuses changed clocks, dropped/weakened constraints and exhausted resources.
    pub fn refinement_inputs(
        &self,
        refined: &Self,
        budget: &mut Budget,
    ) -> Result<LcInputRefinement, LcRelationError> {
        let before = budget.usage();
        budget.work(1, AT)?;
        if self.epoch() != refined.epoch() || self.observations() > refined.observations() {
            return Err(LcRelationError::Containment);
        }
        let bytes = storage::size(self.inputs().len())?
            .saturating_mul(16)
            .saturating_add(512);
        storage::retained(
            bytes
                .saturating_add(self.retained_payload_bytes())
                .saturating_add(refined.retained_payload_bytes()),
            budget,
        )?;
        let mut counterparts = Vec::with_capacity(self.inputs().len());
        for previous in self.inputs() {
            let mut found = None;
            for (index, next) in refined.inputs().iter().enumerate() {
                budget.work(1, AT)?;
                if contained(previous, next, budget)? {
                    found = Some(index);
                    break;
                }
            }
            counterparts.push(found.ok_or(LcRelationError::Containment)?);
        }
        Ok(LcInputRefinement(Rc::new(Data {
            previous: self.clone(),
            refined: refined.clone(),
            counterparts,
            usage: [before, budget.usage()],
            bytes,
        })))
    }
}

impl LcInputRefinement {
    /// Original question whose information remains valid on the refined family.
    #[must_use]
    pub fn previous(&self) -> &LcRelation {
        &self.0.previous
    }

    /// Complete new question; an unresolved solver cannot discard its candidate domain.
    #[must_use]
    pub fn refined(&self) -> &LcRelation {
        &self.0.refined
    }

    /// Checked counterpart index for every previous input in its original order.
    #[must_use]
    pub fn counterparts(&self) -> &[usize] {
        &self.0.counterparts
    }

    /// Before/after input-family implication checking work.
    #[must_use]
    pub fn usage(&self) -> [BudgetUsage; 2] {
        self.0.usage
    }

    /// Conservative new exact evidence ownership.
    #[must_use]
    pub fn retained_payload_bytes(&self) -> u64 {
        self.0.bytes
    }
}

/// Declared finite endpoints require exact order; a missing bound stays truly unbounded.
fn contained(
    previous: &LcInput,
    next: &LcInput,
    budget: &mut Budget,
) -> Result<bool, LcRelationError> {
    budget.work(1, AT)?;
    if previous.quantity() != next.quantity() || previous.descriptor() != next.descriptor() {
        return Ok(false);
    }
    let lower = match (previous.interval().lower(), next.interval().lower()) {
        (None, _) => true,
        (Some(_), None) => false,
        (Some(old), Some(new)) => ordered(old, new, budget)?,
    };
    let upper = match (previous.interval().upper(), next.interval().upper()) {
        (None, _) => true,
        (Some(_), None) => false,
        (Some(old), Some(new)) => ordered(new, old, budget)?,
    };
    Ok(lower && upper)
}
