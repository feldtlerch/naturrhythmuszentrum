// SPDX-License-Identifier: AGPL-3.0-only
//! Native word algebra follows a revalidated identity cell correspondence.

use super::{
    CellCorrespondence, MaskError, MaskVersion, charge, clear_padding, memory, storage::MaskRecord,
    word_count,
};
use crate::Heap;
use nrz_bindings::Budget;

/// Distinct graph roles in a decision's declared demand partition.
#[derive(Clone, Copy, Debug)]
pub struct MaskPartition<'a> {
    /// Settled option or domain masks.
    pub selected: &'a [MaskVersion],
    /// Unresolved demand, which grants no complement membership.
    pub undecided: &'a [MaskVersion],
    /// Explicit refusal coverage, with reasons retained by the calling graph.
    pub refusal: &'a [MaskVersion],
}

impl MaskPartition<'_> {
    /// Preserve role order while validating every participating native allocation.
    fn versions(self) -> impl Iterator<Item = MaskVersion> {
        self.selected
            .iter()
            .chain(self.undecided)
            .chain(self.refusal)
            .copied()
    }
}

/// A closed word law cannot manufacture a correspondence or reinterpret padding.
#[derive(Clone, Copy)]
enum Binary {
    /// Common participating cells.
    Intersection,
    /// Combined participating cells.
    Union,
    /// Relative complement within the supplied filled set.
    Complement,
}

impl Binary {
    /// Static observation vocabulary.
    const fn name(self) -> &'static str {
        match self {
            Self::Intersection => "mask intersection",
            Self::Union => "mask union",
            Self::Complement => "mask complement",
        }
    }

    /// One native word step after the layout checks.
    const fn word(self, left: u64, right: u64) -> u64 {
        match self {
            Self::Intersection => left & right,
            Self::Union => left | right,
            Self::Complement => left & !right,
        }
    }
}

impl Heap {
    /// Publish the intersection under one checked cell correspondence.
    /// # Errors
    /// Refuses foreign, stale or unrelated layouts and exhausted native resources.
    pub fn intersect_masks(
        &mut self,
        correspondence: CellCorrespondence,
        left: MaskVersion,
        right: MaskVersion,
        budget: &mut Budget,
    ) -> Result<MaskVersion, MaskError> {
        self.combine_masks(correspondence, left, right, Binary::Intersection, budget)
    }

    /// Publish the union under one checked cell correspondence.
    /// # Errors
    /// Refuses foreign, stale or unrelated layouts and exhausted native resources.
    pub fn union_masks(
        &mut self,
        correspondence: CellCorrespondence,
        left: MaskVersion,
        right: MaskVersion,
        budget: &mut Budget,
    ) -> Result<MaskVersion, MaskError> {
        self.combine_masks(correspondence, left, right, Binary::Union, budget)
    }

    /// Publish `filled & !excluded`; cells outside the participating filled set stay clear.
    /// # Errors
    /// Refuses foreign, stale or unrelated layouts and exhausted native resources.
    pub fn complement_mask(
        &mut self,
        correspondence: CellCorrespondence,
        filled: MaskVersion,
        excluded: MaskVersion,
        budget: &mut Budget,
    ) -> Result<MaskVersion, MaskError> {
        self.combine_masks(correspondence, filled, excluded, Binary::Complement, budget)
    }

