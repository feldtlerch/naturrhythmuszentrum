// SPDX-License-Identifier: AGPL-3.0-only
//! Exclusive checked bindings, frozen dependencies and private output reservations.
//!
//! # Execution lease and private reservations
//!
//! `Binding::new` requires a `CheckedOperation`. `validate` takes the exclusive heap borrow and captures every actual value identity/version, schema, logical selection and declared-bound occupation, schema and target version. It checks every concrete slot and structural zero, including zero radii for `WholeFamily` zeros. Inputs can overlap or repeat. `ExecutionLimits` bounds the slot count and the total selected centers plus carried companions per scan or private payload set. Heap limits also apply to the complete output reservation.
//!
//! `Bound` owns the checked evidence and real private destination slabs and descriptor reservations. It uses the existing output proof to establish injectivity and mutual disjointness for all destination regions, including companions. `output_reservations` exposes future identities and schemas, never output handles. Dropping a binding releases the reservations without publishing a value. Binding preparation can coexist with one temporary zero payload set and its reserved backend slabs; both sets are bounded by the scratch entry limit. No caller callback runs in publication preparation.
//!
//! `physical_input` lends an immutable `PhysicalView`. `normalized_inputs` lends the complete operation-ordered `NormalizedInputs` bundle with the binding's unique execution identity and its explicit coherent unit-reference interpretation. Companion entries follow exactly the same selected logical map. Neither view grants raw mutation. The exclusive binding and its shorter borrows prevent mutation, growth, compaction and another execution through the same heap until they end. Execution and reserved value identities use the checked nonwrapping process-local allocator. A published ordered shape retains its coordinate identities: a later binding must use those same ordered axes. A projected alias of a derived value needs a matching supported shape contract; matching dimensions or newly repeated coordinate names cannot reinterpret an output.
//!
//! ```rust
//! use nrz_matrix::{Binding, CheckedShape, ExecutionLimits, Handle, Heap, OperationDraft, Value};
//!
//! fn copy_value(heap: &mut Heap, shape: CheckedShape, input: Handle<Value, f64>)
//!     -> Result<Handle<Value, f64>, Box<dyn std::error::Error>>
//! {
//!     let limits = ExecutionLimits::default();
//!     let operation = OperationDraft::copy(vec![shape]).check(limits)?;
//!     let bound = Binding::new(operation, vec![input], limits).validate(heap)?;
//!     let candidate = bound.evaluate(bound.normalized_inputs()?)?;
//!     let outcome = bound.publish(candidate)?;
//!     Ok(outcome.output().first().ok_or("missing copy output")?.value())
//! }
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

mod adapters;
mod error;
mod imports;
mod invariant;
pub use invariant::{InvariantCache, InvariantError, InvariantUse, InvariantValue};
pub(crate) mod kernel;
pub(crate) mod lifecycle;
mod limits;
pub(crate) mod live;
mod operation;
pub(crate) mod parameters;
mod reservation;
pub(crate) mod schema;
mod snapshot;
pub(crate) mod symbols;
mod views;

pub use adapters::{
    AffineAdapter, CompanionAdapter, EntryTicket, OperatorAdapter, ReadingAdapter,
    ReadingAdapterError, ReadingCoverage,
};
pub use error::OperationError;
pub use lifecycle::{SymbolState, ValueBinding};
pub use limits::ExecutionLimits;
pub use live::{
    CompanionEvaluation, CompanionExecution, CompanionExecutionError, CompanionExecutionUsage,
    CoordinateContract, CoordinateError, CoordinateNorm, CoordinateReading, CopyDiscrepancy,
    DependencyPolicy, LcAngularFrequencyParameter, LcAngularFrequencyPublication,
    LcDrivenContinuations, LcDrivenError, LcDrivenInventory, LcDrivenLimits, LcDrivenPoint,
    LcDrivenStatistics, LcDrivenStatus, LcDrivenStratum, LcFrequencyPublication,
    LcImpedancePublication, LcSourceBinding, LcSourceStatistics, LcSpectralBatch,
    LcSpectralBatchInventory, LcSpectralBatchLimits, LcSpectralEvaluation, LcSpectralPublication,
    LcSpectralSchema, LcSpectralStatus, LcSpectrum, LiveBindingError, MappedReading,
    PhasorAllocation, PhasorAllocationChannel, PhasorAllocationStage, PhasorError,
    PhasorEvaluation, PhasorInventory, PhasorOperand, PhasorProduct, PhasorPublication,
    PhasorSolve, PhasorSolveCertificate, PhasorSolveEvaluation, PhasorSolvePublication,
    PhasorStatistics, PhasorTransferBounds, PhasorTransferredResidual, PiCoordinateContract,
    PublishedLcAngularFrequency, PublishedLcFrequency, PublishedLcImpedance, PublishedLcSpectrum,
    PublishedPhasor, PublishedPhasorSolve, ReadingBinding, ReadingImport, ReadingRegion,
    ReadingSelection, ReciprocalDerivativeContract, ReciprocalDerivativeError,
    ReciprocalDerivativeEvaluation, SpectralError, SpectralValue,
};
pub(crate) use operation::NativeRecipe;
pub use operation::{CheckedOperation, OperationDraft, RegionContract};
pub use reservation::OutputReservation;
pub use snapshot::{CompanionSnapshot, InputSnapshot};
pub use views::{NormalizedInputs, NormalizedView, PhysicalView};

