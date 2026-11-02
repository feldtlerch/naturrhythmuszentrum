// SPDX-License-Identifier: AGPL-3.0-only
//! Lazy numerical members preserve the complete mathematical infinite alias family.

use super::{AT, LcProjectionError, lower};
use crate::PiConstant;
use nrz_bindings::readings::algebra::{
    Rational,
    limits::{BudgetKind, BudgetUsage},
};
use nrz_bindings::{Budget, ExactConstant, LcAliasCover};
use nrz_scalar::{ArithmeticContext, Enclosure, LiteralLimits, Scalar};
use std::rc::Rc;

/// Every numerical member retains its index and the complete exact family, without a last cycle.
#[derive(Clone, Debug)]
pub struct LcNumericalAlias<P: Scalar> {
    /// Original complete infinite family and its original unbounded-frequency relation.
    cover: LcAliasCover,
    /// Exact nonnegative mathematical index of this member.
    index: Rational,
    /// Exact affine-pi rate endpoints for this same branch.
    exact_rate: [ExactConstant; 2],
    /// Outward enclosures of the separate rate endpoints.
    rate: [Enclosure<P>; 2],
    /// Outward enclosures of the two monotone inverse-square product endpoints.
    product: [Enclosure<P>; 2],
    /// Cumulative construction snapshots including all exact and numerical endpoint work.
    usage: [BudgetUsage; 2],
}

impl<P: Scalar> LcNumericalAlias<P> {
    /// Complete exact original family; this one rendered member never limits its domain.
    #[must_use]
    pub const fn cover(&self) -> &LcAliasCover {
        &self.cover
    }

    /// Original mathematical branch identifier.
    #[must_use]
    pub const fn index(&self) -> &Rational {
        &self.index
    }

    /// Original exact rate expressions, separately retained from numerical uncertainty.
    #[must_use]
    pub const fn exact_rate(&self) -> &[ExactConstant; 2] {
        &self.exact_rate
    }

    /// Rate endpoint error boxes, ordered by their original exact endpoints.
    #[must_use]
    pub const fn rate_endpoints(&self) -> &[Enclosure<P>; 2] {
        &self.rate
    }

    /// Product endpoint error boxes, retaining the same branch and rate dependence.
    #[must_use]
    pub const fn product_endpoints(&self) -> &[Enclosure<P>; 2] {
        &self.product
    }

    /// Before/after work and storage usage on the caller's cumulative ledger.
    #[must_use]
    pub const fn usage(&self) -> [BudgetUsage; 2] {
        self.usage
    }
}

/// One immutable numerical pi owner renders any requested mathematical alias lazily.
#[derive(Clone, Debug)]
pub struct LcAliasProjection<P: Scalar>(Rc<Data<P>>);

/// Infinite-family ownership never allocates or silently truncates an infinite member list.
#[derive(Debug)]
struct Data<P: Scalar> {
    /// Complete exact alias family and original relation authority.
    cover: LcAliasCover,
    /// Shared certified numerical constant, including its arithmetic revision.
    pi: PiConstant<P>,
    /// Conservative retained metadata plus all borrowed authorities during lowering.
    retained: u64,
}

impl<P: Scalar + 'static> LcAliasProjection<P> {
    /// Retain the complete infinite family and one shared numerical pi authority.
    /// # Errors
    /// Refuses caller storage/work exhaustion, without constructing a partial member list.
    pub fn new(
        cover: LcAliasCover,
        pi: PiConstant<P>,
        budget: &mut Budget,
    ) -> Result<Self, LcProjectionError> {
        budget.work(1, AT)?;
        let retained = cover
            .retained_payload_bytes()
            .saturating_add(cover.relation().retained_payload_bytes())
            .saturating_add(
                u64::try_from(pi.owned_bytes()).map_err(|_| LcProjectionError::Capacity)?,
            )
            .saturating_add(1_024);
        budget.check(BudgetKind::OutputBytes, 1_024, AT)?;
        budget.check(BudgetKind::StagedPayloadBytes, retained, AT)?;
        Ok(Self(Rc::new(Data {
            cover,
            pi,
            retained,
        })))
    }

    /// Complete mathematical union, including its unbounded integer domain.
    #[must_use]
    pub fn cover(&self) -> &LcAliasCover {
        &self.0.cover
    }

    /// Shared numerical pi owner, reused across all requested members.
    #[must_use]
    pub fn pi(&self) -> &PiConstant<P> {
        &self.0.pi
    }

    /// Enclose one arbitrary mathematical member without changing the complete family.
    /// # Errors
    /// Refuses invalid indices, arithmetic revision changes, numerical overflow/underflow
    /// and exhausted caller resources. The retained infinite family is unchanged on failure.
    pub fn branch(
        &self,
        index: Rational,
        literals: LiteralLimits,
        context: &ArithmeticContext,
        budget: &mut Budget,
    ) -> Result<LcNumericalAlias<P>, LcProjectionError> {
        let before = budget.usage();
        budget.work(1, AT)?;
        if context.revision() != self.0.pi.arithmetic_revision() {
            return Err(LcProjectionError::Revision);
        }
        let bytes = budget
            .limits()
            .component_bits
            .div_ceil(8)
            .saturating_mul(32)
            .saturating_add(8_192);
        budget.check(BudgetKind::OutputBytes, bytes, AT)?;
        let retained = self.0.retained.saturating_add(bytes);
        budget.with_retained_payload(retained, AT, |budget| {
            let exact_rate = self.0.cover.rate_endpoints(&index, budget)?;
            let coefficient = self.0.cover.rate_step(budget)?.mul(&index, budget)?;
            let coefficient = lower::decode::<P>(&coefficient, literals, context, budget)?;
            budget.work(1, AT)?;
            let shift = coefficient.mul(self.0.pi.pi().enclosure(), context)?;
            let mut rate = [shift, shift];
            for (endpoint, value) in rate.iter_mut().zip(self.0.cover.base_rate()) {
                let offset = lower::decode(value, literals, context, budget)?;
                budget.work(1, AT)?;
                *endpoint = offset.add(shift, context)?;
            }
            let one = lower::decode::<P>(&Rational::one(), literals, context, budget)?;
            budget.work(4, AT)?;
            let product = [
                one.div(rate[1].square(context)?, context)?,
                one.div(rate[0].square(context)?, context)?,
            ];
            Ok(LcNumericalAlias {
                cover: self.0.cover.clone(),
                index,
                exact_rate,
                rate,
                product,
                usage: [before, budget.usage()],
            })
        })
    }
}
