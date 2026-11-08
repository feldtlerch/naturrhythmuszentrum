// SPDX-License-Identifier: AGPL-3.0-only
//! Compaction publishes a dense filled mask only after complete injective native movement.

use super::{
    CellAxis, CellCorrespondence, CellGather, CellLayoutVersion, MaskError, MaskVersion, charge,
    clear_padding, memory,
    storage::{LayoutRecord, MaskRecord, add_population},
    word_count,
};
use crate::{Extent, Heap, HeapError, Layout, heap::identity};
use nrz_bindings::Budget;

impl Heap {
    /// Compact a branch into a fresh mask and a bounded new-to-original gather edge.
    ///
    /// The first population-many destination cells are filled; unused capacity is
    /// clear. `MaskView::gather` exposes the validated original-index map. Only mask
    /// movement is published: floating components and companion movement remain separate.
    /// # Errors
    /// Refuses stale/unrelated layouts, insufficient destination or map capacity,
    /// count overflow, incomplete movement and resource exhaustion without publication.
    pub fn compact_mask(
        &mut self,
        correspondence: CellCorrespondence,
        branch: MaskVersion,
        capacity: usize,
        map_limit: usize,
        budget: &mut Budget,
    ) -> Result<MaskVersion, MaskError> {
        budget.with_work_phase(super::FAMILY, "mask compaction", |budget| {
            charge(budget, 1)?;
            let source = self.mask(branch, correspondence.layout)?;
            let components = source.components().extent().columns();
            let population = self.mask_population(branch, budget)?;
            if population > capacity {
                return Err(MaskError::CompactionCapacity);
            }
            let limit = map_limit;
            if population > limit {
                return Err(MaskError::MapCapacity {
                    required: population,
                    limit,
                });
            }
            let components = Layout::contiguous(Extent::new(capacity, components)?)?;
            let reservation = memory::Reservation::new(
                self,
                Some(0),
                Some(word_count(capacity)),
                population,
                [&[], &[]],
                budget,
            )?;
            reservation.run(budget, |budget| {
                let mut words = memory::zeroed(word_count(capacity), budget)?;
                let mut indices = memory::vector(population)?;
                gather(
                    self.mask(branch, correspondence.layout)?.words(),
                    &mut indices,
                    population,
                    budget,
                )?;
                fill_prefix(&mut words, population, budget)?;
                charge(budget, 1)?;
                clear_padding(&mut words, capacity);
                validate_movement(
                    self.mask(branch, correspondence.layout)?.words(),
                    &indices,
                    &words,
                    population,
                    budget,
                )?;
                let mapping = original_map(
                    &self.mask(branch, correspondence.layout)?,
                    branch,
                    indices,
                    budget,
                )?;
                charge(budget, 1)?;
                let destination_axis = CellAxis {
                    scope: self.scope,
                    identity: identity::identity()?,
                    capacity,
                };
                self.publish_compacted_mask(
                    LayoutRecord {
                        axis: destination_axis,
                        components,
                        gather: Some(mapping),
                    },
                    words,
                    budget,
                )
            })
        })
    }

    /// Bind both complete occupations before either can become visible.
    fn publish_compacted_mask(
        &mut self,
        layout: LayoutRecord,
        words: Vec<u64>,
        budget: &mut Budget,
    ) -> Result<MaskVersion, MaskError> {
        charge(
            budget,
            memory::add(self.masks.layouts.scan_work(), self.masks.masks.scan_work())?,
        )?;
        let layouts = self
            .masks
            .layouts
            .change_reserved(1, &[], self.limits.descriptors)?;
        let masks = self
            .masks
            .masks
            .change_reserved(1, &[], self.limits.descriptors)?;
        let layout_version = CellLayoutVersion {
            scope: self.scope,
            key: *layouts.keys().first().ok_or(HeapError::Capacity)?,
        };
        let mask_version = MaskVersion {
            scope: self.scope,
            key: *masks.keys().first().ok_or(HeapError::Capacity)?,
            layout: layout_version,
        };
        let mut layout_records = memory::vector(1)?;
        layout_records.push(super::storage::Native::from(layout));
        let mut mask_records = memory::vector(1)?;
        mask_records.push(super::storage::Native::from(MaskRecord {
            layout: layout_version,
            words,
        }));
        let layouts = layouts.bind(layout_records)?;
        let masks = masks.bind(mask_records)?;
        self.masks.layouts.publish(layouts);
        self.masks.masks.publish(masks);
        Ok(mask_version)
    }
}

