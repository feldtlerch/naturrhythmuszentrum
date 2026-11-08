// SPDX-License-Identifier: AGPL-3.0-only
//! Heap-owned mask allocations use the same slot occupations as numerical storage.

use super::{
    CellAxis, CellGather, CellLayoutVersion, MaskError, MaskVersion, MaskView, charge,
    clear_padding, memory, word_count,
};
use crate::{Heap, HeapError, Layout, heap::slots::Slots};
use nrz_bindings::Budget;
use std::rc::Rc;

/// One immutable native cell-to-component address rule.
#[derive(Debug)]
pub(crate) struct LayoutRecord {
    /// Logical cells and original ordered axis association.
    pub(crate) axis: CellAxis,
    /// Row is cell, column is component; both strides belong to this version.
    pub(crate) components: Layout,
    /// Original cell associations belong to the layout shared by all selection masks.
    pub(crate) gather: Option<CellGather>,
}

impl LayoutRecord {
    /// Component and gather capacities remain owned until the final layout header is destroyed.
    pub(super) fn owned_bytes(&self) -> Result<usize, MaskError> {
        let bytes = memory::add(
            crate::loan::rc_bytes::<Self>(),
            self.components.owned_bytes(),
        )?;
        memory::add(
            bytes,
            memory::bytes::<usize>(self.gather.as_ref().map_or(0, |map| map.indices.capacity()))?,
        )
    }
}

/// A separately owned packed allocation naming immutable layout metadata.
#[derive(Debug)]
pub(crate) struct MaskRecord {
    /// Layout occupation captured by the published version.
    pub(crate) layout: CellLayoutVersion,
    /// Zero-initialized, fixed-capacity native storage.
    pub(crate) words: Vec<u64>,
}

/// Mask storage belongs to the existing heap and uses its owner identity and limits.
#[derive(Debug)]
pub(crate) struct Store {
    /// Immutable native cell layouts, with nonwrapping occupation generations.
    pub(crate) layouts: Slots<Native<LayoutRecord>>,
    /// Immutable bit allocations; changed bits require a new occupation.
    pub(crate) masks: Slots<Native<MaskRecord>>,
}

impl Store {
    /// Empty storage allocates no table or payload.
    pub(crate) const fn new() -> Self {
        Self {
            layouts: Slots::new(),
            masks: Slots::new(),
        }
    }

    /// Account actual payload capacities, owner headers and retained table vacancies.
    pub(crate) fn owned_bytes(&self, budget: &mut Budget) -> Result<usize, MaskError> {
        charge(
            budget,
            memory::add(self.layouts.scan_work(), self.masks.scan_work())?,
        )?;
        self.inventory_bytes()
    }

    /// Common inventory counts shared native owners once at their registry roots.
    pub(crate) fn inventory_bytes(&self) -> Result<usize, MaskError> {
        let mut bytes = memory::add(size_of::<Self>(), self.layouts.owned_bytes()?)?;
        bytes = memory::add(bytes, self.masks.owned_bytes()?)?;
        for (_, entry) in self.layouts.iter() {
            let Native::Published {
                owner: layout,
                managed: false,
            } = entry
            else {
                continue;
            };
            bytes = memory::add(bytes, layout.owned_bytes()?)?;
        }
        for (_, entry) in self.masks.iter() {
            let Native::Published {
                owner: mask,
                managed: false,
            } = entry
            else {
                continue;
            };
            bytes = memory::add(bytes, crate::loan::rc_bytes::<MaskRecord>())?;
            bytes = memory::add(bytes, memory::bytes::<u64>(mask.words.capacity())?)?;
        }
        Ok(bytes)
    }

    /// Check the owner before the ordinary slot/generation resolution.
    pub(crate) fn layout(
        &self,
        scope: u64,
        version: CellLayoutVersion,
    ) -> Result<&LayoutRecord, MaskError> {
        if version.scope != scope {
            return Err(HeapError::ForeignHandle.into());
        }
        self.layouts.get(version.key)?.owner().map(Rc::as_ref)
    }

