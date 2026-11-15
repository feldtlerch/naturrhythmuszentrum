// SPDX-License-Identifier: AGPL-3.0-only
//! A complete fixed-capacity original supplied-input inventory precedes numerical loans.

use super::{LcInputShape, LcShapeError};
use crate::{ExecutionAccount, LoanError};
use nrz_bindings::LcForwardCover;
use nrz_scalar::Scalar;

/// All original supplied constraints and their copied tuple outputs, in original order.
///
/// N must equal the complete original input count, including repeated constraints.
/// Every slot is filled before return; no partial inventory or numerical loan is exposed.
/// The fixed array adds no allocation beyond the inherited symbolic declarations.
#[derive(Debug)]
pub struct LcInputInventory<P: Scalar, const N: usize> {
    /// Optional slots permit fallible construction without unchecked initialization.
    inputs: [Option<LcInputShape<P>>; N],
}
impl<P: Scalar, const N: usize> LcInputInventory<P, N> {
    /// Declare every original supplied input and its copied tuple output before execution.
    /// # Errors
    /// Refuses a capacity differing from the complete input count, unsupported roles,
    /// invalid extents and exhausted original resources. No partial inventory escapes.
    pub fn declare(
        question: &LcForwardCover,
        counts: [usize; 3],
        account: &mut ExecutionAccount,
    ) -> Result<Self, LcShapeError> {
        Self::preflight(question, account)?;
        let mut inputs = std::array::from_fn(|_| None);
        for (index, slot) in inputs.iter_mut().enumerate() {
            *slot = Some(LcInputShape::declare(
                question.clone(),
                index,
                counts,
                account,
            )?);
        }
        Ok(Self { inputs })
    }

    /// Borrow the immutable schema for an original supplied-constraint position.
    /// Equal descriptors and equal values do not merge distinct positions.
    #[must_use]
    pub fn input(&self, index: usize) -> Option<&LcInputShape<P>> {
        self.inputs.get(index).and_then(Option::as_ref)
    }

    /// New inventory-only charges precede every inherited schema or symbol declaration.
    pub(super) fn preflight(
        question: &LcForwardCover,
        account: &mut ExecutionAccount,
    ) -> Result<(), LoanError> {
        account
            .ledger
            .with_work_phase(crate::loan::FAMILY, "LC input inventory", |budget| {
                crate::loan::charge(budget, 1)?;
                if question.relation().inputs().len() != N {
                    return Err(LoanError::UnsupportedContract);
                }
                for input in question.relation().inputs() {
                    crate::loan::charge(budget, 1)?;
                    if matches!(
                        input.quantity(),
                        nrz_bindings::LcQuantity::Time(_) | nrz_bindings::LcQuantity::Phase(_)
                    ) {
                        return Err(LoanError::UnsupportedContract);
                    }
                }
                Ok(())
            })
    }
}