use crate::heap::identity::identity;
use crate::{Handle, Heap, HeapError, Value};
use nrz_scalar::Scalar;
use reservation::Reservations;

/// Unique process-local identity of one validated execution, never a buffer address.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExecutionId(pub(crate) u64);

/// Proposed concrete operands; only validation can lend numerical inputs.
#[derive(Debug)]
pub struct Binding<P: Scalar, Q: Scalar = P> {
    /// Private checked recipe, never an unchecked draft.
    pub(crate) operation: CheckedOperation,
    /// Optional exact selected bindings supplied through symbol lookup.
    pub(crate) symbol_inputs: Option<Vec<ValueBinding<P>>>,
    /// Operation-ordered input locators.
    pub(crate) inputs: Vec<Handle<Value, P>>,
    /// Explicit work and private scratch limits.
    pub(crate) limits: ExecutionLimits,
    /// Caller-prepared derivative evidence for the selected reciprocal or quantity recipe.
    pub(crate) reciprocal_statistics: Option<crate::statistics::PreparedDerivative<P>>,
    /// Output scalar type, fixed independently of the input handle type.
    pub(crate) output: std::marker::PhantomData<Q>,
}

impl<P: Scalar + 'static> Binding<P> {
    /// Bind a checked recipe to concrete operands, with no access granted yet.
    #[must_use]
    pub const fn new(
        operation: CheckedOperation,
        inputs: Vec<Handle<Value, P>>,
        limits: ExecutionLimits,
    ) -> Self {
        Self {
            operation,
            symbol_inputs: None,
            inputs,
            limits,
            reciprocal_statistics: None,
            output: std::marker::PhantomData,
        }
    }
}

impl<P: Scalar + 'static> Binding<P> {
    /// Bind a checked recipe to exact symbol-selected inputs.
    #[must_use]
    pub fn from_symbols(
        operation: CheckedOperation,
        inputs: Vec<ValueBinding<P>>,
        limits: ExecutionLimits,
    ) -> Self {
        Self {
            inputs: inputs.iter().map(ValueBinding::handle).collect(),
            symbol_inputs: Some(inputs),
            operation,
            limits,
            reciprocal_statistics: None,
            output: std::marker::PhantomData,
        }
    }
}