    /// Validate both operands before allocating an independent immutable result.
    fn combine_masks(
        &mut self,
        correspondence: CellCorrespondence,
        left: MaskVersion,
        right: MaskVersion,
        operation: Binary,
        budget: &mut Budget,
    ) -> Result<MaskVersion, MaskError> {
        budget.with_work_phase(super::FAMILY, operation.name(), |budget| {
            charge(budget, 2)?;
            let capacity = self.mask(left, correspondence.layout)?.axis().capacity();
            self.mask(right, correspondence.layout)?;
            let reservation = memory::Reservation::new(
                &self.masks,
                None,
                Some(word_count(capacity)),
                0,
                [0, 0],
                budget,
            )?;
            reservation.run(budget, |budget| {
                let mut words = memory::zeroed(word_count(capacity), budget)?;
                let left = self.mask(left, correspondence.layout)?;
                let right = self.mask(right, correspondence.layout)?;
                for ((output, left), right) in words.iter_mut().zip(left.words()).zip(right.words())
                {
                    charge(budget, 1)?;
                    *output = operation.word(*left, *right);
                }
                charge(budget, 1)?;
                clear_padding(&mut words, capacity);
                self.masks.publish_mask(
                    self.scope,
                    MaskRecord {
                        layout: correspondence.layout,
                        words,
                    },
                    self.limits.descriptors,
                    budget,
                )
            })
        })
    }

    /// Require pairwise disjoint outcomes whose union is exactly the declared filled demand.
    /// # Errors
    /// Refuses overlap, omitted or unfilled cells, unrelated/stale masks and Work exhaustion.
    pub fn check_mask_partition(
        &self,
        correspondence: CellCorrespondence,
        filled: MaskVersion,
        partition: MaskPartition<'_>,
        budget: &mut Budget,
    ) -> Result<(), MaskError> {
        budget.with_work_phase(super::FAMILY, "mask decision partition", |budget| {
            charge(budget, 1)?;
            let filled = self.mask(filled, correspondence.layout)?;
            for version in partition.versions() {
                charge(budget, 1)?;
                self.mask(version, correspondence.layout)?;
            }
            for (index, expected) in filled.words().iter().enumerate() {
                charge(budget, 1)?;
                let mut union = 0_u64;
                for version in partition.versions() {
                    charge(budget, 1)?;
                    let mask = self.mask(version, correspondence.layout)?;
                    let word = *mask.words().get(index).ok_or(MaskError::LayoutMismatch)?;
                    if union & word != 0 {
                        return Err(MaskError::PartitionOverlap);
                    }
                    union |= word;
                }
                if union != *expected {
                    return Err(MaskError::PartitionCoverage);
                }
            }
            Ok(())
        })
    }

    /// Publish pointwise availability as the intersection of every required input's filled mask.
    ///
    /// Missing cells remain unavailable. This does not admit a reduction, contraction,
    /// stencil or singleton broadcast without its separately checked incidence law.
    /// # Errors
    /// Refuses no inputs, unrelated/stale layouts and exhausted native resources.
    pub fn intersect_mask_demand(
        &mut self,
        correspondence: CellCorrespondence,
        inputs: &[MaskVersion],
        budget: &mut Budget,
    ) -> Result<MaskVersion, MaskError> {
        budget.with_work_phase(super::FAMILY, "mask demand intersection", |budget| {
            if inputs.is_empty() {
                return Err(MaskError::MissingInputs);
            }
            charge(budget, 1)?;
            let capacity = self
                .masks
                .layout(self.scope, correspondence.layout)?
                .axis
                .capacity;
            for &input in inputs {
                charge(budget, 1)?;
                self.mask(input, correspondence.layout)?;
            }
            let reservation = memory::Reservation::new(
                &self.masks,
                None,
                Some(word_count(capacity)),
                0,
                [0, 0],
                budget,
            )?;
            reservation.run(budget, |budget| {
                let mut words = memory::zeroed(word_count(capacity), budget)?;
                for (index, output) in words.iter_mut().enumerate() {
                    charge(budget, 1)?;
                    let mut word = u64::MAX;
                    for &input in inputs {
                        charge(budget, 1)?;
                        let mask = self.mask(input, correspondence.layout)?;
                        word &= mask.words().get(index).ok_or(MaskError::LayoutMismatch)?;
                    }
                    *output = word;
                }
                charge(budget, 1)?;
                clear_padding(&mut words, capacity);
                self.masks.publish_mask(
                    self.scope,
                    MaskRecord {
                        layout: correspondence.layout,
                        words,
                    },
                    self.limits.descriptors,
                    budget,
                )
            })
        })
    }
}
