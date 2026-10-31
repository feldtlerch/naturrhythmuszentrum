// SPDX-License-Identifier: AGPL-3.0-only
//! A bounded FIFO set of checked scale-root judgments owned by one bindings registry.

use std::sync::{Arc, Mutex, PoisonError};

use nrz_cas::{ExactConstant, Location, PositiveConstant, Rational, limits::BudgetKind};

use crate::{Budget, Limits, UnitError};

/// Cache accounting is separate from exact arithmetic and measure spelling.
const AT: Location = Location::new("coordinate scale root reuse");

/// Bound policy copying and comparison by the complete fixed-width limit-word count.
const POLICY_WORK: u64 = (size_of::<Limits>() / size_of::<u64>()) as u64;

/// Four entries retain a small mixed-measure working set without unbounded accumulation.
const CAPACITY: usize = 4;

/// Snapshot each slot and its footprint plus the cursor and generation metadata.
const LOOKUP_WORK: u64 = (CAPACITY * 2 + 2) as u64;

/// Deterministic FIFO replacement never depends on addresses or reporting names.
#[derive(Debug)]
pub(super) struct ScaleRoots(Mutex<Entries>);

/// Fixed slot order also fixes the order and cost of exact content comparisons.
#[derive(Clone, Debug)]
struct Entries {
    /// At most four independently checked content-and-policy judgments.
    slots: [Option<Arc<CheckedRoot>>; CAPACITY],
    /// Next empty or oldest insertion slot; hits do not change this cursor.
    next: usize,
    /// Detect concurrent publication before committing a staged ownership envelope.
    generation: u64,
}

/// Cloned registries start with an empty operational cache, preserving declaration independence.
impl Clone for ScaleRoots {
    fn clone(&self) -> Self {
        Self::new()
    }
}

/// Exact content and the complete non-Work policy qualify one retained checked result.
#[derive(Debug)]
pub(super) struct CheckedRoot {
    /// Original positive exact content; structural equality never uses an address or spelling.
    source: PositiveConstant,
    /// Effective ceilings with only the cumulative generation allowance omitted from identity.
    policy: Limits,
    /// The unchanged exact result of the positive square-root checker.
    value: ExactConstant,
    /// Conservative retained input, result and entry envelope.
    bytes: u64,
}

/// A pending replacement stays private until the entire consuming operation succeeds.
#[derive(Debug)]
pub(super) enum RootUse {
    /// The positive unit axiom needs no arithmetic preparation or retained graph.
    Unit,
    /// An exact nonunit preparation, pending complete consumer success.
    Retained {
        /// Checked result and the exact key that authorized its reuse.
        checked: Arc<CheckedRoot>,
        /// Existing entries plus pending preparation, or just existing entries on a hit.
        reserved: u64,
        /// Only a miss may insert, and only if the observed cache has not changed.
        generation: Option<u64>,
    },
}

impl ScaleRoots {
    /// Empty cache construction keeps registry declarations available in constant contexts.
    pub(super) const fn new() -> Self {
        Self(Mutex::new(Entries {
            slots: [const { None }; CAPACITY],
            next: 0,
            generation: 0,
        }))
    }

    /// Cite checked content under the same policy, or prepare a private replacement.
    pub(super) fn prepare(
        &self,
        source: &PositiveConstant,
        budget: &mut Budget,
    ) -> Result<RootUse, UnitError> {
        if is_unit(source, budget)? {
            return Ok(RootUse::Unit);
        }
        budget.work(LOOKUP_WORK, AT)?;
        let previous = self
            .0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone();
        budget.work(POLICY_WORK, AT)?;
        let mut policy = budget.limits().clone();
        policy.work = 0;
        let old_bytes = previous
            .slots
            .iter()
            .flatten()
            .fold(size_of::<Entries>() as u64, |total, entry| {
                total.saturating_add(entry.bytes)
            });
        for entry in previous.slots.iter().flatten() {
            budget.work(POLICY_WORK, AT)?;
            if entry.policy == policy
                && entry
                    .source
                    .value()
                    .structurally_equal(source.value(), budget)?
            {
                retained(old_bytes, budget)?;
                budget.work(1, AT)?;
                return Ok(RootUse::Retained {
                    checked: Arc::clone(entry),
                    reserved: old_bytes,
                    generation: None,
                });
            }
        }
        // Prepay a possible slot, cursor and generation update before any consumer publication.
        budget.work(3, AT)?;
        let checked = budget.with_retained_payload(old_bytes, AT, |budget| {
            CheckedRoot::prepare(source, policy, budget)
        })?;
        let replaced = previous.slots[previous.next]
            .as_ref()
            .map_or(0, |entry| entry.bytes);
        retained(
            old_bytes
                .saturating_sub(replaced)
                .saturating_add(checked.bytes),
            budget,
        )?;
        Ok(RootUse::Retained {
            reserved: old_bytes.saturating_add(checked.bytes),
            checked: Arc::new(checked),
            generation: Some(previous.generation),
        })
    }

