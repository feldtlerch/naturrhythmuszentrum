// SPDX-License-Identifier: AGPL-3.0-only
//! Checked retirement and replacement over the supported whole-region dependency graph.
//!
//! # Replacement, retirement and stale bundles
//!
//! `Heap::replace_represented` and `Heap::replace_declared_bounds` require the caller's captured `ValueVersion`. They preserve the value handle and base `ValueId`, advance the whole base version exactly once, and atomically replace the entire supported companion set. Extents, quantity schemas and precision remain fixed. Even identical center bits with changed bounds advance the version. Represented replacement explicitly removes declared-input coverage. Captured old bound handles become stale; unrelated values and companions remain valid. A stale expected version, invalid payload, capacity refusal or exhausted version leaves the old bundle unchanged.
//!
//! `Heap::retire` retires the whole value/bound bundle and releases its storage. Vacant descriptor and arena slots are reused with incremented generations; exhausted generations are never reused. Old handles and cached internal arena keys cannot revive. A fresh import has a fresh `ValueId` and starts at version one even when its schema and bytes match retired data. Schema capacity counts immutable lifetime entries, including those no longer used by a live value. Retiring a base also retires every alias of that occupation. Retiring only an alias releases its descriptor; nested aliases already point directly to the base.
//!
//! `Heap::retire` can reclaim a stale derived base and all its companions and aliases. Retirement checks live storage ownership, formats, layouts, schemas and direct target/reverse edges through a private structural snapshot; it neither reads numerical entries nor authorizes stale mathematical evidence. Historical operation records remain retained. Current compaction refuses while an unreclaimed stale derived descriptor exists; retire those stale bundles before compacting the surviving current heap. Relocation of stale storage is outside this contract; any extension must preserve evidence access checks.

use nrz_scalar::Scalar;

use super::identity::Key;
use super::records::Meaning;
use super::schema::Schema;
use super::{Handle, Heap, HeapError, PublishedBundle, Value, ValueStamp, ValueVersion};
use crate::storage::Region;

/// Validated current bundle; its private snapshot is used only under the exclusive change borrow.
#[derive(Debug)]
pub(crate) struct Snapshot {
    /// Surviving value occupation.
    pub(crate) key: Key,
    /// Captured identity and version.
    pub(crate) stamp: ValueStamp,
    /// Immutable physical interpretation, unchanged by supported replacements.
    pub(crate) schema: Schema,
    /// Current value allocation.
    pub(crate) region: Region,
    /// Reverse dependency edge and allocation of the current bound.
    pub(crate) bound: Option<(Key, Region)>,
    /// Additional evaluation allocation retired with its complete target bundle.
    pub(crate) evaluation: Option<(Key, Region)>,
    /// Product contribution allocations retire with the complete target bundle.
    pub(crate) contributions: Option<[(Key, Region); 3]>,
}

/// Fixed whole-bundle retirement data lives on the stack through admission and replacement.
#[derive(Debug)]
pub(crate) struct Retirement {
    /// At most value, total, evaluation and three contribution occupations.
    keys: [Key; 6],
    /// Corresponding original allocations in the same order.
    regions: [Region; 6],
    /// Initialized bundle prefix.
    length: usize,
}

impl Retirement {
    /// The base occupation survives replacement and is included only for retirement.
    pub(crate) fn keys(&self, include_value: bool) -> &[Key] {
        &self.keys[usize::from(!include_value)..self.length]
    }

    /// The complete original allocation set, without a heap-backed snapshot.
    pub(crate) fn regions(&self) -> &[Region] {
        &self.regions[..self.length]
    }
}

impl Snapshot {
    /// Capture the fixed bundle once, before admission, without allocating.
    pub(crate) fn retirement(&self) -> Retirement {
        let mut result = Retirement {
            keys: [self.key; 6],
            regions: [self.region; 6],
            length: 0,
        };
        let members = std::iter::once((self.key, self.region))
            .chain(self.bound)
            .chain(self.evaluation)
            .chain(self.contributions.into_iter().flatten());
        for ((key, region), (slot, allocation)) in
            members.zip(result.keys.iter_mut().zip(&mut result.regions))
        {
            *slot = key;
            *allocation = region;
            result.length += 1;
        }
        result
    }
}

