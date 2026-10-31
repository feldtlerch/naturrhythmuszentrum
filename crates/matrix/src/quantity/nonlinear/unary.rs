// SPDX-License-Identifier: AGPL-3.0-only
//! Exact unary powers select a finite scalar schedule and retain the original heap dependency.

use super::{ScalarQuantityOp, ScalarQuantityRecipe};
use crate::binding::NativeRecipe;
use crate::kernels::contract::Recipe;
use crate::quantity::{Quantity, QuantityContext, QuantityError, QuantityRole, charge, lineage};
use nrz_bindings::Budget;
use nrz_scalar::Scalar;

impl QuantityContext {
    /// Raise a scalar difference quantity to an exact signed integer power.
    /// # Errors
    /// Refuses points, counts, unsupported kinds or exponents, statistics and uncertified domains.
    pub fn integer_power<P: Scalar + 'static, R: QuantityRole>(
        &self,
        heap: &mut crate::Heap,
        input: &Quantity<P, R>,
        exponent: i64,
        budget: &mut Budget,
    ) -> Result<Quantity<P>, QuantityError<P>> {
        charge(budget, 1)?;
        Self::nonlinear_kind::<P>(input.descriptor())?;
        let descriptor = self.declarations.registry().coordinate_integer_power(
            input.descriptor(),
            exponent,
            budget,
        )?;
        self.nonlinear_unary(
            heap,
            input,
            &descriptor,
            ScalarQuantityOp::IntegerPower(exponent),
            budget,
        )
    }

    /// Take a square root with an exact half-dimensional measure and positive-box certificates.
    /// # Errors
    /// Refuses points, counts, unsupported exponent policies or scales, statistics and nonpositive boxes.
    pub fn square_root<P: Scalar + 'static, R: QuantityRole>(
        &self,
        heap: &mut crate::Heap,
        input: &Quantity<P, R>,
        budget: &mut Budget,
    ) -> Result<Quantity<P>, QuantityError<P>> {
        charge(budget, 1)?;
        Self::nonlinear_kind::<P>(input.descriptor())?;
        self.declarations.registry().with_coordinate_square_root(
            input.descriptor(),
            budget,
            |descriptor, budget| {
                self.nonlinear_unary(
                    heap,
                    input,
                    &descriptor,
                    ScalarQuantityOp::SquareRoot,
                    budget,
                )
            },
        )
    }

    /// Keep the original operand and exact law in the ordinary checked operation.
    fn nonlinear_unary<P: Scalar + 'static, R: QuantityRole>(
        &self,
        heap: &mut crate::Heap,
        input: &Quantity<P, R>,
        descriptor: &nrz_bindings::Descriptor,
        operation: ScalarQuantityOp,
        budget: &mut Budget,
    ) -> Result<Quantity<P>, QuantityError<P>> {
        self.check_descriptor(descriptor)?;
        let inputs = vec![self.nonlinear_operand(heap, input, budget)?];
        let recipe = ScalarQuantityRecipe {
            operation,
            allowance: 0,
        };
        let operation = self.nonlinear_contract(inputs, descriptor, recipe)?;
        let counts = lineage::merge(&input.counts, &None, budget)?;
        let mut result = self.execute(heap, operation, vec![input.binding().clone()], budget)?;
        result.counts = counts;
        Ok(result)
    }
}

impl crate::CheckedOperation {
    /// Exact integer parameter of this scalar quantity power, retained beside its original source.
    #[must_use]
    pub fn quantity_integer_exponent(&self) -> Option<i64> {
        if let NativeRecipe::Kernel(contract) = &self.recipe
            && let Recipe::Quantity(recipe) = &contract.recipe
            && let ScalarQuantityOp::IntegerPower(exponent) = recipe.operation
        {
            return Some(exponent);
        }
        None
    }
}
