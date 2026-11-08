// SPDX-License-Identifier: AGPL-3.0-only
//! Cell axes and component strides are immutable, owner-qualified layout occupations.

use super::{
    CellAxis, CellCorrespondence, CellLayoutVersion, MaskError, charge, memory,
    storage::LayoutRecord,
};
use crate::{Heap, HeapError, Layout, heap::identity};
use nrz_bindings::Budget;

impl Heap {
    /// Declare a fresh ordered native cell-axis association, with no per-cell allocation.
    /// # Errors
    /// Refuses identity exhaustion or the caller's Work limit.
    pub fn create_cell_axis(
        &self,
        capacity: usize,
        budget: &mut Budget,
    ) -> Result<CellAxis, MaskError> {
        budget.with_work_phase(super::FAMILY, "cell axis", |budget| {
            charge(budget, 1)?;
            Ok(CellAxis {
                scope: self.scope,
                identity: identity::identity()?,
                capacity,
            })
        })
    }

    /// Publish a checked cell-row/component-column map for one exact ordered axis.
    ///
    /// Component count must be positive; cell capacity may be zero. Equal maps
    /// published independently remain distinct layout occupations.
    /// # Errors
    /// Refuses a foreign axis, capacity mismatch, invalid map or resource exhaustion.
    pub fn create_cell_layout(
        &mut self,
        axis: CellAxis,
        components: Layout,
        budget: &mut Budget,
    ) -> Result<CellLayoutVersion, MaskError> {
        budget.with_work_phase(super::FAMILY, "cell layout publication", |budget| {
            self.publish_cell_layout(axis, components, None, budget)
        })
    }

    /// Replace a component map atomically; every mask naming the old layout becomes stale.
    ///
    /// The axis and cell capacity stay fixed. Mask-only native storage has no reader
    /// pins; this operation does not grant authority to replace numerical regions.
    /// # Errors
    /// Refuses stale layouts, mismatched extents, counter exhaustion and resource limits.
    pub fn replace_cell_layout(
        &mut self,
        previous: CellLayoutVersion,
        components: Layout,
        budget: &mut Budget,
    ) -> Result<CellLayoutVersion, MaskError> {
        budget.with_work_phase(super::FAMILY, "cell layout replacement", |budget| {
            charge(budget, 1)?;
            let axis = self.masks.layout(self.scope, previous)?.axis;
            self.masks.layouts.get(previous.key)?.unprotected()?;
            self.publish_cell_layout(axis, components, Some(previous), budget)
        })
    }

    /// Retire a native cell layout and invalidate masks that name its occupation.
    ///
    /// Retired masks can be released independently with `Heap::retire_mask`.
    /// This native metadata operation grants no numerical region retirement authority.
    /// # Errors
    /// Refuses foreign/stale layouts and resource exhaustion before changing any occupation.
    pub fn retire_cell_layout(
        &mut self,
        version: CellLayoutVersion,
        budget: &mut Budget,
    ) -> Result<(), MaskError> {
        budget.with_work_phase(super::FAMILY, "cell layout retirement", |budget| {
            charge(budget, 1)?;
            self.masks.layout(self.scope, version)?;
            self.masks.layouts.get(version.key)?.unprotected()?;
            let reservation =
                memory::Reservation::new(self, None, None, 0, [&[version.key], &[]], budget)?;
            reservation.run(budget, |budget| {
                charge(budget, self.masks.layouts.scan_work())?;
                let plan = self.masks.layouts.change_reserved(
                    0,
                    &[version.key],
                    self.limits.descriptors,
                )?;
                let prepared = plan.bind(Vec::<LayoutRecord>::new())?;
                self.masks.layouts.publish(prepared);
                Ok(())
            })
        })
    }

    /// Common preparation preserves old layout and masks until every check succeeds.
    fn publish_cell_layout(
        &mut self,
        axis: CellAxis,
        components: Layout,
        previous: Option<CellLayoutVersion>,
        budget: &mut Budget,
    ) -> Result<CellLayoutVersion, MaskError> {
        charge(budget, 1)?;
        if axis.scope != self.scope {
            return Err(HeapError::ForeignHandle.into());
        }
        if components.extent().rows() != axis.capacity || components.extent().columns() == 0 {
            return Err(MaskError::LayoutMismatch);
        }
        charge(budget, components.validation_work())?;
        components.validate(components.region_entries(), self.limits.layout_work)?;
        let original_map = previous
            .map(|version| self.masks.layout(self.scope, version))
            .transpose()?
            .and_then(|layout| layout.gather.as_ref());
        let map_entries = original_map.map_or(0, |map| map.indices.len());
        let retired = previous.map(|version| version.key);
        let retired = retired.as_slice();
        let reservation = memory::Reservation::new(
            self,
            Some(components.owned_bytes()),
            None,
            map_entries,
            [retired, &[]],
            budget,
        )?;
        reservation.run(budget, |budget| {
            let gather = if let Some(version) = previous {
                self.masks
                    .layout(self.scope, version)?
                    .gather
                    .as_ref()
                    .map(|map| copy_gather(map, budget))
                    .transpose()?
            } else {
                None
            };
            charge(budget, self.masks.layouts.scan_work())?;
            let plan = self
                .masks
                .layouts
                .change_reserved(1, retired, self.limits.descriptors)?;
            let key = *plan.keys().first().ok_or(HeapError::Capacity)?;
            let mut records = memory::vector(1)?;
            records.push(super::storage::Native::from(LayoutRecord {
                axis,
                components,
                gather,
            }));
            let prepared = plan.bind(records)?;
            self.masks.layouts.publish(prepared);
            Ok(CellLayoutVersion {
                scope: self.scope,
                key,
            })
        })
    }

    /// Validate the identity correspondence between two current layout versions.
    ///
    /// General pullback and pushforward through distinct layouts return
    /// `MaskError::NonIdentityCorrespondence`; equal dimensions or strides are insufficient.
    /// # Errors
    /// Refuses foreign/stale versions, non-identity correspondence and Work exhaustion.
    pub fn mask_correspondence(
        &self,
        source: CellLayoutVersion,
        target: CellLayoutVersion,
        budget: &mut Budget,
    ) -> Result<CellCorrespondence, MaskError> {
        budget.with_work_phase(super::FAMILY, "mask correspondence", |budget| {
            charge(budget, 2)?;
            self.masks.layout(self.scope, source)?;
            self.masks.layout(self.scope, target)?;
            if source != target {
                return Err(MaskError::NonIdentityCorrespondence);
            }
            Ok(CellCorrespondence { layout: source })
        })
    }
}

/// Stride replacement preserves bounded original-association metadata under its new reservation.
fn copy_gather(
    map: &super::CellGather,
    budget: &mut Budget,
) -> Result<super::CellGather, MaskError> {
    let mut indices = memory::vector(map.indices.len())?;
    for &index in &map.indices {
        charge(budget, 1)?;
        indices.push(index);
    }
    Ok(super::CellGather {
        parent: map.parent,
        source: map.source,
        axis: map.axis,
        indices,
    })
}
