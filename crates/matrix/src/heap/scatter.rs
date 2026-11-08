// SPDX-License-Identifier: AGPL-3.0-only
//! Selected writes are checked scatters into fresh whole-base scratch, then one publication.

use nrz_scalar::Scalar;

use super::publication::scan;
use super::resources::{ResourcePolicy, add};
use super::{Handle, Heap, HeapError, PublishedBundle, Value, ValueVersion};
use crate::Layout;

impl Heap {
    /// Dispatch a base replacement or build complete base scratch from an injective alias.
    pub(crate) fn replace<P: Scalar + 'static>(
        &mut self,
        handle: Handle<Value, P>,
        expected: ValueVersion,
        centers: &[P],
        bounds: Option<&[P]>,
    ) -> Result<PublishedBundle<P>, HeapError> {
        self.replace_with_policy(
            handle,
            expected,
            centers,
            bounds,
            ResourcePolicy::compatibility(),
        )
    }

    /// Plan the complete base change and any alias copies before exporting numerical scratch.
    fn replace_with_policy<P: Scalar + 'static>(
        &mut self,
        handle: Handle<Value, P>,
        expected: ValueVersion,
        centers: &[P],
        bounds: Option<&[P]>,
        policy: ResourcePolicy,
    ) -> Result<PublishedBundle<P>, HeapError> {
        let base = self.base_handle(handle)?;
        let snapshot = self.snapshot(base, Some(expected))?;
        let retirement = snapshot.retirement();
        let count = 1 + usize::from(bounds.is_some());
        let mut resources = self.legacy_plan::<P>(
            std::iter::repeat_n(snapshot.schema.extent.entries(), count),
            Some(snapshot.key),
            retirement.keys(false),
            retirement.regions(),
        )?;
        if base != handle {
            let copies = snapshot
                .schema
                .extent
                .entries()
                .checked_mul(count)
                .and_then(|entries| entries.checked_mul(size_of::<P>()))
                .ok_or(HeapError::Capacity)?;
            resources.scratch = add(resources.scratch, copies)?;
            let layout = &self.record(handle)?.layout;
            if layout.dense_range()?.is_none() {
                // Non-dense injectivity uses an address vector after its one-range probe drops.
                let addresses = layout
                    .extent()
                    .entries()
                    .checked_mul(size_of::<(super::identity::Key, usize)>())
                    .ok_or(HeapError::Capacity)?;
                resources.scratch = add(
                    resources.scratch,
                    addresses.max(size_of::<(super::identity::Key, usize, usize)>()),
                )?;
            }
        }
        let permit = self.preflight_resources(resources.bytes_only(), policy)?;
        if base == handle {
            return self.replace_base(handle, &snapshot, &retirement, centers, bounds, &permit);
        }
        let layout = &self.record(handle)?.layout;
        scan(layout.extent(), centers, bounds)?;
        crate::layout::validate_outputs(&[(snapshot.region, layout)], self.limits.layout_work)?;
        let mut whole_centers = self.value(base)?.region().export()?;
        scatter(layout, centers, &mut whole_centers)?;
        let whole_bounds = bounds
            .map(|entries| {
                let mut full = self.bounds_for_scatter(base, layout)?;
                scatter(layout, entries, &mut full)?;
                Ok::<_, HeapError>(full)
            })
            .transpose()?;
        let mut published = self.replace_base(
            base,
            &snapshot,
            &retirement,
            &whole_centers,
            whole_bounds.as_deref(),
            &permit,
        )?;
        published.value = handle;
        Ok(published)
    }

    /// Retain known bounds outside the selection; never invent missing physical coverage.
    fn bounds_for_scatter<P: Scalar + 'static>(
        &self,
        base: Handle<Value, P>,
        layout: &Layout,
    ) -> Result<Vec<P>, HeapError> {
        let value = self.value(base)?;
        if value.coverage() == super::Coverage::DeclaredBounds
            && let Some(bound) = value.bound()
        {
            return self.companion(bound)?.region().export();
        }
        // An injective map with this cardinality selects every base entry exactly once.
        if layout.extent().entries() != layout.region_entries() {
            return Err(HeapError::MissingCoverage);
        }
        let mut full = Vec::new();
        full.try_reserve_exact(layout.region_entries())
            .map_err(|_| HeapError::Capacity)?;
        full.resize(layout.region_entries(), P::ZERO);
        Ok(full)
    }
}

/// Copy through checked offsets only after output injectivity has been established.
fn scatter<P: Scalar>(layout: &Layout, selected: &[P], whole: &mut [P]) -> Result<(), HeapError> {
    if selected.len() != layout.extent().entries() || whole.len() != layout.region_entries() {
        return Err(HeapError::LengthMismatch);
    }
    for (logical, value) in selected.iter().enumerate() {
        let destination = whole
            .get_mut(layout.flat_index(logical)?)
            .ok_or(HeapError::InvalidLayout)?;
        *destination = *value;
    }
    Ok(())
}

/// Selected replacement must admit its complete exported base before numerical copying.
#[cfg(test)]
mod tests {
    use super::*;

    /// A two-entry base keeps its slots while the selected one-entry write stages one raw copy.
    fn boundaries<P: Scalar + 'static>() -> Result<(), HeapError> {
        for succeeds in [false, true] {
            let mut heap = Heap::new(crate::HeapLimits::default())?;
            let base = heap
                .import_represented(
                    crate::Extent::new(2, 1)?,
                    &nrz_bindings::Descriptor::ratio(),
                    &[P::ONE; 2],
                )?
                .value();
            let alias = heap.alias(
                base,
                &Layout::strided(crate::Extent::new(1, 1)?, 2, 0, 1, 1)?,
            )?;
            let stamp = heap.value(base)?.stamp();
            let staged = 2 * 64
                + size_of::<faer::Mat<P>>()
                + 4 * size_of::<crate::heap::identity::Key>()
                + size_of::<crate::heap::records::Record>()
                + 2 * size_of::<P>();
            let limit = staged - usize::from(!succeeds);
            let injection =
                crate::loan::fault::Injection::after(if succeeds { usize::MAX } else { 0 });
            let result = heap.replace_with_policy(
                alias,
                stamp.version(),
                &[P::from_u8(3)],
                None,
                ResourcePolicy {
                    output: 64,
                    staged: u64::try_from(limit).map_err(|_| HeapError::Capacity)?,
                },
            );
            match result {
                Ok(_) => {
                    assert!(succeeds);
                    assert_eq!(crate::loan::fault::Injection::raw_copies(), 1);
                    assert_eq!(heap.value(base)?.region().get(0, 0)?, P::from_u8(3));
                    assert_eq!(heap.value(base)?.region().get(1, 0)?, P::ONE);
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
                    assert_eq!(heap.value(base)?.stamp(), stamp);
                }
            }
            drop(injection);
        }
        Ok(())
    }

    /// Alias copies and replacement slabs share exact staged neighbours in both precisions.
    /// ´test:unit:loan-alias-scatter-raw-copy-byte-neighbours´
    #[test]
    fn loan_alias_scatter_raw_copy_byte_neighbours() -> Result<(), HeapError> {
        boundaries::<f32>()?;
        boundaries::<f64>()
    }
}
