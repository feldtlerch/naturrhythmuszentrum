// SPDX-License-Identifier: AGPL-3.0-only
//! Historical outcome tables are distinct from revalidated numerical access.

use super::{
    LcDrivenContinuations, LcDrivenError, LcDrivenInventory, LcDrivenPoint, LcDrivenStatus,
    LcDrivenStratum,
};
use crate::{Heap, LcSpectralBatch, PublishedPhasorSolve};
use nrz_bindings::{AxisPhaseOutcome, Budget, readings::algebra::limits::BudgetUsage};
use nrz_scalar::{ArithmeticContext, Enclosure, Scalar};

impl<P: Scalar> LcDrivenContinuations<P> {
    /// Original invariant producer, preserving its independent successful rows and receipts.
    #[must_use]
    pub const fn invariants(&self) -> &LcSpectralBatch<P> {
        &self.invariants
    }

    /// Stable original point identities, including every refused or unavailable row.
    pub fn point_keys(&self) -> impl ExactSizeIterator<Item = u64> + '_ {
        self.rows.iter().map(|row| row.plan.point)
    }

    /// Complete original current slots; inspecting declarations exposes no numerical value.
    #[must_use]
    pub fn point(&self, row: usize) -> Option<&LcDrivenPoint<P>> {
        self.rows.get(row).map(|row| &row.plan)
    }

    /// Historical decision, current and phase statuses, in that order.
    #[must_use]
    pub fn statuses(&self, row: usize) -> Option<[LcDrivenStatus; 3]> {
        self.rows.get(row).map(|row| row.status)
    }

    /// Historical exact verdict, separate from a current live attachment.
    #[must_use]
    pub fn verdict(&self, row: usize) -> Option<AxisPhaseOutcome> {
        self.rows.get(row)?.verdict
    }

    /// The declared unresolved continuation has zero remaining refinement attempts.
    #[must_use]
    pub fn continuation(&self, row: usize) -> Option<&'static str> {
        (self.verdict(row)? == AxisPhaseOutcome::Undecided).then_some("retain-unresolved-reactance")
    }

    /// Original typed failure for a driven node; invariant failures remain with their owner.
    #[must_use]
    pub fn failure(&self, row: usize, stratum: LcDrivenStratum) -> Option<&LcDrivenError> {
        self.rows
            .get(row)?
            .failure
            .get(stratum.index().checked_sub(1)?)?
            .as_ref()
    }

    /// Each stratum's actual caller snapshots, including consumed failure prefixes.
    #[must_use]
    pub const fn usage(&self, stratum: LcDrivenStratum) -> Option<[BudgetUsage; 2]> {
        self.usage[stratum.index()]
    }

    /// Scoped new table ownership only; no shared child or backend aggregate is implied.
    #[must_use]
    pub const fn inventory(&self) -> &LcDrivenInventory {
        &self.inventory
    }
}

impl<P: Scalar + 'static> LcDrivenContinuations<P> {
    /// Borrow a complete published current only after validating its decision and all outputs.
    /// # Errors
    /// Refuses unavailable rows, stale original inputs/companions, foreign owners and resources.
    pub fn current<'a>(
        &'a self,
        row: usize,
        heap: &Heap,
        context: &ArithmeticContext,
        budget: &mut Budget,
    ) -> Result<&'a PublishedPhasorSolve<P>, LcDrivenError> {
        self.invariants.check_owner(heap, context)?;
        let row = self.rows.get(row).ok_or(LcDrivenError::Identity)?;
        row.selection
            .as_ref()
            .ok_or(LcDrivenError::Identity)?
            .current(heap, context, budget)?;
        let current = row.current.as_ref().ok_or(LcDrivenError::Identity)?;
        for reading in current.readings().bindings() {
            heap.validate_reading(reading)
                .map_err(crate::PhasorError::from)?;
        }
        Ok(current)
    }

    /// Read a selected phase enclosure only after its declared later stratum has run.
    ///
    /// This is external phase evidence, not a heap angle-value publication.
    /// # Errors
    /// Refuses pending/failed rows, undefined or undecided phase, stale sources and resources.
    pub fn phase(
        &self,
        row: usize,
        heap: &Heap,
        context: &ArithmeticContext,
        budget: &mut Budget,
    ) -> Result<Enclosure<P>, LcDrivenError> {
        self.invariants.check_owner(heap, context)?;
        let row = self.rows.get(row).ok_or(LcDrivenError::Identity)?;
        if !matches!(
            row.status[2],
            LcDrivenStatus::Successful | LcDrivenStatus::Undefined | LcDrivenStatus::Undecided
        ) {
            return Err(LcDrivenError::Identity);
        }
        Ok(row
            .selection
            .as_ref()
            .ok_or(LcDrivenError::Identity)?
            .phase(heap, context, budget)?)
    }
}
