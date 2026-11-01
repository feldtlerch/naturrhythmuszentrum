// SPDX-License-Identifier: AGPL-3.0-only
//! Native scalar and original phasor inputs retain their complete source and companion closure.

use super::AT;
use crate::publication::decision_inputs;
use crate::{
    Coverage, DecisionInputSnapshot, DecisionSnapshotError, Heap, HeapError, PhasorOperand,
    ValueBinding,
};
use nrz_bindings::{
    AxisPhaseCase, AxisPhaseImpedance, Budget, DecisionError, readings::algebra::limits::BudgetKind,
};
use nrz_scalar::Scalar;

/// Complete original block and exact association, with every separately rounded copy.
#[derive(Debug)]
struct PhasorInput<P: Scalar> {
    /// Sealed original physical/CAS association, prepared before any numerical read.
    declaration: AxisPhaseImpedance,
    /// Whole-family and every original selected copy retain the existing live group.
    operand: PhasorOperand<P>,
    /// Per-copy center, total and numerical radius in logical-column order.
    coordinates: [[P; 3]; 4],
    /// Captured complete deterministic coverage.
    coverage: Coverage,
}

/// One immutable original association and its complete numerical dependency closure.
#[derive(Debug)]
pub struct FrozenAxisPhaseInput<P: Scalar> {
    /// Allocating heap authority, separate from portable mathematical proof.
    scope: u64,
    /// Original physical source, sign case and both continuations.
    case: AxisPhaseCase,
    /// Native scalar inventory, empty for an original phasor source.
    operands: Vec<DecisionInputSnapshot<P>>,
    /// Original complete phasor, absent for native scalar input.
    phasor: Option<Box<PhasorInput<P>>>,
    /// Fixed owned snapshot allowance.
    bytes: u64,
}

impl<P: Scalar> FrozenAxisPhaseInput<P> {
    /// Original question and complete designated symbol bundle.
    #[must_use]
    pub const fn case(&self) -> &AxisPhaseCase {
        &self.case
    }
    /// Historical native scalar inventory; empty when the input is a complete phasor block.
    #[must_use]
    pub fn operands(&self) -> &[DecisionInputSnapshot<P>] {
        &self.operands
    }
    /// Historical complete original phasor inventory, when this source uses the block route.
    #[must_use]
    pub fn phasor(&self) -> Option<&PhasorOperand<P>> {
        self.phasor.as_ref().map(|source| &source.operand)
    }
    /// Original predeclared physical association for the complete impedance block.
    #[must_use]
    pub fn impedance(&self) -> Option<&AxisPhaseImpedance> {
        self.phasor.as_ref().map(|source| &source.declaration)
    }
    /// Fixed owned snapshot allowance, excluding original declaration owners.
    #[must_use]
    pub const fn retained_payload_bytes(&self) -> u64 {
        self.bytes
    }
    /// Every actual stored copy remains separate until the numerical containment check.
    pub(super) fn coordinates(&self) -> ([[P; 3]; 4], Coverage) {
        self.phasor.as_ref().map_or_else(
            || {
                let source = &self.operands[0];
                let [center, total, error] = source.coordinates();
                (
                    [
                        [P::ZERO; 3],
                        [center, total, error],
                        [-center, total, error],
                        [P::ZERO; 3],
                    ],
                    source.input().coverage,
                )
            },
            |source| (source.coordinates, source.coverage),
        )
    }
}

impl Heap {
    /// Freeze the predeclared normalized-reactance input and both companions under one borrow.
    /// # Errors
    /// Refuses foreign symbols, selections without native scalar coverage, stale inputs,
    /// missing radius channels and cumulative resources without reserving any output.
    pub fn freeze_axis_phase_input<P: Scalar + 'static>(
        &self,
        case: &AxisPhaseCase,
        input: &ValueBinding<P>,
        budget: &mut Budget,
    ) -> Result<FrozenAxisPhaseInput<P>, DecisionSnapshotError> {
        let bytes = decision_inputs::snapshot_bytes::<P, FrozenAxisPhaseInput<P>>(1)?;
        let operands = decision_inputs::capture(
            self,
            std::slice::from_ref(input),
            bytes,
            budget,
            |symbols, budget| {
                let [symbols] = symbols else {
                    return Err(DecisionError::Association);
                };
                case.authenticate(symbols, budget)
            },
        )?;
        if operands.len() != 1 {
            return Err(HeapError::Capacity.into());
        }
        Ok(FrozenAxisPhaseInput {
            scope: self.scope,
            case: case.clone(),
            operands,
            phasor: None,
            bytes,
        })
    }
    /// Capture the actual original impedance block and every copy's separate radius versions.
    /// # Errors
    /// Refuses substituted families, stale sources, incomplete companions or caller limits.
    pub fn freeze_phasor_axis_phase_input<P: Scalar + 'static>(
        &self,
        declaration: &AxisPhaseImpedance,
        operand: &PhasorOperand<P>,
        budget: &mut Budget,
    ) -> Result<FrozenAxisPhaseInput<P>, DecisionSnapshotError> {
        let bytes =
            u64::try_from(size_of::<FrozenAxisPhaseInput<P>>() + size_of::<PhasorInput<P>>())
                .map_err(|_| HeapError::Capacity)?;
        budget.check(BudgetKind::StagedPayloadBytes, bytes, AT)?;
        budget.check(BudgetKind::OutputBytes, bytes, AT)?;
        budget.work(bytes, AT)?;
        declaration
            .authenticate(operand.whole().adapter().representation(), budget)
            .map_err(|cause| DecisionSnapshotError::Association(Box::new(cause)))?;
        charge_phasor_scan(self, operand, budget)?;
        let (coordinates, coverage) = operand
            .axis_coordinates(self, budget)
            .map_err(|cause| DecisionSnapshotError::Phasor(Box::new(cause)))?;
        Ok(FrozenAxisPhaseInput {
            scope: self.scope,
            case: declaration.case().clone(),
            operands: Vec::new(),
            phasor: Some(Box::new(PhasorInput {
                declaration: declaration.clone(),
                operand: operand.clone(),
                coordinates,
                coverage,
            })),
            bytes,
        })
    }
    /// Validate the complete original lease, including source-only and radius-only changes.
    /// # Errors
    /// Refuses foreign heaps, stale source/companion versions and exhausted resources.
    pub fn check_axis_phase_input<P: Scalar + 'static>(
        &self,
        frozen: &FrozenAxisPhaseInput<P>,
        budget: &mut Budget,
    ) -> Result<(), DecisionSnapshotError> {
        decision_inputs::check(self, frozen.scope, &frozen.operands, budget)?;
        if let Some(source) = &frozen.phasor {
            charge_phasor_scan(self, &source.operand, budget)?;
            for copy in std::iter::once(source.operand.whole()).chain(source.operand.copies()) {
                self.validate_reading(copy)
                    .map_err(|cause| DecisionSnapshotError::Phasor(Box::new(cause.into())))?;
            }
        }
        Ok(())
    }
}