/// Checked word-prefix counts fix each subsequent compacted position before emission.
fn gather(
    words: &[u64],
    indices: &mut Vec<usize>,
    population: usize,
    budget: &mut Budget,
) -> Result<(), MaskError> {
    let mut prefix = 0_usize;
    for (position, &word) in words.iter().enumerate() {
        charge(budget, 1)?;
        let end = add_population(prefix, word.count_ones())?;
        let mut remaining = word;
        while remaining != 0 {
            charge(budget, 1)?;
            if indices.len() >= population {
                return Err(MaskError::IncompleteMovement);
            }
            let bit = usize::try_from(remaining.trailing_zeros())
                .map_err(|_| MaskError::CountOverflow)?;
            let index = position
                .checked_mul(64)
                .and_then(|base| base.checked_add(bit))
                .ok_or(MaskError::CountOverflow)?;
            indices.push(index);
            remaining &= remaining - 1;
        }
        if indices.len() != end {
            return Err(MaskError::IncompleteMovement);
        }
        prefix = end;
    }
    if prefix != population {
        return Err(MaskError::IncompleteMovement);
    }
    Ok(())
}

/// Dense destination population is represented by a bounded prefix, including empty output.
fn fill_prefix(words: &mut [u64], population: usize, budget: &mut Budget) -> Result<(), MaskError> {
    for (position, word) in words.iter_mut().enumerate() {
        charge(budget, 1)?;
        *word = prefix_word(position, population)?;
    }
    Ok(())
}

/// Compute the expected word without shifting by 64 or overflowing an address.
fn prefix_word(position: usize, population: usize) -> Result<u64, MaskError> {
    let base = position.checked_mul(64).ok_or(MaskError::CountOverflow)?;
    let bits = population.saturating_sub(base).min(64);
    Ok(if bits == 64 {
        u64::MAX
    } else {
        (1_u64 << bits) - 1
    })
}

/// Independently check range, strict order, selection membership and complete mask movement.
pub(super) fn validate_movement(
    source: &[u64],
    indices: &[usize],
    destination: &[u64],
    population: usize,
    budget: &mut Budget,
) -> Result<(), MaskError> {
    if indices.len() != population {
        return Err(MaskError::IncompleteMovement);
    }
    let mut previous = None;
    for &index in indices {
        charge(budget, 1)?;
        if previous.is_some_and(|value| value >= index) {
            return Err(MaskError::IncompleteMovement);
        }
        let word = source
            .get(index / 64)
            .ok_or(MaskError::IncompleteMovement)?;
        if word & (1_u64 << (index % 64)) == 0 {
            return Err(MaskError::IncompleteMovement);
        }
        previous = Some(index);
    }
    let actual = super::storage::population(source, budget)?;
    if actual != population {
        return Err(MaskError::IncompleteMovement);
    }
    let mut moved = 0_usize;
    for (position, &word) in destination.iter().enumerate() {
        charge(budget, 1)?;
        if word != prefix_word(position, population)? {
            return Err(MaskError::IncompleteMovement);
        }
        moved = add_population(moved, word.count_ones())?;
    }
    if moved != population {
        return Err(MaskError::IncompleteMovement);
    }
    Ok(())
}

/// Compose only previously validated compaction edges, retaining the original axis and occupation.
fn original_map(
    source: &super::MaskView<'_>,
    parent: MaskVersion,
    mut indices: Vec<usize>,
    budget: &mut Budget,
) -> Result<CellGather, MaskError> {
    let Some(original) = source.gather() else {
        return Ok(CellGather {
            parent,
            source: parent,
            axis: source.axis(),
            indices,
        });
    };
    let mut previous = None;
    for index in &mut indices {
        charge(budget, 1)?;
        let mapped = *original
            .indices
            .get(*index)
            .ok_or(MaskError::IncompleteMovement)?;
        if previous.is_some_and(|value| value >= mapped) {
            return Err(MaskError::IncompleteMovement);
        }
        *index = mapped;
        previous = Some(mapped);
    }
    Ok(CellGather {
        parent,
        source: original.source,
        axis: original.axis,
        indices,
    })
}