    /// All bit readers require the exact original layout and allocation occupations.
    pub(crate) fn resolve(
        &self,
        scope: u64,
        version: MaskVersion,
        layout: CellLayoutVersion,
    ) -> Result<MaskView<'_>, MaskError> {
        if version.scope != scope {
            return Err(HeapError::ForeignHandle.into());
        }
        let record = self.masks.get(version.key)?.owner()?;
        let cells = self.layout(scope, version.layout)?;
        self.layout(scope, layout)?;
        if version.layout != layout || record.layout != layout {
            return Err(MaskError::LayoutMismatch);
        }
        Ok(MaskView {
            words: &record.words,
            axis: cells.axis,
            components: &cells.components,
            gather: cells.gather.as_ref(),
        })
    }

    /// Prepare the complete owner record before the no-fail slot publication.
    pub(crate) fn publish_mask(
        &mut self,
        scope: u64,
        record: MaskRecord,
        limit: usize,
        budget: &mut Budget,
    ) -> Result<MaskVersion, MaskError> {
        charge(budget, self.masks.scan_work())?;
        let plan = self.masks.change_reserved(1, &[], limit)?;
        let key = *plan.keys().first().ok_or(HeapError::Capacity)?;
        let version = MaskVersion {
            scope,
            key,
            layout: record.layout,
        };
        let mut records = memory::vector(1)?;
        records.push(Native::from(record));
        let prepared = plan.bind(records)?;
        self.masks.publish(prepared);
        Ok(version)
    }
}

impl Heap {
    /// Report actual native mask/layout owner bytes, including retained table capacity.
    ///
    /// This scoped inventory includes word and gather capacities and owner metadata;
    /// numerical regions, caller-owned handles and allocator overhead are separate.
    /// # Errors
    /// Refuses byte/count overflow and Work exhaustion during the bounded slot scan.
    pub fn mask_storage_bytes(&self, budget: &mut Budget) -> Result<usize, MaskError> {
        budget.with_work_phase(super::FAMILY, "mask storage inventory", |budget| {
            self.masks.owned_bytes(budget)
        })
    }

    /// Borrow an immutable mask only against its current exact cell-layout version.
    /// # Errors
    /// Refuses foreign, retired, reused or mismatched layout/allocation occupations.
    pub fn mask(
        &self,
        version: MaskVersion,
        layout: CellLayoutVersion,
    ) -> Result<MaskView<'_>, MaskError> {
        self.masks.resolve(self.scope, version, layout)
    }

    /// Publish a selection mask from native cell indices; repeated indices are idempotent.
    ///
    /// Every allocation starts at zero. This does not authorize reads of floating
    /// values or certify host initialization; those require a qualified value publication.
    /// # Errors
    /// Refuses stale layouts, out-of-capacity cells, arithmetic overflow and resource limits.
    pub fn create_mask(
        &mut self,
        layout: CellLayoutVersion,
        cells: &[usize],
        budget: &mut Budget,
    ) -> Result<MaskVersion, MaskError> {
        budget.with_work_phase(super::FAMILY, "mask publication", |budget| {
            charge(budget, 1)?;
            let cells_layout = self.masks.layout(self.scope, layout)?;
            let capacity = cells_layout.axis.capacity;
            let associated = cells_layout
                .gather
                .as_ref()
                .map_or(capacity, |map| map.indices.len());
            let reservation = memory::Reservation::new(
                self,
                None,
                Some(word_count(capacity)),
                0,
                [&[], &[]],
                budget,
            )?;
            reservation.run(budget, |budget| {
                let mut words = memory::zeroed(word_count(capacity), budget)?;
                for &cell in cells {
                    charge(budget, 1)?;
                    if cell >= capacity {
                        return Err(MaskError::CellOutOfBounds);
                    }
                    if cell >= associated {
                        return Err(MaskError::UnmappedCell);
                    }
                    *words.get_mut(cell / 64).ok_or(MaskError::CellOutOfBounds)? |=
                        1_u64 << (cell % 64);
                }
                charge(budget, 1)?;
                clear_padding(&mut words, capacity);
                self.masks.publish_mask(
                    self.scope,
                    MaskRecord { layout, words },
                    self.limits.descriptors,
                    budget,
                )
            })
        })
    }

    /// Retire one immutable allocation; later reuse advances its ordinary slot generation.
    /// # Errors
    /// Refuses a foreign or stale version and preserves all allocations on resource refusal.
    pub fn retire_mask(
        &mut self,
        version: MaskVersion,
        budget: &mut Budget,
    ) -> Result<(), MaskError> {
        budget.with_work_phase(super::FAMILY, "mask retirement", |budget| {
            charge(budget, 1)?;
            if version.scope != self.scope {
                return Err(HeapError::ForeignHandle.into());
            }
            self.masks.masks.get(version.key)?.unprotected()?;
            let reservation =
                memory::Reservation::new(self, None, None, 0, [&[], &[version.key]], budget)?;
            reservation.run(budget, |budget| {
                charge(budget, self.masks.masks.scan_work())?;
                let plan =
                    self.masks
                        .masks
                        .change_reserved(0, &[version.key], self.limits.descriptors)?;
                let prepared = plan.bind(Vec::<MaskRecord>::new())?;
                self.masks.masks.publish(prepared);
                Ok(())
            })
        })
    }

    /// Checked native population for reporting and allocation, without physical count meaning.
    /// # Errors
    /// Refuses stale occupations, Work exhaustion or native count overflow.
    pub fn mask_population(
        &self,
        version: MaskVersion,
        budget: &mut Budget,
    ) -> Result<usize, MaskError> {
        budget.with_work_phase(super::FAMILY, "mask population", |budget| {
            charge(budget, 1)?;
            population(self.mask(version, version.layout)?.words(), budget)
        })
    }
}