impl<P: Scalar + 'static, Q: Scalar + 'static> Binding<P, Q> {
    /// Freeze actual dependencies and reserve complete fresh disjoint output bundles.
    ///
    /// Dropping the result discards all reservations without publishing any handle.
    /// # Errors
    /// Refuses stale/foreign/category/precision mismatches, schemas, structure or limits.
    pub fn validate(self, heap: &mut Heap) -> Result<Bound<'_, P, Q>, OperationError> {
        let budget = nrz_bindings::Budget::new(nrz_bindings::Limits::default())?;
        self.validate_with_resource_budget(heap, &budget)
    }

    /// Validate legacy execution against the original account and all detached owner debits.
    /// # Errors
    /// Refuses combined resource capacity or the ordinary binding checks before zero allocation.
    pub fn validate_with_account<'h>(
        self,
        heap: &'h mut Heap,
        account: &mut crate::ExecutionAccount,
    ) -> Result<Bound<'h, P, Q>, OperationError> {
        let account_identity = account.identity;
        account
            .ledger
            .with_work_phase("matrix.loan", "legacy resource preparation", |budget| {
                let slots = heap.resource_work()?;
                budget.work(
                    u64::try_from(slots).map_err(|_| OperationError::ResourceLimit)?,
                    nrz_bindings::readings::algebra::Location::new("legacy common inventory"),
                )?;
                let policy = crate::heap::resources::ResourcePolicy::from_budget(budget);
                self.validate_inventory(
                    heap,
                    crate::statistics::PlanBudget::Quantity(budget),
                    None,
                    (policy, Some(account_identity)),
                )
            })
    }

    /// Validate with explicit staged and retained ceilings for execution resource metadata.
    /// # Errors
    /// Refuses metadata exhaustion or any ordinary checked binding failure before a factor escapes.
    pub fn validate_with_resource_budget<'h>(
        self,
        heap: &'h mut Heap,
        budget: &nrz_bindings::Budget,
    ) -> Result<Bound<'h, P, Q>, OperationError> {
        self.validate_inventory(
            heap,
            crate::statistics::PlanBudget::Independent(budget),
            None,
            (
                crate::heap::resources::ResourcePolicy::compatibility(),
                None,
            ),
        )
    }

    /// Quantity outputs were completely declared on the caller's cumulative ledger.
    pub(crate) fn validate_quantity<'h>(
        self,
        heap: &'h mut Heap,
        budget: &mut nrz_bindings::Budget,
        symbols: Vec<nrz_bindings::BindingSymbols>,
    ) -> Result<Bound<'h, P, Q>, OperationError> {
        self.validate_inventory(
            heap,
            crate::statistics::PlanBudget::Quantity(budget),
            Some(symbols),
            (
                crate::heap::resources::ResourcePolicy::compatibility(),
                None,
            ),
        )
    }

    /// Authenticate recipe, concrete operands and exact symbol associations before planning bytes.
    fn capture_dependencies(&self, heap: &Heap) -> Result<Vec<InputSnapshot<P>>, OperationError> {
        match self.operation.recipe {
            NativeRecipe::Precision { source, target }
                if source == P::PRECISION && target == Q::PRECISION => {}
            NativeRecipe::Copy
            | NativeRecipe::Affine(_)
            | NativeRecipe::Kernel(_)
            | NativeRecipe::Reciprocal { .. }
            | NativeRecipe::ReciprocalStatistics { .. }
            | NativeRecipe::Solve(_)
                if P::PRECISION == Q::PRECISION => {}
            _ => return Err(HeapError::PrecisionMismatch.into()),
        }
        if self.inputs.len() != self.operation.inputs.len() {
            return Err(OperationError::InputCount);
        }
        self.limits.check(self.inputs.len(), 0)?;
        if let NativeRecipe::Solve(contract) = &self.operation.recipe {
            contract.preflight(heap, &self.inputs, self.limits)?;
        }
        if let Some(symbols) = &self.symbol_inputs {
            for binding in symbols {
                heap.value_by_symbol(binding.symbol(), binding)?;
            }
        }
        capture(heap, &self.inputs)
    }

    /// `validate_quantity` and `validate_with_resource_budget` retain metadata-only ceilings.
    /// `validate_with_account` supplies original account ceilings for the complete physical plan.
    fn resource_permit(
        &self,
        heap: &Heap,
        dependencies: &[InputSnapshot<P>],
        entries: [usize; 2],
        statistics: &crate::statistics::Plan<P>,
        (policy, account): (crate::heap::resources::ResourcePolicy, Option<u64>),
    ) -> Result<crate::heap::resources::ResourcePermit, OperationError> {
        let [entries, output_entries] = entries;
        let parameters = statistics.parameter_lengths(&self.operation)?;
        let mut plan =
            Reservations::<Q>::resource_plan(heap, dependencies, &self.operation, parameters)?;
        let [mut metadata, mut numerical] = statistics.physical_bytes::<Q>(&self.operation)?;
        if let NativeRecipe::Solve(contract) = &self.operation.recipe {
            let extra =
                crate::solve::scratch::Run::physical_bytes(contract, statistics, dependencies)?;
            metadata = metadata
                .checked_add(extra[0])
                .ok_or(OperationError::ResourceLimit)?;
            numerical = numerical
                .checked_add(extra[1])
                .ok_or(OperationError::ResourceLimit)?;
        }
        plan.retained = plan
            .retained
            .checked_add(metadata)
            .ok_or(OperationError::ResourceLimit)?;
        plan.scratch = plan
            .scratch
            .checked_add(numerical)
            .ok_or(OperationError::ResourceLimit)?;
        let scratch_entries = self.operation.solve_work().unwrap_or(
            entries
                .checked_add(output_entries)
                .ok_or(OperationError::ResourceLimit)?,
        );
        let scalar_scratch = scratch_entries
            .checked_mul(size_of::<P>().max(size_of::<Q>()))
            .ok_or(OperationError::ResourceLimit)?;
        let scratch = match &self.operation.recipe {
            NativeRecipe::Kernel(contract)
            | NativeRecipe::Reciprocal {
                kernel: contract, ..
            }
            | NativeRecipe::ReciprocalStatistics {
                kernel: contract, ..
            } => contract.physical_scratch::<P, Q>(scalar_scratch)?,
            // Copy and either precision conversion allocate only their already reserved
            // candidate channels; per-entry verification and conversion remain on the stack.
            NativeRecipe::Copy | NativeRecipe::Precision { .. } => scalar_scratch,
            NativeRecipe::Affine(contracts) => {
                let [retained, scratch] = crate::conversion::physical_bytes::<Q>(
                    contracts,
                    statistics.lineage.is_some(),
                )?;
                plan.retained = plan
                    .retained
                    .checked_add(retained)
                    .ok_or(OperationError::ResourceLimit)?;
                scalar_scratch
                    .checked_add(scratch)
                    .ok_or(OperationError::ResourceLimit)?
            }
            NativeRecipe::Solve(contract) => {
                // The finite RHS/restoration/recovery/statistical requests were added above.
                // The unchanged work allowance covers opaque LU storage and workspaces.
                scalar_scratch
                    .checked_add(contract.physical_factor::<P>()?)
                    .ok_or(OperationError::ResourceLimit)?
            }
        };
        plan.scratch = plan
            .scratch
            .checked_add(scratch)
            .ok_or(OperationError::ResourceLimit)?;
        let execution_plan = heap.executions.admission_plan()?;
        plan.retained = plan
            .retained
            .checked_add(execution_plan.retained)
            .ok_or(OperationError::ResourceLimit)?;
        plan.scratch = plan
            .scratch
            .checked_add(execution_plan.scratch)
            .ok_or(OperationError::ResourceLimit)?;
        if account.is_none() {
            // Compatibility validates ordinary destinations here and complete statistical
            // destination occupations at publication; all physical bytes are admitted here.
            plan.elements = [0; 2];
            plan.elements[crate::storage::Format::of::<Q>().index()] = output_entries;
        }
        let mut resource_permit = heap.preflight_resources(plan, policy)?;
        resource_permit.account = account;
        Ok(resource_permit)
    }

    /// Ordinary transactions and quantity transactions share every live and resource check.
    fn validate_inventory<'h>(
        self,
        heap: &'h mut Heap,
        mut budget: crate::statistics::PlanBudget<'_>,
        symbols: Option<Vec<nrz_bindings::BindingSymbols>>,
        policy: (crate::heap::resources::ResourcePolicy, Option<u64>),
    ) -> Result<Bound<'h, P, Q>, OperationError> {
        let dependencies = self.capture_dependencies(heap)?;
        let entries = payload_entries(&dependencies)?;
        if let NativeRecipe::Kernel(contract)
        | NativeRecipe::Reciprocal {
            kernel: contract, ..
        }
        | NativeRecipe::ReciprocalStatistics {
            kernel: contract, ..
        } = &self.operation.recipe
        {
            self.limits.check(self.inputs.len(), contract.work)?;
        }
        let output_entries = Reservations::<Q>::payload_entries(&dependencies, &self.operation)?;
        self.limits.check(self.inputs.len(), output_entries)?;
        self.limits.check(self.inputs.len(), entries)?;
        for (slot, (dependency, contract)) in dependencies
            .iter()
            .zip(self.operation.inputs.iter())
            .enumerate()
        {
            dependency.validate(heap, contract, slot)?;
            heap.validate_bound_shape(dependency.handle(), contract, slot)?;
        }
        let execution = ExecutionId(identity()?);
        let resources = crate::execution::Lease::new(execution, budget.resources())?;
        let mut statistics = crate::statistics::Plan::prepare::<Q>(
            heap,
            &self.operation,
            &self.inputs,
            self.limits,
            &resources.trace,
            &mut budget,
            self.reciprocal_statistics.clone(),
        )?;
        let resource_permit = self.resource_permit(
            heap,
            &dependencies,
            [entries, output_entries],
            &statistics,
            policy,
        )?;
        heap.executions.register(&resources.trace)?;
        statistics.capture(heap, &self.operation, &self.inputs)?;
        if !matches!(self.operation.recipe, NativeRecipe::Solve(_)) {
            resources.completed_backend(
                0,
                0,
                statistics
                    .work
                    .checked_mul(size_of::<Q>())
                    .ok_or(OperationError::ResourceLimit)?,
            )?;
        }
        let reservations = Reservations::prepare(
            heap,
            &dependencies,
            &self.operation,
            symbols,
            &resource_permit,
        )?;
        reservations.validate(heap)?;
        Ok(Bound {
            heap,
            operation: self.operation,
            dependencies,
            statistics,
            execution,
            resources,
            reservations,
            resource_permit,
            issued: std::cell::Cell::new(None),
        })
    }
}

