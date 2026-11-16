// SPDX-License-Identifier: AGPL-3.0-only
//! A shared checked constant prelude supplies local cyclic frequency and period factors.

mod prelude;
use super::{LcFrequencyCell, LcFrequencyState, LcInputProduct, LcProductError, work};
use crate::{ExecutionAccount, LoanError, NumericalErrorScope};
use nrz_scalar::Scalar;
pub use prelude::LcFrequencyPrelude;

/// Independently enclosed local quantities, with operation-local numerical channels.
#[derive(Clone, Copy, Debug)]
pub struct LcQuantityValues<P: Scalar> {
    /// The existing reciprocal result, interpreted as local angular frequency.
    pub omega: [P; 3],
    /// Cyclic frequency center, propagated total and multiplication rounding radius.
    pub frequency: [P; 3],
    /// Period center, propagated total and multiplication rounding radius.
    pub period: [P; 3],
}
/// Complete original frequency observation with its actual shared factor prelude.
#[derive(Debug)]
pub struct LcQuantityCell<'a, 'p, P: Scalar, const N: usize> {
    /// Original inputs and scoped sign/continuation outcome.
    frequency: LcFrequencyCell<'a, P, N>,
    /// Borrowed original numerical factors, never reconstructed from equal values.
    prelude: &'p LcFrequencyPrelude<P>,
    /// Values exist only when the positive chain and both final products succeed.
    values: Option<LcQuantityValues<P>>,
}
impl<P: Scalar, const N: usize> LcQuantityCell<'_, '_, P, N> {
    /// Original observation and sign outcome remain available on every success.
    #[must_use]
    pub const fn frequency(&self) -> &LcFrequencyCell<'_, P, N> {
        &self.frequency
    }
    /// Actual factor prelude used by this cell.
    #[must_use]
    pub const fn prelude(&self) -> &LcFrequencyPrelude<P> {
        self.prelude
    }
    /// Ready quantities, absent for an invalid or undecided frequency domain.
    #[must_use]
    pub const fn values(&self) -> Option<LcQuantityValues<P>> {
        self.values
    }
    /// Each numerical channel covers only its producing operation's nominal rounding.
    #[must_use]
    pub const fn numerical_error_scope(&self) -> NumericalErrorScope {
        NumericalErrorScope::LocalOperation
    }
}
impl<'a, P: Scalar + 'static, const N: usize> LcInputProduct<'a, P, N> {
    /// Evaluate the local positive chain and reuse fixed factors for frequency and period.
    /// # Errors
    /// Refuses foreign authority, Work, incompatible arithmetic or missing coverage before
    /// loads; finite-image errors preserve the original cell and permit other-cell retries.
    pub fn read_quantities<'p>(
        &self,
        cell: usize,
        prelude: &'p LcFrequencyPrelude<P>,
        account: &mut ExecutionAccount,
    ) -> Result<LcQuantityCell<'a, 'p, P, N>, LcProductError> {
        let original = self
            .cells
            .readers
            .inventory
            .input(0)
            .ok_or(LoanError::UnsupportedContract)?;
        account.authenticate(original.account)?;
        account
            .ledger
            .with_work_phase(crate::loan::FAMILY, "LC local quantities", |budget| {
                let visits = work::<N>(7)?
                    .checked_add(10)
                    .ok_or(LoanError::CountOverflow)?;
                crate::loan::charge(budget, visits)?;
                if prelude.pi().arithmetic_revision() != self.context.revision() {
                    return Err(LoanError::UnsupportedContract.into());
                }
                let frequency = self.evaluate_frequency(cell)?;
                let values = if let LcFrequencyState::Ready { tau, rate } = frequency.state() {
                    let [turn, inverse_turn] = prelude.factors();
                    Some(LcQuantityValues {
                        omega: rate,
                        frequency: self.scale(rate, inverse_turn, cell)?,
                        period: self.scale(tau, turn, cell)?,
                    })
                } else {
                    None
                };
                Ok(LcQuantityCell {
                    frequency,
                    prelude,
                    values,
                })
            })
    }
    /// Ordinary products propagate actual input and constant totals without changing error scope.
    fn scale(&self, input: [P; 3], factor: [P; 3], cell: usize) -> Result<[P; 3], LcProductError> {
        let value = crate::kernels::binary::evaluate_selected(
            [input[0], factor[0]],
            [input[1], factor[1]],
            crate::BinaryElementwise::Product,
            cell,
            &self.context,
        )?;
        Ok([value.center, value.total, value.error])
    }
}
