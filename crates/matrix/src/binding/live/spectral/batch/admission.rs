// SPDX-License-Identifier: AGPL-3.0-only
//! Original schema authority and full immutable selection references define each local key.

use super::{
    AT, ArithmeticContext, Budget, Entry, Header, Heap, HeapError, Key, LcSpectralBatch,
    LcSpectralBatchLimits, LcSpectralSchema, LcSpectralStatus, LcSpectrum, Row, Scalar,
    SpectralError, coefficients::Coefficients, storage,
};
use std::rc::Rc;

impl<P: Scalar + 'static> LcSpectralBatch<P> {
    /// Admit a complete finite family before any numerical producer executes.
    ///
    /// Input plans must be deterministic instances of this exact schema. Original
    /// point keys are unique; repeated bindings are permitted and retain row order.
    /// The bound key bytes include the common header and every reserved entry slot.
    /// # Errors
    /// Refuses foreign schemas/heaps, stale inputs, duplicate point keys, full key
    /// capacity, checked-size overflow, allocation failure and cumulative exhaustion.
    pub fn prepare(
        schema: &LcSpectralSchema,
        heap: &Heap,
        points: &[(u64, LcSpectrum<P>)],
        limits: LcSpectralBatchLimits,
        context: &ArithmeticContext,
        budget: &mut Budget,
    ) -> Result<Self, SpectralError> {
        if points.len() > limits.points {
            return Err(HeapError::Capacity.into());
        }
        let capacity = points.len().min(limits.bindings);
        let mut inventory = storage::planned::<P>(points.len(), capacity, budget)?;
        if inventory.key_bytes > limits.key_bytes {
            return Err(HeapError::Capacity.into());
        }
        let mut rows = Vec::new();
        let mut entries = Vec::new();
        rows.try_reserve_exact(points.len())
            .map_err(|_| HeapError::Capacity)?;
        entries
            .try_reserve_exact(capacity)
            .map_err(|_| HeapError::Capacity)?;
        inventory.observed_capacity_bytes = storage::observed(&rows, &entries, budget)?;
        let header = Header {
            schema: schema.clone(),
            scope: heap.scope,
            precision: P::PRECISION,
            revision: context.revision(),
        };
        let mut owner = Self {
            header,
            rows,
            entries,
            inventory,
            coefficients: Coefficients::pending(),
            attempted: false,
            usage: None,
        };
        for (point, plan) in points {
            owner.admit(heap, *point, plan, capacity, budget)?;
        }
        Ok(owner)
    }

    /// Deterministic linear lookup meters every visited row, key and selected-reference comparison.
    fn admit(
        &mut self,
        heap: &Heap,
        point: u64,
        plan: &LcSpectrum<P>,
        capacity: usize,
        budget: &mut Budget,
    ) -> Result<(), SpectralError> {
        budget.work(1, AT)?;
        if !self.header.schema.authenticates(plan) {
            return Err(SpectralError::Identity);
        }
        plan.validate(heap)?;
        for row in &self.rows {
            budget.work(1, AT)?;
            if row.point == point {
                return Err(SpectralError::Identity);
            }
        }
        for (index, entry) in self.entries.iter().enumerate() {
            budget.work(1, AT)?;
            if same_inputs(&entry.key, plan, budget)? {
                self.rows.push(Row {
                    point,
                    binding: index,
                });
                return Ok(());
            }
        }
        if self.entries.len() == capacity {
            return Err(HeapError::Capacity.into());
        }
        let binding = self.entries.len();
        self.entries.push(Entry {
            key: Key {
                inputs: plan.inputs().clone(),
            },
            plan: plan.clone(),
            status: LcSpectralStatus::Pending,
            result: None,
            failure: None,
            blocked_by: None,
        });
        self.rows.push(Row { point, binding });
        Ok(())
    }
}

/// Sealed retained groups include all original maps, values, companions and occupation versions.
fn same_inputs<P: Scalar>(
    key: &Key<P>,
    plan: &LcSpectrum<P>,
    budget: &mut Budget,
) -> Result<bool, SpectralError> {
    for (left, right) in key.inputs.iter().zip(plan.inputs()) {
        budget.work(1, AT)?;
        if !Rc::ptr_eq(&left.group, &right.group) || left.index != right.index {
            return Ok(false);
        }
    }
    Ok(true)
}