    /// Publication is infallible and happens only after the consumer returns a complete result.
    pub(super) fn publish(&self, root: RootUse) {
        if let RootUse::Retained {
            checked,
            generation: Some(observed),
            ..
        } = root
        {
            let mut entries = self.0.lock().unwrap_or_else(PoisonError::into_inner);
            if entries.generation == observed
                && let Some(generation) = observed.checked_add(1)
            {
                let next = entries.next;
                entries.slots[next] = Some(checked);
                entries.next = (next + 1) % CAPACITY;
                entries.generation = generation;
            }
        }
    }
}

impl CheckedRoot {
    /// Preserve the existing exact checker and retain its unmodified result.
    fn prepare(
        source: &PositiveConstant,
        policy: Limits,
        budget: &mut Budget,
    ) -> Result<Self, UnitError> {
        let value = source.sqrt(budget)?;
        let bytes = constant_bytes(source.value())
            .saturating_add(constant_bytes(&value))
            .saturating_add(size_of::<Self>() as u64);
        retained(bytes, budget)?;
        budget.work(copy_work(source.value()), AT)?;
        Ok(Self {
            source: source.clone(),
            policy,
            value,
            bytes,
        })
    }
}

impl RootUse {
    /// Copy literal limbs or share the immutable checked exact graph with the result descriptor.
    pub(super) fn value(&self, budget: &mut Budget) -> Result<ExactConstant, UnitError> {
        match self {
            Self::Unit => {
                budget.work(2, AT)?;
                Ok(ExactConstant::one())
            }
            Self::Retained { checked, .. } => copy(&checked.value, budget),
        }
    }

    /// Reserve both current and pending owners through the rest of the operation.
    pub(super) const fn reserved(&self) -> u64 {
        match self {
            Self::Unit => 0,
            Self::Retained { reserved, .. } => *reserved,
        }
    }
}

/// Canonical positive components with one bit each are exactly one, without a temporary literal.
fn is_unit(source: &PositiveConstant, budget: &mut Budget) -> Result<bool, UnitError> {
    budget.work(1, AT)?;
    let Some(number) = source.value().as_rational() else {
        return Ok(false);
    };
    budget.work(2, AT)?;
    Ok(number.component_bits() == (1, 1))
}

/// Rational copies visit their actual component bytes; graph copies share one immutable owner.
fn copy_work(value: &ExactConstant) -> u64 {
    value.as_rational().map_or(1, |number| {
        let (top, bottom) = number.component_bits();
        top.div_ceil(8).saturating_add(bottom.div_ceil(8))
    })
}

/// Charge each owned exact value copy before copying its limbs or shared reference.
fn copy(value: &ExactConstant, budget: &mut Budget) -> Result<ExactConstant, UnitError> {
    budget.work(copy_work(value), AT)?;
    Ok(value.clone())
}

/// Bound limb allocation or graph records and their retained canonical structural key.
fn constant_bytes(value: &ExactConstant) -> u64 {
    value.as_rational().map_or_else(
        || {
            value.graph().map_or(512, |(graph, _)| {
                let counts = graph.counts();
                counts
                    .payload_bytes
                    .saturating_mul(2)
                    .saturating_add((counts.vertices as u64).saturating_mul(128))
                    .saturating_add((counts.edges as u64).saturating_mul(128))
                    .saturating_add(512)
            })
        },
        |number: &Rational| {
            let (top, bottom) = number.component_bits();
            top.div_ceil(8)
                .saturating_add(bottom.div_ceil(8))
                .saturating_mul(4)
                .saturating_add(128)
        },
    )
}

/// Both cold preparation and warm citation obey the active result and temporary ceilings.
fn retained(bytes: u64, budget: &Budget) -> Result<(), UnitError> {
    budget.check(BudgetKind::OutputBytes, bytes, AT)?;
    budget.check(BudgetKind::StagedPayloadBytes, bytes, AT)?;
    Ok(())
}