/// Prepay native provenance and every immutable general-reading comparison/entry scan.
fn charge_phasor_scan<P: Scalar>(
    heap: &Heap,
    operand: &PhasorOperand<P>,
    budget: &mut Budget,
) -> Result<(), DecisionSnapshotError> {
    let usage = heap.provenance_usage();
    let scans = count(usage.work())?
        .checked_add(count(usage.records())?)
        .and_then(|work| work.checked_add(1))
        .and_then(|work| work.checked_mul(160))
        .ok_or(HeapError::Capacity)?;
    budget.work(scans, AT)?;
    let group = &operand.whole().group;
    for group in std::iter::once(group).chain(group.antecedents.iter().map(|source| &source.group))
    {
        group_scan(group, budget)?;
    }
    Ok(())
}

/// Validation scans companions and may search every target's original reading; prepay that bound.
fn group_scan(
    group: &crate::binding::live::Group,
    budget: &mut Budget,
) -> Result<(), DecisionSnapshotError> {
    let mut largest = 1;
    let mut targets = 0_u64;
    let mut entries = count(group.coverage.entries().len())?
        .checked_add(count(group.states.len())?)
        .and_then(|size| size.checked_add(count(group.native.len()).ok()?))
        .ok_or(HeapError::Capacity)?;
    for adapter in group.coverage.readings() {
        budget.work(1, AT)?;
        largest = largest.max(reading_cost(adapter.reading())?);
    }
    for companion in &group.companions {
        budget.work(1, AT)?;
        entries = entries
            .checked_add(count(companion.entries().len())?)
            .ok_or(HeapError::Capacity)?;
        for target in companion.targets() {
            budget.work(1, AT)?;
            targets = targets.checked_add(1).ok_or(HeapError::Capacity)?;
            largest = largest.max(reading_cost(target.reading())?);
        }
    }
    for copy in &group.copies {
        budget.work(1, AT)?;
        entries = entries
            .checked_add(count(copy.entries().len())?)
            .and_then(|size| size.checked_add(count(copy.parameters().len()).ok()?))
            .ok_or(HeapError::Capacity)?;
    }
    let comparisons = count(group.coverage.readings().len())?
        .checked_add(1)
        .and_then(|size| size.checked_mul(targets))
        .and_then(|size| size.checked_mul(largest))
        .and_then(|size| size.checked_mul(2))
        .ok_or(HeapError::Capacity)?;
    let work = entries
        .checked_add(comparisons)
        .and_then(|size| size.checked_add(1))
        .and_then(|size| size.checked_mul(5))
        .ok_or(HeapError::Capacity)?;
    budget.work(work, AT)?;
    Ok(())
}

/// Match the existing exact reading-identity accounting units without traversing numerical values.
fn reading_cost(reading: &nrz_bindings::readings::PhysicalReading) -> Result<u64, HeapError> {
    [
        reading.entries().len(),
        reading.family().entries().len(),
        reading.family().original_entries().len(),
        reading.selection().indices().len(),
        reading.map().coefficients().len(),
        reading.map().offsets().len(),
    ]
    .into_iter()
    .try_fold(1_u64, |sum, value| {
        sum.checked_add(count(value)?).ok_or(HeapError::Capacity)
    })
}

/// Native counts never truncate into a smaller declared allowance.
fn count(value: usize) -> Result<u64, HeapError> {
    u64::try_from(value).map_err(|_| HeapError::Capacity)
}
