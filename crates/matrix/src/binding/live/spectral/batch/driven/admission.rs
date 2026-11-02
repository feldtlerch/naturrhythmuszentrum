// SPDX-License-Identifier: AGPL-3.0-only
//! Complete immutable point identities and continuation slots precede every execution.

use super::{
    AT, LcDrivenContinuations, LcDrivenError, LcDrivenInventory, LcDrivenLimits, LcDrivenPoint, Row,
};
use crate::{AxisPhasePlan, Heap, HeapError, LcSpectralBatch, PhasorSolvePublication};
use nrz_bindings::{
    AxisPhaseImpedance, Budget,
    readings::algebra::{decision::RationalEnclosure, limits::BudgetKind},
};
use nrz_scalar::{ArithmeticContext, Scalar};

impl<P: Scalar + 'static> LcDrivenPoint<P> {
    /// Admit one original equation, sign question, certificate and complete current slot.
    /// # Errors
    /// Refuses mismatched exact impedance/phase targets or exhausted caller resources.
    pub fn new(
        point: u64,
        current: PhasorSolvePublication<P>,
        phase: AxisPhasePlan<P>,
        enclosure: RationalEnclosure,
        budget: &mut Budget,
    ) -> Result<Self, LcDrivenError> {
        let impedance = AxisPhaseImpedance::new(
            phase.case().clone(),
            current.solve().definition().clone(),
            budget,
        )
        .map_err(|cause| {
            LcDrivenError::Decision(Box::new(crate::LiveDecisionError::Admission(Box::new(
                cause,
            ))))
        })?;
        Ok(Self {
            point,
            current,
            phase,
            impedance,
            enclosure,
        })
    }

    /// Complete predeclared current inventory remains inspectable even at zero or undecided.
    #[must_use]
    pub const fn current_publication(&self) -> &PhasorSolvePublication<P> {
        &self.current
    }
}

impl<P: Scalar + 'static> LcDrivenContinuations<P> {
    /// Attach finite later strata to an unattempted autonomous owner in the same heap.
    ///
    /// Point order and identities must exactly match. Each point declares one sign attempt
    /// and the named unresolved continuation; there is no implicit retry capacity.
    /// The point and table-byte limits bound only this new table, not shared child owners.
    /// # Errors
    /// Refuses used/foreign invariant owners, mismatched rows, stale inputs and capacities.
    pub fn prepare(
        invariants: LcSpectralBatch<P>,
        points: Vec<LcDrivenPoint<P>>,
        heap: &Heap,
        context: &ArithmeticContext,
        limits: LcDrivenLimits,
        budget: &mut Budget,
    ) -> Result<Self, LcDrivenError> {
        invariants.check_owner(heap, context)?;
        if invariants.attempted
            || points.len() > limits.points
            || points.len() != invariants.rows.len()
        {
            return Err(LcDrivenError::Identity);
        }
        let logical = bytes::<P>(points.len())?;
        if logical > limits.table_bytes {
            return Err(HeapError::Capacity.into());
        }
        admit(logical, budget)?;
        let mut rows = Vec::new();
        rows.try_reserve_exact(points.len())
            .map_err(|_| HeapError::Capacity)?;
        let observed = bytes::<P>(rows.capacity())?;
        if observed > limits.table_bytes {
            return Err(HeapError::Capacity.into());
        }
        admit(observed, budget)?;
        for (point, expected) in points.into_iter().zip(invariants.point_keys()) {
            budget.work(1, AT)?;
            if point.point != expected {
                return Err(LcDrivenError::Identity);
            }
            for operand in point.current.solve().operands() {
                for reading in std::iter::once(operand.whole()).chain(operand.copies()) {
                    heap.validate_reading(reading)
                        .map_err(crate::PhasorError::from)?;
                }
            }
            rows.push(Row {
                plan: point,
                status: [super::LcDrivenStatus::Pending; 3],
                verdict: None,
                selection: None,
                current: None,
                failure: [None, None, None],
            });
        }
        let inventory = LcDrivenInventory {
            logical_payload_bytes: logical,
            observed_capacity_bytes: observed,
            points: rows.len(),
        };
        Ok(Self {
            invariants,
            rows,
            next: 0,
            stopped: false,
            usage: [None; 4],
            inventory,
        })
    }
}

/// Count the new fixed wrapper and row storage, excluding the embedded original invariant owner.
fn bytes<P: Scalar>(points: usize) -> Result<u64, HeapError> {
    let bytes = points
        .checked_mul(size_of::<Row<P>>())
        .and_then(|bytes| {
            bytes.checked_add(
                size_of::<LcDrivenContinuations<P>>() - size_of::<LcSpectralBatch<P>>(),
            )
        })
        .ok_or(HeapError::ExtentOverflow)?;
    u64::try_from(bytes).map_err(|_| HeapError::ExtentOverflow)
}

/// Both staging and retained ceilings apply to this scoped table even after prior success.
pub(super) fn admit(bytes: u64, budget: &Budget) -> Result<(), LcDrivenError> {
    budget.check(BudgetKind::StagedPayloadBytes, bytes, AT)?;
    budget.check(BudgetKind::OutputBytes, bytes, AT)?;
    Ok(())
}