impl Heap {
    /// Replace selected centers while reducing the entire base to represented coverage.
    ///
    /// The handle and value identity survive; the version advances even for identical bits.
    /// All aliases resolve the advanced base version. Old bound occupations are retired
    /// atomically. Extent, quantity and format stay fixed; overlapping destinations refuse.
    ///
    /// # Errors
    /// Refuses invalid handles, a mismatched expected version, exhausted counters,
    /// nonfinite/wrong-length payloads or insufficient preparation capacity.
    /// Every refusal leaves the old value, version and companion readable.
    pub fn replace_represented<P: Scalar + 'static>(
        &mut self,
        handle: Handle<Value, P>,
        expected: ValueVersion,
        centers: &[P],
    ) -> Result<PublishedBundle<P>, HeapError> {
        self.replace(handle, expected, centers, None)
    }

    /// Atomically replace selected centers and their declared componentwise bounds.
    ///
    /// Bounds are fresh assumptions for the new version, including with identical centers.
    /// Unselected entries keep their current centers and bounds. A partial selection of
    /// a represented-only base refuses because bounds for the remaining entries are unknown.
    /// The entire base advances once; unrelated allocations retain their versions.
    ///
    /// # Errors
    /// In addition to represented-replacement refusals, rejects invalid radii and
    /// unsupported radius derivation. No old evidence survives as evidence for the new version.
    pub fn replace_declared_bounds<P: Scalar + 'static>(
        &mut self,
        handle: Handle<Value, P>,
        expected: ValueVersion,
        centers: &[P],
        bounds: &[P],
    ) -> Result<PublishedBundle<P>, HeapError> {
        self.replace(handle, expected, centers, Some(bounds))
    }

    /// Retire a value and its entire supported companion set, releasing their arenas.
    ///
    /// Copies of all retired handles remain stale after slot reuse. Exhausted generations
    /// retire permanently; immutable schemas remain in the heap's lifetime-bounded pool.
    ///
    /// # Errors
    /// Refuses foreign/stale handles, invalid evidence or a failed preparation allocation.
    /// Retirement is atomic, including when both value and bound have to be released.
    pub fn retire<P: Scalar + 'static>(
        &mut self,
        handle: Handle<Value, P>,
    ) -> Result<(), HeapError> {
        self.retire_with_policy(handle, super::resources::ResourcePolicy::compatibility())
    }

    /// The original retirement transaction admits removal keys with zero retained bytes.
    fn retire_with_policy<P: Scalar + 'static>(
        &mut self,
        handle: Handle<Value, P>,
        policy: super::resources::ResourcePolicy,
    ) -> Result<(), HeapError> {
        if matches!(self.record(handle)?.meaning, Meaning::Alias { .. }) {
            let key = handle.key;
            let resources =
                self.legacy_plan::<P>(std::iter::empty(), None, std::slice::from_ref(&key), &[])?;
            let permit = self.preflight_resources(resources, policy)?;
            let plan = self.change_descriptors(0, None, std::slice::from_ref(&key), &permit)?;
            self.descriptors.publish(plan.bind(Vec::new())?);
            return Ok(());
        }
        let snapshot = self.retirement_snapshot(handle)?;
        let retirement = snapshot.retirement();
        let aliases = self.descriptors.iter().filter(|(_, record)| {
            matches!(record.meaning, Meaning::Alias { base } if base == snapshot.key)
        }).count();
        let mut resources = self.legacy_plan::<P>(
            std::iter::empty(),
            None,
            retirement.keys(true),
            retirement.regions(),
        )?;
        let count = super::resources::add(retirement.keys(true).len(), aliases)?;
        let extra_keys = if aliases == 0 {
            0
        } else {
            super::resources::add(count, aliases)?
        };
        resources.scratch = super::resources::add(
            resources.scratch,
            extra_keys
                .checked_mul(size_of::<Key>())
                .ok_or(HeapError::Capacity)?,
        )?;
        let permit = self.preflight_resources(resources, policy)?;
        let mut expanded = Vec::new();
        let keys = if aliases == 0 {
            retirement.keys(true)
        } else {
            #[cfg(test)]
            crate::loan::fault::entered_retirement_keys();
            expanded
                .try_reserve_exact(count)
                .map_err(|_| HeapError::Capacity)?;
            expanded.extend_from_slice(retirement.keys(true));
            expanded.extend(self.descriptors.iter().filter_map(|(key, record)| {
                matches!(record.meaning, Meaning::Alias { base } if base == snapshot.key)
                    .then_some(key)
            }));
            expanded.as_slice()
        };
        let plan = self.change_descriptors(0, None, keys, &permit)?;
        let descriptors = plan.bind(Vec::new())?;
        let storage = self
            .storage
            .prepare::<P>(&[], retirement.regions(), self.limits, &permit)?;
        let invalidations = crate::binding::live::invalidation::whole(self, snapshot.key);
        self.storage.publish(storage);
        self.descriptors.publish(descriptors);
        crate::binding::live::invalidation::publish(invalidations);
        Ok(())
    }

    /// Resolve every dependency before capturing the private retirement set.
    pub(crate) fn snapshot<P: Scalar + 'static>(
        &self,
        handle: Handle<Value, P>,
        expected: Option<ValueVersion>,
    ) -> Result<Snapshot, HeapError> {
        let view = self.value(handle)?;
        let stamp = view.stamp();
        if let Some(expected) = expected
            && expected != stamp.version
        {
            return Err(HeapError::VersionMismatch {
                expected,
                actual: stamp.version,
            });
        }
        let record = self.record(handle)?;
        let Meaning::Value {
            bound,
            contributions,
            ..
        } = record.meaning
        else {
            return Err(HeapError::CategoryMismatch);
        };
        let companion = bound
            .map(|key| {
                self.descriptors
                    .get(key)
                    .and_then(|entry| self.owning_region(entry).map(|region| (key, region)))
            })
            .transpose()?;
        Ok(Snapshot {
            key: handle.key,
            stamp,
            schema: *self
                .schemas
                .get(record.schema)
                .ok_or(HeapError::InvalidLayout)?,
            region: self.owning_region(record)?,
            bound: companion,
            contributions: self.contribution_regions::<P>(
                contributions,
                handle.key,
                stamp,
                view.region().extent(),
            )?,
            evaluation: view
                .evaluation()
                .map(|handle| {
                    self.record(handle).and_then(|entry| {
                        self.owning_region(entry).map(|region| (handle.key, region))
                    })
                })
                .transpose()?,
        })
    }
}