/// Evidence-owning execution lease; every view borrows for less than this exclusive borrow.
#[derive(Debug)]
pub struct Bound<'h, P: Scalar, Q: Scalar = P> {
    /// The only mutable access to the owner throughout this execution.
    pub(crate) heap: &'h mut Heap,
    /// Checked original recipe and exact normalization references.
    pub(crate) operation: CheckedOperation,
    /// Actual ordered input versions, schemas, selections and companion targets.
    pub(crate) dependencies: Vec<InputSnapshot<P>>,
    /// Complete source model and checked statistical execution allowance.
    pub(crate) statistics: crate::statistics::Plan<P>,
    /// Unforgeable association between a binding and its borrowed numerical inputs.
    pub(crate) execution: ExecutionId,
    /// Independent numerical resource shares exist before any factor can escape this binding.
    pub(crate) resources: crate::execution::Lease,
    /// Fully private output storage and descriptor occupations.
    pub(crate) reservations: Reservations<Q>,
    /// Combined legacy/managed peak remains admitted for this entire exclusive lease.
    pub(crate) resource_permit: crate::heap::resources::ResourcePermit,
    /// The one issued candidate token, unavailable before successful evaluation.
    pub(crate) issued: std::cell::Cell<Option<u64>>,
}

impl<P: Scalar, Q: Scalar> Bound<'_, P, Q> {
    /// Execution identity shared by this binding's views and later candidates.
    #[must_use]
    pub const fn execution(&self) -> ExecutionId {
        self.execution
    }
    /// Checked native recipe and normalization definitions.
    #[must_use]
    pub const fn operation(&self) -> &CheckedOperation {
        &self.operation
    }
    /// Actual operation-ordered dependencies, including companion-only evidence.
    #[must_use]
    pub fn dependencies(&self) -> &[InputSnapshot<P>] {
        &self.dependencies
    }
    /// Future output versions and interpretations; no value handles escape here.
    #[must_use]
    pub fn output_reservations(&self) -> &[OutputReservation] {
        &self.reservations.outputs
    }
}

