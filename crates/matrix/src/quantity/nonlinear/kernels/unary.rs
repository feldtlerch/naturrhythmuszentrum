// SPDX-License-Identifier: AGPL-3.0-only
//! Unary schedules compose existing scalar certificates without hidden coordinate conversion.

use super::{Channels, Schedule};
use nrz_bindings::BinaryOp;
use nrz_scalar::{Enclosure, Finite, KernelError, KernelEvaluation, Scalar};

impl Schedule<'_> {
    /// No exponent selects the positive root; an integer retains strict original power definedness.
    pub(super) fn unary<P: Scalar>(
        &mut self,
        input: Channels<P>,
        exponent: Option<i64>,
    ) -> Result<[P; 3], KernelError> {
        let inputs = self.inputs(input)?;
        let mut outputs = [Enclosure::singleton(Finite::ONE); 3];
        for (output, input) in outputs.iter_mut().zip(inputs) {
            *output = if let Some(exponent) = exponent {
                self.power(input, exponent)?
            } else {
                let result = KernelEvaluation::square_root(input, self.context, &mut self.budget)?;
                self.retain(&result)?
            };
        }
        let center = if let Some(exponent) = exponent {
            self.power_center(input.center, exponent)?
        } else {
            self.budget.charge(1)?;
            Finite::new(input.center.get().sqrt())?
        };
        self.finish(center, outputs)
    }

    /// Invert first for negative powers; exponent zero retains the same nonzero-domain guard.
    fn power<P: Scalar>(
        &mut self,
        input: Enclosure<P>,
        exponent: i64,
    ) -> Result<Enclosure<P>, KernelError> {
        let mut factor = if exponent <= 0 {
            let result = KernelEvaluation::reciprocal(input, self.context, &mut self.budget)?;
            self.retain(&result)?
        } else {
            input
        };
        let mut remaining = exponent.unsigned_abs();
        let mut result = None;
        while remaining != 0 {
            if remaining & 1 == 1 {
                result = Some(if let Some(prior) = result {
                    self.product(prior, factor, BinaryOp::Multiply)?
                } else {
                    factor
                });
            }
            remaining >>= 1;
            if remaining != 0 {
                factor = self.product(factor, factor, BinaryOp::Multiply)?;
            }
        }
        Ok(result.unwrap_or(Enclosure::singleton(Finite::ONE)))
    }

    /// Choose a finite native center by the identical inversion-first multiplication order.
    fn power_center<P: Scalar>(
        &mut self,
        input: Finite<P>,
        exponent: i64,
    ) -> Result<Finite<P>, KernelError> {
        if exponent == 0 {
            return Ok(Finite::ONE);
        }
        let mut factor = if exponent < 0 {
            self.budget.charge(1)?;
            Finite::new(P::ONE / input.get())?
        } else {
            input
        };
        let mut remaining = exponent.unsigned_abs();
        let mut result = None;
        while remaining != 0 {
            if remaining & 1 == 1 {
                result = Some(if let Some(prior) = result {
                    self.native_product(prior, factor)?
                } else {
                    factor
                });
            }
            remaining >>= 1;
            if remaining != 0 {
                factor = self.native_product(factor, factor)?;
            }
        }
        Ok(result.unwrap_or(Finite::ONE))
    }

    /// Every native multiplication is charged before it can overflow or produce a center.
    fn native_product<P: Scalar>(
        &mut self,
        left: Finite<P>,
        right: Finite<P>,
    ) -> Result<Finite<P>, KernelError> {
        self.budget.charge(1)?;
        Ok(Finite::new(left.get() * right.get())?)
    }
}