/// Inaccessible version and dependency-corruption boundaries.
#[cfg(test)]
mod tests {
    use nrz_bindings::Descriptor;

    use super::super::{Extent, HeapLimits};
    use super::{Heap, HeapError, Meaning, ValueVersion};

    /// Retirement joins descriptor-removal and arena-removal keys without retaining a new owner.
    /// ´test:unit:loan-retirement-physical-key-neighbours´
    #[test]
    fn loan_retirement_physical_key_neighbours() -> Result<(), HeapError> {
        for (alias_only, aliases) in [(true, 1), (false, 0), (false, 1), (false, 7)] {
            let mut heap = Heap::new(HeapLimits::default())?;
            let base = heap
                .import_represented(Extent::new(2, 1)?, &Descriptor::ratio(), &[1.0_f64; 2])?
                .value();
            let mut alias = base;
            for _ in 0..aliases {
                alias = heap.alias(
                    base,
                    &crate::Layout::strided(Extent::new(1, 1)?, 2, 0, 1, 1)?,
                )?;
            }
            let handle = if alias_only { alias } else { base };
            let staged = if alias_only {
                size_of::<super::Key>()
            } else {
                // One slab, all descriptor removals, two arena keys and, only with aliases,
                // one exact expanded input-key backing. The six-member snapshots are stack values.
                let input_keys = if aliases == 0 { 0 } else { 1 + aliases };
                64 + (3 + aliases + input_keys) * size_of::<super::Key>()
            };
            let injection = crate::loan::fault::Injection::after(0);
            assert_eq!(
                heap.retire_with_policy(
                    handle,
                    super::super::resources::ResourcePolicy {
                        output: 0,
                        staged: u64::try_from(staged - 1).map_err(|_| HeapError::Capacity)?,
                    }
                ),
                Err(HeapError::ResourceBytes {
                    staged: true,
                    used: u64::try_from(staged).map_err(|_| HeapError::Capacity)?,
                    limit: u64::try_from(staged - 1).map_err(|_| HeapError::Capacity)?,
                })
            );
            assert_eq!(crate::loan::fault::Injection::attempts(), 0);
            assert_eq!(crate::loan::fault::Injection::retirement_keys(), 0);
            drop(injection);
            assert_eq!(
                heap.value(alias)?.region().get(0, 0)?.to_bits(),
                1.0_f64.to_bits()
            );
            heap.retire_with_policy(
                handle,
                super::super::resources::ResourcePolicy {
                    output: 0,
                    staged: u64::try_from(staged).map_err(|_| HeapError::Capacity)?,
                },
            )?;
            assert_eq!(
                crate::loan::fault::Injection::retirement_keys(),
                usize::from(!alias_only && aliases != 0)
            );
            assert!(matches!(heap.value(alias), Err(HeapError::StaleHandle)));
            assert_eq!(heap.storage.elements(), [0, if alias_only { 2 } else { 0 }]);
        }
        Ok(())
    }

