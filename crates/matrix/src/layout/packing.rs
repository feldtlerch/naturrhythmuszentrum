// SPDX-License-Identifier: AGPL-3.0-only
//! Fresh contiguous backend scratch for maps without an expressible borrowed view.

use std::marker::PhantomData;

use nrz_scalar::Scalar;

use crate::storage::{Format, Region, Storage};
use crate::{Extent, HeapError, HeapLimits};

/// An owned raw snapshot packed in logical column order, without heap evidence.
///
/// Backend mutation stays private. This object cannot serve as a published value handle.
#[derive(Debug)]
pub struct PackedRegion<P: Scalar> {
    /// Fresh backend owner, sharing no allocation with the heap.
    storage: Storage,
    /// The one packed contiguous allocation.
    region: Region,
    /// Logical rectangle represented by the one-column slab.
    extent: Extent,
    /// Sealed primitive format.
    marker: PhantomData<P>,
}

impl<P: Scalar + 'static> PackedRegion<P> {
    /// Allocate backend scratch only after the caller has bounded and checked the copy.
    pub(crate) fn new(view: &crate::RegionView<'_, P>) -> Result<Self, HeapError> {
        Self::with_policy(
            view,
            crate::heap::resources::ResourcePolicy::compatibility(),
        )
    }

    /// The complete raw-copy and slab plan precedes either numerical allocation.
    fn with_policy(
        view: &crate::RegionView<'_, P>,
        policy: crate::heap::resources::ResourcePolicy,
    ) -> Result<Self, HeapError> {
        let extent = view.extent();
        let mut owner = crate::Heap::new(HeapLimits::new(1, extent.entries(), 0))?;
        let mut plan =
            owner.storage_plan::<P>(std::iter::once(extent.entries()), std::iter::empty())?;
        let raw = extent
            .entries()
            .checked_mul(size_of::<P>())
            .ok_or(HeapError::Capacity)?;
        plan.scratch = crate::heap::resources::add(plan.scratch, raw)?;
        let permit = owner.preflight_resources(plan, policy)?;
        let values = view.export()?;
        let prepared = owner
            .storage
            .prepare(&[values.as_slice()], &[], owner.limits, &permit)?;
        let mut storage = owner.storage;
        let arena = *prepared.keys().first().ok_or(HeapError::Capacity)?;
        storage.publish(prepared);
        let epoch = storage.epoch();
        Ok(Self {
            storage,
            region: Region {
                epoch,
                arena,
                format: Format::of::<P>(),
                offset: 0,
                length: values.len(),
            },
            extent,
            marker: PhantomData,
        })
    }

    /// Original logical rectangle, now packed contiguously.
    #[must_use]
    pub const fn extent(&self) -> Extent {
        self.extent
    }

    /// Read a raw packed entry in logical column order.
    ///
    /// # Errors
    /// Refuses out-of-range indices or an invalid internal allocation.
    pub fn get(&self, row: usize, column: usize) -> Result<P, HeapError> {
        self.storage
            .resolve(self.region)?
            .entry(self.extent.index(row, column)?)
    }
}

/// Raw copy and backend packing share one pre-allocation byte ceiling.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::heap::resources::ResourcePolicy;

    /// Independent two-entry snapshot owns a four-row arena, one slab, one key and a raw copy.
    fn boundaries<P: Scalar + 'static>() -> Result<(), HeapError> {
        let mut heap = crate::Heap::new(HeapLimits::default())?;
        let value = heap
            .import_represented(
                Extent::new(2, 1)?,
                &nrz_bindings::Descriptor::ratio(),
                &[P::ONE; 2],
            )?
            .value();
        let view = heap.value(value)?;
        let retained = 4 * size_of::<(u64, Option<faer::Mat<P>>)>() + 64;
        let staged = retained
            + size_of::<faer::Mat<P>>()
            + size_of::<crate::heap::identity::Key>()
            + 2 * size_of::<P>();
        for (limit, succeeds) in [(staged - 1, false), (staged, true)] {
            let injection =
                crate::loan::fault::Injection::after(if succeeds { usize::MAX } else { 0 });
            let result = PackedRegion::with_policy(
                view.region(),
                ResourcePolicy {
                    output: u64::try_from(retained).map_err(|_| HeapError::Capacity)?,
                    staged: u64::try_from(limit).map_err(|_| HeapError::Capacity)?,
                },
            );
            match result {
                Ok(packed) => {
                    assert!(succeeds);
                    assert_eq!(crate::loan::fault::Injection::raw_copies(), 1);
                    assert_eq!(packed.get(1, 0)?, P::ONE);
                }
                Err(error) => {
                    assert!(!succeeds);
                    assert_eq!(
                        error,
                        HeapError::ResourceBytes {
                            staged: true,
                            used: u64::try_from(staged).map_err(|_| HeapError::Capacity)?,
                            limit: u64::try_from(limit).map_err(|_| HeapError::Capacity)?
                        }
                    );
                    assert_eq!(crate::loan::fault::Injection::raw_copies(), 0);
                    assert_eq!(crate::loan::fault::Injection::attempts(), 0);
                }
            }
            drop(injection);
        }
        assert_eq!(view.region().pack()?.get(0, 0)?, P::ONE);
        Ok(())
    }

    /// Both precisions refuse before exporting and pack at the independently derived neighbour.
    /// ´test:unit:loan-packing-raw-copy-byte-neighbours´
    #[test]
    fn loan_packing_raw_copy_byte_neighbours() -> Result<(), HeapError> {
        boundaries::<f32>()?;
        boundaries::<f64>()
    }
}
