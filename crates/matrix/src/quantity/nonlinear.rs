// SPDX-License-Identifier: AGPL-3.0-only
//! Exact quantity admission selects the existing bound transaction and heap provenance authority.

mod evidence;
mod kernels;
mod unary;
pub use evidence::QuantityKernelEvidence;
use evidence::ScalarQuantityOp;

use super::{Quantity, QuantityContext, QuantityError, QuantityRole, charge, lineage};
use crate::binding::NativeRecipe;
use crate::kernels::contract::{KernelContract, Recipe};
use crate::{CheckedOperation, Heap, KernelKind, OperationRecord, RegionContract};
use nrz_bindings::{BinaryOp, Budget, Descriptor, Kind, UncertaintyMode};
use nrz_scalar::Scalar;
use std::rc::Rc;

/// Closed numerical law and explicit per-attempt scalar allowance, with no stored float payload.
#[derive(Clone, Copy, Debug)]
#[expect(
    clippy::redundant_pub_crate,
    reason = "The existing kernel contract retains this closed quantity schedule."
)]
pub(crate) struct ScalarQuantityRecipe {
    /// Exact closed arithmetic schedule.
    operation: ScalarQuantityOp,
    /// Remaining caller Work after complete binding and registration preparation.
    allowance: u64,
}

impl ScalarQuantityRecipe {
    /// Existing public kernel identity, independent of physical unit names.
    pub(crate) const fn kind(self) -> KernelKind {
        self.operation.kind()
    }

    /// Complete successfully exported record count, including output radius packing.
    pub(crate) fn records(self) -> usize {
        match self.operation {
            ScalarQuantityOp::Product => 9,
            ScalarQuantityOp::Quotient => 12,
            ScalarQuantityOp::IntegerPower(exponent) => {
                4 + 3 * power_products(exponent) + if exponent <= 0 { 3 } else { 0 }
            }
            ScalarQuantityOp::SquareRoot => 7,
        }
    }

    /// Complete scalar construction, export and native-center schedule.
    pub(crate) fn work(self) -> usize {
        match self.operation {
            ScalarQuantityOp::Product => 1_681,
            ScalarQuantityOp::Quotient => 2_197,
            ScalarQuantityOp::IntegerPower(exponent) => {
                744 + 565 * power_products(exponent)
                    + if exponent <= 0 { 516 } else { 0 }
                    + usize::from(exponent < 0)
            }
            ScalarQuantityOp::SquareRoot => 1_258,
        }
    }
}

/// Squaring plus noninitial accumulation count, bounded by the exact integer's bit length.
fn power_products(exponent: i64) -> usize {
    let magnitude = exponent.unsigned_abs();
    if magnitude == 0 {
        0
    } else {
        usize::try_from(magnitude.ilog2() + magnitude.count_ones() - 1).unwrap_or(usize::MAX)
    }
}

impl<P: Scalar + 'static, R: QuantityRole> Quantity<P, R> {
    /// Current operation and original source dependencies, owned by the existing heap graph.
    /// # Errors
    /// Refuses an imported quantity without a derived record, stale sources or foreign storage.
    pub fn operation_record<'h>(
        &self,
        heap: &'h Heap,
    ) -> Result<&'h OperationRecord, crate::HeapError> {
        let stamp = self.view(heap)?.stamp();
        let id = heap
            .provenance
            .find(stamp)
            .ok_or(crate::HeapError::StaleEvidence)?;
        heap.operation_record(heap.provenance(id)?.record())
    }
}