    /// A maximum content version refuses replacement without changing bytes or identity.
    /// ´test:unit:value-version-exhaustion-is-atomic´
    #[test]
    fn value_version_exhaustion_is_atomic() -> Result<(), HeapError> {
        let mut heap = Heap::new(HeapLimits::new(1, 1, 1))?;
        let bundle =
            heap.import_represented(Extent::new(1, 1)?, &Descriptor::ratio(), &[2.0_f64])?;
        let record = heap.descriptors.get_mut(bundle.value().key)?;
        let Meaning::Value { stamp, .. } = &mut record.meaning else {
            return Err(HeapError::CategoryMismatch);
        };
        stamp.version = ValueVersion(u64::MAX);
        let captured = heap.value(bundle.value())?.stamp();
        assert!(matches!(
            heap.replace_represented(bundle.value(), captured.version(), &[3.0]),
            Err(HeapError::CounterExhausted)
        ));
        assert_eq!(heap.value(bundle.value())?.stamp(), captured);
        assert_eq!(
            heap.value(bundle.value())?.region().get(0, 0)?.to_bits(),
            2.0_f64.to_bits()
        );
        heap.retire(bundle.value())?;
        let fresh =
            heap.import_represented(Extent::new(1, 1)?, &Descriptor::ratio(), &[3.0_f64])?;
        assert_ne!(heap.value(fresh.value())?.stamp().id(), captured.id());
        assert_eq!(heap.value(fresh.value())?.stamp().version().number(), 1);
        Ok(())
    }

    /// Even equal bytes and live slots cannot authorize evidence for another target version.
    /// ´test:unit:companion-access-rechecks-captured-version´
    #[test]
    fn companion_access_rechecks_captured_version() -> Result<(), HeapError> {
        let mut heap = Heap::new(HeapLimits::default())?;
        let bundle = heap.import_declared_bounds(
            Extent::new(1, 1)?,
            &Descriptor::ratio(),
            &[2.0_f64],
            &[0.5],
        )?;
        let companion = bundle.bound().ok_or(HeapError::StaleEvidence)?;
        let record = heap.descriptors.get_mut(bundle.value().key)?;
        let Meaning::Value { stamp, .. } = &mut record.meaning else {
            return Err(HeapError::CategoryMismatch);
        };
        stamp.version = ValueVersion(2);
        assert!(matches!(
            heap.companion(companion),
            Err(HeapError::StaleEvidence)
        ));
        assert!(matches!(
            heap.value(bundle.value()),
            Err(HeapError::StaleEvidence)
        ));
        Ok(())
    }

    /// A forged forward target must also agree with the target's reverse companion edge.
    /// ´test:unit:companion-access-rechecks-reverse-edge´
    #[test]
    fn companion_access_rechecks_reverse_edge() -> Result<(), HeapError> {
        let mut heap = Heap::new(HeapLimits::default())?;
        let extent = Extent::new(1, 1)?;
        let quantity = Descriptor::ratio();
        let first = heap.import_declared_bounds(extent, &quantity, &[2.0_f64], &[0.5])?;
        let second = heap.import_declared_bounds(extent, &quantity, &[2.0_f64], &[0.5])?;
        let current = heap.value(second.value())?.stamp();
        let companion = first.bound().ok_or(HeapError::StaleEvidence)?;
        let record = heap.descriptors.get_mut(companion.key)?;
        record.meaning = Meaning::Bound {
            target: second.value().key,
            stamp: current,
        };
        assert!(matches!(
            heap.companion(companion),
            Err(HeapError::StaleEvidence)
        ));
        assert_eq!(heap.value(second.value())?.stamp(), current);
        Ok(())
    }
}
