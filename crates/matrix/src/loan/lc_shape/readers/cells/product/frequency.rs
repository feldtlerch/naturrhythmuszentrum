// SPDX-License-Identifier: AGPL-3.0-only
//! Complete original boxes authorize only strictly positive local frequency continuations.

mod quantities;
pub use quantities::{LcFrequencyPrelude, LcQuantityCell, LcQuantityValues};

use super::{LcInputCell, LcInputProduct, LcProductCell, LcProductError, work};
use crate::{Elementwise, ExecutionAccount, KernelError, LoanError, NumericalErrorScope};
use nrz_scalar::Scalar;

/// Original target whose sign obstructed the local frequency continuation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LcFrequencyTarget {
    /// Explicit original inductance role.
    Inductance,
    /// Explicit original capacitance role.
    Capacitance,
    /// Product of those original roles.
    Product,
    /// Enclosed root whose strict positive separation was lost numerically.
    Root,
}
/// Scoped numerical outcome; none asserts complete physical feasibility.
#[derive(Clone, Copy, Debug)]
pub enum LcFrequencyState<P: Scalar> {
    /// Both native operations succeeded on positively separated whole boxes.
    Ready {
        /// Root center, propagated total and local root rounding radius.
        tau: [P; 3],
        /// Reciprocal center, propagated total and local reciprocal rounding radius.
        rate: [P; 3],
    },
    /// No strict positive separation or exact equality certificate is available.
    Undecided(LcFrequencyTarget),
    /// A complete original box is strictly negative, excluding its required positive domain.
    Invalid(LcFrequencyTarget),
}
/// Original observation before arithmetic or together with its actual product.
#[derive(Debug)]
enum Observation<'a, P: Scalar, const N: usize> {
    /// An original-element sign outcome requires no product.
    Inputs(LcInputCell<'a, P, N>),
    /// Positively separated original elements authorized this product.
    Product(LcProductCell<'a, P, N>),
}
/// Local frequency outcome retaining the complete original input observation.
#[derive(Debug)]
pub struct LcFrequencyCell<'a, P: Scalar, const N: usize> {
    /// Complete original observation, with product companions only after evaluation.
    observation: Observation<'a, P, N>,
    /// Selected original inductance and capacitance positions.
    positions: [usize; 2],
    /// Positive continuation or scoped unresolved/invalid target.
    state: LcFrequencyState<P>,
}
impl<P: Scalar, const N: usize> LcFrequencyCell<'_, P, N> {
    /// Actual product, absent when an original-element sign outcome stops the chain.
    #[must_use]
    pub const fn product(&self) -> Option<&LcProductCell<'_, P, N>> {
        match &self.observation {
            Observation::Inputs(_) => None,
            Observation::Product(product) => Some(product),
        }
    }
    /// Complete original observation, including when no arithmetic ran.
    #[must_use]
    pub const fn inputs(&self) -> &LcInputCell<'_, P, N> {
        match &self.observation {
            Observation::Inputs(inputs) => inputs,
            Observation::Product(product) => product.inputs(),
        }
    }
    /// Original inductance then capacitance positions selected at tile binding.
    #[must_use]
    pub const fn positions(&self) -> [usize; 2] {
        self.positions
    }
    /// Native outcome with local root/reciprocal companions when ready.
    #[must_use]
    pub const fn state(&self) -> LcFrequencyState<P> {
        self.state
    }
    /// Each returned numerical channel covers that operation's nominal rounding only.
    #[must_use]
    pub const fn numerical_error_scope(&self) -> NumericalErrorScope {
        NumericalErrorScope::LocalOperation
    }
}
impl<'a, P: Scalar + 'static, const N: usize> LcInputProduct<'a, P, N> {
    /// Classify complete original boxes and evaluate only the authorized root/reciprocal chain.
    /// # Errors
    /// Refuses authority, Work and incomplete coverage before loads. Native finite-image
    /// refusal retains its original tuple; the view remains usable for other cells.
    pub fn read_frequency(
        &self,
        cell: usize,
        account: &mut ExecutionAccount,
    ) -> Result<LcFrequencyCell<'a, P, N>, LcProductError> {
        let original = self
            .cells
            .readers
            .inventory
            .input(0)
            .ok_or(LoanError::UnsupportedContract)?;
        account.authenticate(original.account)?;
        account
            .ledger
            .with_work_phase(crate::loan::FAMILY, "LC frequency continuation", |budget| {
                let visits = work::<N>(7)?
                    .checked_add(7)
                    .ok_or(LoanError::CountOverflow)?;
                crate::loan::charge(budget, visits)?;
                self.evaluate_frequency(cell)
            })
    }
    /// Complete observation and continuation shared by prepaid local consumers.
    fn evaluate_frequency(&self, cell: usize) -> Result<LcFrequencyCell<'a, P, N>, LcProductError> {
        let inputs = self.cells.observe(cell)?;
        let values = self
            .positions
            .map(|position| inputs.readings()[position].values());
        let targets = [
            LcFrequencyTarget::Inductance,
            LcFrequencyTarget::Capacitance,
        ];
        if let Some(state) = classify(targets, values) {
            return Ok(LcFrequencyCell {
                observation: Observation::Inputs(inputs),
                positions: self.positions,
                state,
            });
        }
        let product = self.evaluate(inputs)?;
        let state = if let Some(state) = classify([LcFrequencyTarget::Product], [product.values()])
        {
            state
        } else {
            self.continue_positive(&product)?
        };
        Ok(LcFrequencyCell {
            observation: Observation::Product(product),
            positions: self.positions,
            state,
        })
    }
    /// Apply ordinary kernels with their own full-box domain and finite-format checks.
    fn continue_positive(
        &self,
        product: &LcProductCell<'_, P, N>,
    ) -> Result<LcFrequencyState<P>, LcProductError> {
        let entry = product.inputs().cell();
        let values = product.values();
        let tau = crate::kernels::nonlinear::evaluate_selected(
            values[0],
            nrz_accuracy::Nonnegative::new(values[1]).map_err(KernelError::from)?,
            Elementwise::SquareRoot,
            entry,
            &self.context,
        )?;
        if tau.center <= tau.total {
            return Ok(LcFrequencyState::Undecided(LcFrequencyTarget::Root));
        }
        let rate = crate::kernels::nonlinear::evaluate_selected(
            tau.center,
            nrz_accuracy::Nonnegative::new(tau.total).map_err(KernelError::from)?,
            Elementwise::Reciprocal,
            entry,
            &self.context,
        )?;
        Ok(LcFrequencyState::Ready {
            tau: [tau.center, tau.total, tau.error],
            rate: [rate.center, rate.total, rate.error],
        })
    }
}
/// Strict dyadic comparisons prove sign without rounded endpoint construction or nominal assumptions.
fn classify<P: Scalar, const N: usize>(
    targets: [LcFrequencyTarget; N],
    values: [[P; 3]; N],
) -> Option<LcFrequencyState<P>> {
    let mut undecided = None;
    for (target, [center, total, _]) in targets.into_iter().zip(values) {
        if center < -total {
            return Some(LcFrequencyState::Invalid(target));
        }
        let unresolved = center <= total;
        let first = undecided.is_none();
        if unresolved && first {
            undecided = Some(LcFrequencyState::Undecided(target));
        }
    }
    undecided
}
