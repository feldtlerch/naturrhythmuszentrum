// SPDX-License-Identifier: AGPL-3.0-only
//! Executed full-box scalar certificates publish through the ordinary bound transaction.

mod unary;

use super::{QuantityKernelEvidence, ScalarQuantityOp, ScalarQuantityRecipe};
use crate::kernels::arithmetic::{operand, operand_error};
use crate::publication::Payload;
use crate::{KernelEvidence, NormalizedView, OperationError, OutputReservation, RecipeEvidence};
use nrz_bindings::BinaryOp;
use nrz_scalar::{
    ArithmeticContext, Enclosure, Finite, KernelBudget, KernelCertificate, KernelError,
    KernelEvaluation, Scalar,
};

/// Complete deterministic and overlapping numerical input channels.
#[derive(Clone, Copy)]
struct Channels<P: Scalar> {
    /// Exact represented coordinate.
    center: Finite<P>,
    /// Complete deterministic radius.
    total: Finite<P>,
    /// Numerical-only radius, already included in the total.
    error: Finite<P>,
}

/// A bounded temporary schedule and its executed pure certificates.
struct Schedule<'a> {
    /// Admitted native arithmetic environment.
    context: &'a ArithmeticContext,
    /// Monotone scalar allowance, capped by the caller's remaining Work.
    budget: KernelBudget,
    /// Fixed schedule records, reserved before any numerical work.
    records: Vec<KernelCertificate>,
}

/// Execute one closed schedule with original normalized operands and its reserved fresh target.
#[expect(
    clippy::redundant_pub_crate,
    reason = "The quantity module re-exports this evaluator to the existing kernel owner."
)]
pub(crate) fn evaluate<P: Scalar + 'static, Q: Scalar + 'static>(
    recipe: ScalarQuantityRecipe,
    views: &[NormalizedView<'_, P>],
    reservation: OutputReservation,
    context: &ArithmeticContext,
) -> Result<(Payload<Q>, RecipeEvidence), OperationError> {
    let left = channels(views.first().ok_or(OperationError::InputCount)?)?;
    let mut schedule = Schedule {
        context,
        budget: KernelBudget::new(recipe.allowance),
        records: crate::binding::buffer(recipe.records())?,
    };
    let mut centers = crate::binding::buffer(1)?;
    let mut totals = crate::binding::buffer(1)?;
    let mut errors = crate::binding::buffer(1)?;
    let outcome = match recipe.operation {
        ScalarQuantityOp::Product | ScalarQuantityOp::Quotient => {
            let right = channels(views.get(1).ok_or(OperationError::InputCount)?)?;
            let operation = if recipe.operation == ScalarQuantityOp::Product {
                BinaryOp::Multiply
            } else {
                BinaryOp::Divide
            };
            schedule.binary(left, right, operation)
        }
        ScalarQuantityOp::IntegerPower(exponent) => schedule.unary(left, Some(exponent)),
        ScalarQuantityOp::SquareRoot => schedule.unary(left, None),
    };
    let [center, total, error] = outcome.map_err(|cause| crate::KernelError::Quantity {
        cause,
        work: schedule.budget.usage().work,
    })?;
    centers.push(crate::conversion::same_format(center)?);
    totals.push(crate::conversion::same_format(total)?);
    errors.push(crate::conversion::same_format(error)?);
    let evidence = QuantityKernelEvidence {
        kernel: KernelEvidence {
            kind: recipe.kind(),
            entries: 1,
            coverage: reservation.coverage(),
            precision: Q::PRECISION,
            revision: context.revision(),
            work: recipe.work(),
        },
        certificates: schedule.records.into(),
        usage: schedule.budget.usage(),
    };
    Ok((
        Payload {
            extent: reservation.extent(),
            target: reservation.stamp(),
            centers: centers.into(),
            bounds: Some(totals.into()),
            evaluation: Some(errors.into()),
            contributions: None,
        },
        RecipeEvidence::QuantityKernel(Box::new(evidence)),
    ))
}

/// Borrow finite native coordinates only after the binding has validated their complete sources.
fn channels<P: Scalar + 'static>(
    view: &NormalizedView<'_, P>,
) -> Result<Channels<P>, OperationError> {
    let (center, total) = operand(view, 0)?;
    let error = operand_error(view, 0)?;
    Ok(Channels {
        center: Finite::new(center).map_err(crate::KernelError::from)?,
        total: Finite::new(total.get()).map_err(crate::KernelError::from)?,
        error: Finite::new(error.get()).map_err(crate::KernelError::from)?,
    })
}