impl QuantityContext {
    /// Multiply scalar quantities with exact composed measures and certified full-box bounds.
    /// # Errors
    /// Refuses points, unsupported kinds, nonscalar inputs, incomplete statistical evidence or exhausted resources.
    pub fn multiply<P: Scalar + 'static, L: QuantityRole, R: QuantityRole>(
        &self,
        heap: &mut Heap,
        left: &Quantity<P, L>,
        right: &Quantity<P, R>,
        budget: &mut Budget,
    ) -> Result<Quantity<P>, QuantityError<P>> {
        self.nonlinear_binary(heap, left, right, BinaryOp::Multiply, budget)
    }

    /// Divide scalar quantities after certifying the entire divisor enclosure excludes zero.
    /// # Errors
    /// Refuses points, unsupported kinds, statistical companions, zero-containing divisors or exhausted resources.
    pub fn divide<P: Scalar + 'static, L: QuantityRole, R: QuantityRole>(
        &self,
        heap: &mut Heap,
        left: &Quantity<P, L>,
        right: &Quantity<P, R>,
        budget: &mut Budget,
    ) -> Result<Quantity<P>, QuantityError<P>> {
        self.nonlinear_binary(heap, left, right, BinaryOp::Divide, budget)
    }

    /// Check original meanings and complete live inputs before selecting the closed numerical law.
    fn nonlinear_binary<P: Scalar + 'static, L: QuantityRole, R: QuantityRole>(
        &self,
        heap: &mut Heap,
        left: &Quantity<P, L>,
        right: &Quantity<P, R>,
        operation: BinaryOp,
        budget: &mut Budget,
    ) -> Result<Quantity<P>, QuantityError<P>> {
        charge(budget, 1)?;
        for descriptor in [left.descriptor(), right.descriptor()] {
            Self::nonlinear_kind::<P>(descriptor)?;
        }
        let descriptor = self.declarations.registry().coordinate_binary(
            operation,
            left.descriptor(),
            right.descriptor(),
            budget,
        )?;
        self.check_descriptor(&descriptor)?;
        let (left_contract, left_statistics) =
            self.nonlinear_operand(heap, left, operation == BinaryOp::Multiply, budget)?;
        let (right_contract, right_statistics) =
            self.nonlinear_operand(heap, right, operation == BinaryOp::Multiply, budget)?;
        let inputs = vec![left_contract, right_contract];
        let recipe = ScalarQuantityRecipe {
            operation: if operation == BinaryOp::Multiply {
                ScalarQuantityOp::Product
            } else {
                ScalarQuantityOp::Quotient
            },
            allowance: 0,
        };
        let operation = self.nonlinear_contract(inputs, &descriptor, recipe)?;
        let derivative = if left_statistics || right_statistics {
            Some(crate::statistics::prepare_quantity(
                heap, &operation, budget,
            )?)
        } else {
            None
        };
        let counts = lineage::merge(&left.counts, &right.counts, budget)?;
        let mut result = self.execute_with_sensitivity(
            heap,
            operation,
            vec![left.binding().clone(), right.binding().clone()],
            derivative,
            budget,
        )?;
        result.counts = counts;
        Ok(result)
    }

    /// Statistical value descriptors cannot enter deterministic nonlinear transport.
    const fn nonlinear_kind<P: Scalar>(descriptor: &Descriptor) -> Result<(), QuantityError<P>> {
        if matches!(
            descriptor.kind(),
            Kind::Uncertainty(
                UncertaintyMode::Standard
                    | UncertaintyMode::RelativeStandard
                    | UncertaintyMode::Variance
                    | UncertaintyMode::Covariance,
                ..
            )
        ) {
            return Err(QuantityError::NonlinearStatistics);
        }
        Ok(())
    }

    /// Retain exact presentation measures beside the admitted coherent tangent shapes.
    fn nonlinear_contract(
        &self,
        inputs: Vec<RegionContract>,
        descriptor: &Descriptor,
        recipe: ScalarQuantityRecipe,
    ) -> Result<CheckedOperation, crate::OperationError> {
        self.execution.check(inputs.len() + 1, recipe.work())?;
        let extent = crate::Extent::new(1, 1)?;
        let mut output = RegionContract::check(self.shape(descriptor, extent)?)?;
        output.quantity = descriptor.clone();
        let numerical = KernelContract {
            inputs: vec![extent; inputs.len()].into(),
            output: extent,
            recipe: Recipe::Quantity(recipe),
            work: recipe.work(),
        };
        Ok(CheckedOperation {
            inputs: inputs.into(),
            outputs: vec![output].into(),
            recipe: NativeRecipe::Kernel(Rc::new(numerical)),
            revision: 1,
        })
    }

    /// Reject unsupported statistical channels before any numerical coordinate is borrowed.
    fn nonlinear_operand<P: Scalar + 'static, R: QuantityRole>(
        &self,
        heap: &Heap,
        input: &Quantity<P, R>,
        statistics: bool,
        budget: &mut Budget,
    ) -> Result<(RegionContract, bool), QuantityError<P>> {
        let shape = self.input_shape(heap, input)?;
        if shape.extent().rows() != 1 || shape.extent().columns() != 1 {
            return Err(QuantityError::NonlinearExtent);
        }
        charge(
            budget,
            1 + heap
                .joint
                .records
                .len()
                .saturating_add(heap.statistics.records.len()),
        )?;
        let stamp = input.view(heap)?.stamp();
        let joint = heap
            .joint
            .records
            .iter()
            .any(|record| record.targets.iter().any(|target| target.stamp == stamp));
        let moments = heap
            .statistics
            .records
            .iter()
            .any(|record| record.targets.iter().any(|target| target.stamp() == stamp));
        let present = joint || moments;
        if present && !statistics {
            return Err(QuantityError::NonlinearStatistics);
        }
        let mut contract = RegionContract::check(shape)?;
        contract.quantity = input.descriptor().clone();
        Ok((contract, present))
    }
}

/// Configure only this attempt's new recipe after output registration consumes caller Work.
pub(super) fn configure(
    operation: &mut CheckedOperation,
    budget: &Budget,
) -> Result<(), crate::OperationError> {
    if let NativeRecipe::Kernel(contract) = &mut operation.recipe
        && matches!(contract.recipe, Recipe::Quantity(_))
    {
        let contract = Rc::make_mut(contract);
        let Recipe::Quantity(recipe) = &mut contract.recipe else {
            return Err(crate::OperationError::CandidateContract);
        };
        recipe.allowance = budget
            .limits()
            .work
            .saturating_sub(budget.usage().work)
            .min(u64::try_from(recipe.work()).unwrap_or(u64::MAX));
        let bytes = recipe
            .records()
            .saturating_mul(size_of::<nrz_scalar::KernelCertificate>())
            .saturating_add(size_of::<QuantityKernelEvidence>() + 2 * size_of::<usize>());
        for kind in [
            nrz_bindings::readings::algebra::limits::BudgetKind::StagedPayloadBytes,
            nrz_bindings::readings::algebra::limits::BudgetKind::OutputBytes,
        ] {
            budget.check(
                kind,
                u64::try_from(bytes).unwrap_or(u64::MAX),
                nrz_bindings::readings::algebra::Location::new("quantity kernel evidence"),
            )?;
        }
    }
    Ok(())
}

#[expect(
    clippy::redundant_pub_crate,
    reason = "The existing kernel evaluator calls the quantity schedule through this re-export."
)]
pub(crate) use kernels::evaluate;

#[cfg(test)]
mod tests;
