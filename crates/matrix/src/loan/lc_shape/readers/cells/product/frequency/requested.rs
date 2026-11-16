// SPDX-License-Identifier: AGPL-3.0-only
//! Demand selects local successors while every original input remains required.

mod demand;
use super::{
    LcFrequencyPrelude, LcFrequencyTarget, LcInputCell, LcInputProduct, LcProductCell,
    LcProductError, Observation, observation, work,
};
use crate::{ExecutionAccount, LoanError, NumericalErrorScope};
pub use demand::LcQuantityDemand;
use nrz_scalar::Scalar;

/// Only requested local quantities have numerical values.
#[derive(Clone, Copy, Debug)]
pub struct LcRequestedValues<P: Scalar> {
    /// Angular frequency retains the reciprocal's local error.
    pub omega: Option<[P; 3]>,
    /// Cyclic frequency retains the final product's local error.
    pub frequency: Option<[P; 3]>,
    /// Period retains the final product's local error.
    pub period: Option<[P; 3]>,
}
/// Scoped requested-path outcome over represented original boxes.
#[derive(Clone, Copy, Debug)]
pub enum LcRequestedState<P: Scalar> {
    /// Every requested quantity completed, with actual common dependencies retained.
    Ready {
        /// Root center, propagated total and local root rounding radius.
        tau: [P; 3],
        /// Reciprocal dependency, absent for period-only demand.
        rate: Option<[P; 3]>,
        /// Unrequested quantities are absent even if an equal-valued dependency exists.
        values: LcRequestedValues<P>,
    },
    /// An original or intermediate target has no strict positive separation.
    Undecided(LcFrequencyTarget),
    /// An original box is strictly negative, excluding its required positive domain.
    Invalid(LcFrequencyTarget),
}
/// Complete original observation and shared factors retained by one local demand.
#[derive(Debug)]
pub struct LcRequestedCell<'a, 'p, P: Scalar, const N: usize> {
    /// All original readings survive classification before Product.
    observation: Observation<'a, P, N>,
    /// Original ordered inductance and capacitance positions.
    positions: [usize; 2],
    /// Fixed requested subset, never inferred from numerical values.
    demand: LcQuantityDemand,
    /// Actual factors, absent when the path does not use them.
    prelude: Option<&'p LcFrequencyPrelude<P>>,
    /// Scoped selected-path outcome and operation-local companions.
    state: LcRequestedState<P>,
}
impl<P: Scalar, const N: usize> LcRequestedCell<'_, '_, P, N> {
    /// Complete original input observation, including unused roles and companions.
    #[must_use]
    pub const fn inputs(&self) -> &LcInputCell<'_, P, N> {
        match &self.observation {
            Observation::Inputs(inputs) => inputs,
            Observation::Product(product) => product.inputs(),
        }
    }
    /// Actual product, absent when original element classification stops the path.
    #[must_use]
    pub const fn product(&self) -> Option<&LcProductCell<'_, P, N>> {
        match &self.observation {
            Observation::Inputs(_) => None,
            Observation::Product(product) => Some(product),
        }
    }
    /// Explicit original positional roles used by this path.
    #[must_use]
    pub const fn positions(&self) -> [usize; 2] {
        self.positions
    }
    /// Original requested subset remains available on unresolved outcomes.
    #[must_use]
    pub const fn demand(&self) -> LcQuantityDemand {
        self.demand
    }
    /// Actual shared prelude, absent for omega-only even if one was supplied.
    #[must_use]
    pub const fn prelude(&self) -> Option<&LcFrequencyPrelude<P>> {
        self.prelude
    }
    /// Scoped requested values and actual chain dependencies, or unresolved/invalid target.
    #[must_use]
    pub const fn state(&self) -> LcRequestedState<P> {
        self.state
    }
    /// Requested numerical values are absent on invalid or undecided outcomes.
    #[must_use]
    pub const fn values(&self) -> Option<LcRequestedValues<P>> {
        match self.state {
            LcRequestedState::Ready { values, .. } => Some(values),
            _ => None,
        }
    }
    /// Each error covers only its producing operation's nominal rounding.
    #[must_use]
    pub const fn numerical_error_scope(&self) -> NumericalErrorScope {
        NumericalErrorScope::LocalOperation
    }
}
impl<'a, P: Scalar + 'static, const N: usize> LcInputProduct<'a, P, N> {
    /// Evaluate only the requested local successors of the positive product/root.
    ///
    /// Omega alone ignores the optional prelude. Other demands require their compatible
    /// original prelude. No output region or mathematical feasibility is published.
    /// # Errors
    /// Refuses authority, Work, missing/incompatible prelude and incomplete coverage before
    /// loads; finite-image errors retain the original cell and permit other-cell retries.
    pub fn read_requested<'p>(
        &self,
        cell: usize,
        demand: LcQuantityDemand,
        prelude: Option<&'p LcFrequencyPrelude<P>>,
        account: &mut ExecutionAccount,
    ) -> Result<LcRequestedCell<'a, 'p, P, N>, LcProductError> {
        self.observe_attempt(
            cell,
            demand.label(),
            account,
            |account| self.requested_attempt(cell, demand, prelude, account),
            |result| observation::classify(result.state()),
        )
    }
    /// Prepay the complete selected path before compatibility, observation or arithmetic.
    fn requested_attempt<'p>(
        &self,
        cell: usize,
        demand: LcQuantityDemand,
        prelude: Option<&'p LcFrequencyPrelude<P>>,
        account: &mut ExecutionAccount,
    ) -> Result<LcRequestedCell<'a, 'p, P, N>, LcProductError> {
        let original = self
            .cells
            .readers
            .inventory
            .input(0)
            .ok_or(LoanError::UnsupportedContract)?;
        account.authenticate(original.account)?;
        account
            .ledger
            .with_work_phase(crate::loan::FAMILY, "LC requested quantities", |budget| {
                let visits = work::<N>(7)?
                    .checked_add(demand.extra_work())
                    .ok_or(LoanError::CountOverflow)?;
                crate::loan::charge(budget, visits)?;
                let [_, frequency, period] = demand.selection();
                let prelude = if frequency || period {
                    let value = prelude.ok_or(LoanError::UnsupportedContract)?;
                    if value.pi().arithmetic_revision() != self.context.revision() {
                        return Err(LoanError::UnsupportedContract.into());
                    }
                    Some(value)
                } else {
                    None
                };
                let (observation, blocked) = self.observe_product(cell)?;
                let state = if let Some(blocked) = blocked {
                    blocked
                } else {
                    let Observation::Product(product) = &observation else {
                        return Err(LoanError::UnsupportedContract.into());
                    };
                    self.requested_positive(product, demand, prelude)?
                };
                Ok(LcRequestedCell {
                    observation,
                    positions: self.positions,
                    demand,
                    prelude,
                    state,
                })
            })
    }
    /// Common dependencies run once, and unrequested successors remain absent.
    fn requested_positive(
        &self,
        product: &LcProductCell<'_, P, N>,
        demand: LcQuantityDemand,
        prelude: Option<&LcFrequencyPrelude<P>>,
    ) -> Result<LcRequestedState<P>, LcProductError> {
        let tau = self.root(product)?;
        if tau[0] <= tau[1] {
            return Ok(LcRequestedState::Undecided(LcFrequencyTarget::Root));
        }
        let cell = product.inputs().cell();
        let [omega, frequency, period] = demand.selection();
        let rate = if omega || frequency {
            Some(self.reciprocal(tau, cell)?)
        } else {
            None
        };
        let mut values = LcRequestedValues {
            omega: if omega { rate } else { None },
            frequency: None,
            period: None,
        };
        if let Some(prelude) = prelude {
            if frequency {
                values.frequency = Some(self.scale(
                    rate.ok_or(LoanError::UnsupportedContract)?,
                    prelude.factors()[1],
                    cell,
                )?);
            }
            if period {
                values.period = Some(self.scale(tau, prelude.factors()[0], cell)?);
            }
        }
        Ok(LcRequestedState::Ready { tau, rate, values })
    }
}