impl Schedule<'_> {
    /// Complete total and numerical-only inputs are different targets with shared centers.
    fn inputs<P: Scalar>(&mut self, input: Channels<P>) -> Result<[Enclosure<P>; 3], KernelError> {
        let center = Enclosure::singleton(input.center);
        let total = self.sum(
            center,
            Enclosure::new(input.total.negated().get(), input.total.get())?,
        )?;
        let error = self.sum(
            center,
            Enclosure::new(input.error.negated().get(), input.error.get())?,
        )?;
        Ok([center, total, error])
    }

    /// Record the existing full-box sum and export before returning its numerical enclosure.
    fn sum<P: Scalar>(
        &mut self,
        left: Enclosure<P>,
        right: Enclosure<P>,
    ) -> Result<Enclosure<P>, KernelError> {
        let result = KernelEvaluation::sum(left, right, self.context, &mut self.budget)?;
        self.retain(&result)
    }

    /// Retain only successfully exported records; a later failure discards the private trace.
    fn retain<P: Scalar>(
        &mut self,
        result: &KernelEvaluation<P>,
    ) -> Result<Enclosure<P>, KernelError> {
        self.records.push(result.export(&mut self.budget)?);
        Ok(result.enclosure())
    }

    /// Quotients retain the exact reciprocal interval used by the subsequent product.
    fn product<P: Scalar>(
        &mut self,
        left: Enclosure<P>,
        right: Enclosure<P>,
        operation: BinaryOp,
    ) -> Result<Enclosure<P>, KernelError> {
        let right = if operation == BinaryOp::Divide {
            let result = KernelEvaluation::reciprocal(right, self.context, &mut self.budget)?;
            self.retain(&result)?
        } else {
            right
        };
        let result = KernelEvaluation::product(left, right, self.context, &mut self.budget)?;
        self.retain(&result)
    }

    /// Evaluate all three targets, then enclose the displacement of the chosen native center.
    fn binary<P: Scalar>(
        &mut self,
        left: Channels<P>,
        right: Channels<P>,
        operation: BinaryOp,
    ) -> Result<[P; 3], KernelError> {
        let first = self.inputs(left)?;
        let second = self.inputs(right)?;
        let mut outputs = [Enclosure::singleton(Finite::ZERO); 3];
        for ((output, first), second) in outputs.iter_mut().zip(first).zip(second) {
            *output = self.product(first, second, operation)?;
        }
        self.budget.charge(1)?;
        let center = Finite::new(if operation == BinaryOp::Multiply {
            left.center.get() * right.center.get()
        } else {
            left.center.get() / right.center.get()
        })?;
        self.finish(center, outputs)
    }

    /// Pack complete total and numerical-only boxes around the chosen finite native center.
    fn finish<P: Scalar>(
        &mut self,
        center: Finite<P>,
        outputs: [Enclosure<P>; 3],
    ) -> Result<[P; 3], KernelError> {
        let total = self
            .sum(Enclosure::singleton(center), outputs[1].negated())?
            .abs()
            .upper();
        let error = self
            .sum(Enclosure::singleton(center), outputs[2].negated())?
            .abs()
            .upper();
        let total = if total < error { error } else { total };
        Ok([center.get(), total.get(), error.get()])
    }
}