/// Each visited word contributes one checked native prefix count.
pub(crate) fn population(words: &[u64], budget: &mut Budget) -> Result<usize, MaskError> {
    let mut total = 0_usize;
    for word in words {
        charge(budget, 1)?;
        total = add_population(total, word.count_ones())?;
    }
    Ok(total)
}

/// Isolated checked prefix arithmetic also supplies an overflow control without huge allocations.
pub(crate) fn add_population(prefix: usize, population: u32) -> Result<usize, MaskError> {
    prefix
        .checked_add(usize::try_from(population).map_err(|_| MaskError::CountOverflow)?)
        .ok_or(MaskError::CountOverflow)
}

/// Native slots are occupied before a detached owner can escape.
#[derive(Debug)]
pub(crate) enum Native<T> {
    /// Prospective owner belongs to this reserved managed bundle root.
    Reserved(crate::heap::identity::Key),
    /// Immutable storage shared with qualified attachments when managed.
    Published {
        /// The unique registry root; retaining uses own explicit additional edges.
        owner: Rc<T>,
        /// Automatic reclamation is restricted to managed roots.
        managed: bool,
    },
}
impl<T> From<T> for Native<T> {
    fn from(value: T) -> Self {
        Self::Published {
            owner: Rc::new(value),
            managed: false,
        }
    }
}
impl<T> Native<T> {
    /// Exact bundle that occupies a private native reservation.
    pub(crate) const fn reserved_root(&self) -> Option<crate::heap::identity::Key> {
        match self {
            Self::Reserved(root) => Some(*root),
            Self::Published { .. } => None,
        }
    }
    /// A reserved slot cannot expose an incomplete native owner.
    pub(crate) fn owner(&self) -> Result<&Rc<T>, MaskError> {
        match self {
            Self::Reserved(_) => Err(HeapError::OwnershipPending.into()),
            Self::Published { owner, .. } => Ok(owner),
        }
    }
    /// Refuse eager replacement or retirement while a bundle retains this owner.
    pub(crate) fn unprotected(&self) -> Result<(), MaskError> {
        let owner = self.owner()?;
        if Rc::strong_count(owner) == 1 {
            Ok(())
        } else {
            Err(HeapError::PinnedUses.into())
        }
    }
}
