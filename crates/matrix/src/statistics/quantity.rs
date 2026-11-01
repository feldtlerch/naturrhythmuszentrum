// SPDX-License-Identifier: AGPL-3.0-only
//! Checked quantity coordinate identities use the ordinary moment and source route.

use super::{
    StatisticsError,
    affine::Affine,
    inventory::Role,
    matrix::{Matrix, singleton},
};
use crate::{CheckedOperation, Extent, KernelKind, OperationError, binding::NativeRecipe};
use nrz_bindings::{
    Budget, CheckedSensitivity, SensitivityError, SensitivityKernel, SensitivityRecipe,
};
use nrz_scalar::Scalar;
use std::rc::Rc;

/// Prepare or reuse the exact derivative while every symbolic phase shares caller Work.
pub(crate) fn prepare(
    heap: &crate::Heap,
    operation: &CheckedOperation,
    budget: &mut Budget,
) -> Result<Rc<CheckedSensitivity>, OperationError> {
    if let Some(proof) = reused(heap, operation, budget)? {
        validate(operation, &proof)?;
        return Ok(proof);
    }
    let [left, right] = operation.inputs() else {
        return Err(StatisticsError::Contract.into());
    };
    let proof = CheckedSensitivity::quantity_product(
        left.shape().declarations().registry(),
        [left.descriptor(), right.descriptor()],
        budget,
    )
    .map_err(refusal)?;
    validate(operation, &proof)?;
    Ok(Rc::new(proof))
}

/// A prepared identity cannot authorize changed exact descriptors or a different recipe.
pub(super) fn validate(
    operation: &CheckedOperation,
    proof: &CheckedSensitivity,
) -> Result<(), OperationError> {
    let NativeRecipe::Kernel(contract) = &operation.recipe else {
        return Err(StatisticsError::Contract.into());
    };
    if contract.kind() != KernelKind::ScalarQuantityProduct
        || proof.recipe() != SensitivityRecipe::QuantityProduct
    {
        return Err(StatisticsError::NonlinearStatistics.into());
    }
    let same_inputs = operation
        .inputs()
        .iter()
        .map(crate::RegionContract::descriptor)
        .eq(proof.inputs());
    let same_outputs = operation
        .outputs()
        .iter()
        .map(crate::RegionContract::descriptor)
        .eq(proof.outputs());
    if !same_inputs || !same_outputs {
        return Err(StatisticsError::Contract.into());
    }
    Ok(())
}

/// Reuse only immutable symbolic facts from published provenance, never numerical parameters.
fn reused(
    heap: &crate::Heap,
    operation: &CheckedOperation,
    budget: &mut Budget,
) -> Result<Option<Rc<CheckedSensitivity>>, OperationError> {
    for record in heap.statistics.records.iter().rev() {
        budget.work(
            1,
            nrz_bindings::readings::algebra::Location::new("quantity sensitivity lookup"),
        )?;
        let Some(proof) = &record.derivative else {
            continue;
        };
        let Some(original) = record.inventory.operation() else {
            continue;
        };
        if proof.recipe() != SensitivityRecipe::QuantityProduct
            || original.revision != operation.revision
        {
            continue;
        }
        let same_inputs = original.inputs().len() == operation.inputs().len()
            && original
                .inputs()
                .iter()
                .zip(operation.inputs())
                .all(|(left, right)| {
                    left.descriptor() == right.descriptor()
                        && left.shape().declarations().id() == right.shape().declarations().id()
                });
        let same_outputs = original.outputs().len() == operation.outputs().len()
            && original
                .outputs()
                .iter()
                .zip(operation.outputs())
                .all(|(left, right)| left.descriptor() == right.descriptor());
        if same_inputs && same_outputs && proof.matches_symbolic_policy(budget).map_err(refusal)? {
            return Ok(Some(Rc::clone(proof)));
        }
    }
    Ok(None)
}

/// Retain the bindings refusal and complete original symbolic outcome.
fn refusal(cause: SensitivityError) -> OperationError {
    StatisticsError::Sensitivity(Rc::new(cause)).into()
}

/// The product coefficients are the other represented coordinates with exact zero evaluation error.
pub(super) fn map<P: Scalar + 'static, Q: Scalar + 'static>(
    points: &[Matrix<P>],
    derivative: Option<&CheckedSensitivity>,
    run: &super::inventory::Run,
) -> Result<Affine<Q>, OperationError> {
    let proof = derivative.ok_or(StatisticsError::Contract)?;
    if proof.recipe() != SensitivityRecipe::QuantityProduct
        || points.len() != 2
        || proof.entries().len() != 2
    {
        return Err(StatisticsError::Contract.into());
    }
    let mut result = Affine {
        jacobian: run.matrix_at(Role::Jacobian, 0, Extent::new(1, 2)?)?,
        offsets: run.matrix_at(Role::Offsets, 0, Extent::new(1, 1)?)?,
    };
    for (column, entry) in proof.entries().iter().enumerate() {
        let SensitivityKernel::QuantityProduct { coordinate } = entry.kernel() else {
            return Err(StatisticsError::Contract.into());
        };
        if coordinate != 1 - column {
            return Err(StatisticsError::Contract.into());
        }
        let point = points.get(coordinate).ok_or(StatisticsError::Contract)?;
        if point.extent != Extent::new(1, 1)? {
            return Err(StatisticsError::Contract.into());
        }
        let center = crate::conversion::same_format(
            *point.centers.first().ok_or(StatisticsError::Contract)?,
        )?;
        result.jacobian.set(0, column, center, singleton(center)?)?;
    }
    Ok(result)
}