/// Capture metadata before scanning numerical entries under the total work limit.
fn capture<P: Scalar + 'static>(
    heap: &Heap,
    inputs: &[Handle<Value, P>],
) -> Result<Vec<InputSnapshot<P>>, OperationError> {
    let mut snapshots = buffer(inputs.len())?;
    for &handle in inputs {
        snapshots.push(InputSnapshot::capture(heap, handle)?);
    }
    Ok(snapshots)
}

/// Count centers and all carried companions without confusing base and selected extents.
fn payload_entries<P: Scalar>(inputs: &[InputSnapshot<P>]) -> Result<usize, OperationError> {
    inputs.iter().try_fold(0_usize, |total, input| {
        let count = input
            .layout()
            .extent()
            .entries()
            .checked_mul(
                1 + usize::from(input.carries_bound())
                    + usize::from(input.carries_evaluation())
                    + 3 * usize::from(input.contributions.is_some()),
            )
            .ok_or(OperationError::ResourceLimit)?;
        total
            .checked_add(count)
            .ok_or(OperationError::ResourceLimit)
    })
}

/// Fallibly reserve a metadata or scalar vector before entering an append-only phase.
pub(crate) fn buffer<T>(count: usize) -> Result<Vec<T>, HeapError> {
    let mut result = Vec::new();
    result
        .try_reserve_exact(count)
        .map_err(|_| HeapError::Capacity)?;
    Ok(result)
}
