// SPDX-License-Identifier: AGPL-3.0-only
//! Lazy both-format time boxes and tail starts share one numerical pi authority.

use super::{AT, LcProjectionError, lower};
use crate::PiConstant;
use nrz_bindings::readings::algebra::{
    Rational,
    limits::{BudgetKind, BudgetUsage},
};
use nrz_bindings::{Budget, LcRelationError, LcTimeBox, LcTimeTail, LcZeroTimes};
use nrz_scalar::{ArithmeticContext, Enclosure, LiteralLimits, Scalar};
use std::rc::Rc;

/// One numbered box retains the entire unbounded original question.
#[derive(Clone, Debug)]
pub struct LcNumericalTime<P: Scalar> {
    /// Shared original family and numerical constant.
    projection: LcTimeProjection<P>,
    /// Exact cycle and both closed mathematical endpoints.
    exact: LcTimeBox,
    /// Independent rounding enclosure for each endpoint.
    endpoints: [Enclosure<P>; 2],
    /// Cumulative before/after materialization snapshots.
    usage: [BudgetUsage; 2],
}

impl<P: Scalar> LcNumericalTime<P> {
    /// Complete original family; this member cannot impose a cutoff.
    #[must_use]
    pub const fn projection(&self) -> &LcTimeProjection<P> {
        &self.projection
    }

    /// Exact original index and endpoints.
    #[must_use]
    pub const fn exact(&self) -> &LcTimeBox {
        &self.exact
    }

    /// Both endpoint errors, separate from physical rate spread.
    #[must_use]
    pub const fn endpoints(&self) -> &[Enclosure<P>; 2] {
        &self.endpoints
    }

    /// Exact and numerical work spent during this materialization.
    #[must_use]
    pub const fn usage(&self) -> [BudgetUsage; 2] {
        self.usage
    }
}

/// An enclosed start with a mathematical unbounded upper endpoint and explicit assurance.
#[derive(Clone, Debug)]
pub struct LcNumericalTail<P: Scalar> {
    /// Retains the numerical constant and complete original family.
    projection: LcTimeProjection<P>,
    /// Minimal overlap and optional total-existence certificate.
    exact: LcTimeTail,
    /// Numerical error on the exact included start only.
    start: Enclosure<P>,
    /// Before/after cumulative lowering work.
    usage: [BudgetUsage; 2],
}

impl<P: Scalar> LcNumericalTail<P> {
    /// Original numerical authority and whole family.
    #[must_use]
    pub const fn projection(&self) -> &LcTimeProjection<P> {
        &self.projection
    }

    /// Exact tail with the assurance strength attached to its evidence.
    #[must_use]
    pub const fn exact(&self) -> &LcTimeTail {
        &self.exact
    }

    /// Enclosure of the included exact start, with no floating infinity endpoint.
    #[must_use]
    pub const fn start(&self) -> Enclosure<P> {
        self.start
    }

    /// Exact and numerical work for this separate tail-start value.
    #[must_use]
    pub const fn usage(&self) -> [BudgetUsage; 2] {
        self.usage
    }
}

/// Lazy complete time family with one shared immutable numerical pi constant.
#[derive(Clone, Debug)]
pub struct LcTimeProjection<P: Scalar>(Rc<Data<P>>);

/// Original exact and numerical authorities are never reconstructed for each member.
#[derive(Debug)]
struct Data<P: Scalar> {
    /// Complete unbounded family and original physical declaration.
    cover: LcZeroTimes,
    /// Shared certified scalar pi, including arithmetic revision.
    pi: PiConstant<P>,
    /// Conservative scoped retained ownership during materialization.
    retained: u64,
}

impl<P: Scalar + 'static> LcTimeProjection<P> {
    /// Admit the complete family without materializing or truncating its indices.
    /// # Errors
    /// Refuses resource exhaustion without losing the caller's exact family.
    pub fn new(
        cover: LcZeroTimes,
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

    /// Complete exact family and original relation.
    #[must_use]
    pub fn cover(&self) -> &LcZeroTimes {
        &self.0.cover
    }

    /// Original shared numerical pi authority.
    #[must_use]
    pub fn pi(&self) -> &PiConstant<P> {
        &self.0.pi
    }

    /// Enclose any nonnegative integer member, including the separate exact epoch.
    /// # Errors
    /// Refuses invalid indices, changed arithmetic revisions or exhausted resources.
    pub fn member(
        &self,
        index: Rational,
        literals: LiteralLimits,
        context: &ArithmeticContext,
        budget: &mut Budget,
    ) -> Result<LcNumericalTime<P>, LcProjectionError> {
        let before = budget.usage();
        let retained = self.preflight(context, budget)?;
        budget.with_retained_payload(retained, AT, |budget| {
            let exact = self.0.cover.member(index, budget)?;
            let [lower, upper] = exact.endpoints();
            let endpoints = [
                lower::endpoint(lower, &self.0.pi, literals, context, budget)?,
                lower::endpoint(upper, &self.0.pi, literals, context, budget)?,
            ];
            Ok(LcNumericalTime {
                projection: self.clone(),
                exact,
                endpoints,
                usage: [before, budget.usage()],
            })
        })
    }

    /// Enclose a checked tail from this identical complete time question.
    /// # Errors
    /// Refuses a foreign question, changed arithmetic revision or exhausted resources.
    pub fn tail(
        &self,
        exact: LcTimeTail,
        literals: LiteralLimits,
        context: &ArithmeticContext,
        budget: &mut Budget,
    ) -> Result<LcNumericalTail<P>, LcProjectionError> {
        let before = budget.usage();
        let retained = self.preflight(context, budget)?;
        if !exact.cover().shares_allocation(&self.0.cover) {
            return Err(LcRelationError::Containment.into());
        }
        let retained = retained
            .saturating_add(exact.mathematical().retained_payload_bytes())
            .saturating_add(
                exact
                    .existence_witness()
                    .map_or(0, nrz_bindings::LcWitness::retained_payload_bytes),
            );
        budget.with_retained_payload(retained, AT, |budget| {
            let start = lower::endpoint(
                exact.mathematical().start(),
                &self.0.pi,
                literals,
                context,
                budget,
            )?;
            Ok(LcNumericalTail {
                projection: self.clone(),
                exact,
                start,
                usage: [before, budget.usage()],
            })
        })
    }

    /// Charge all checks and bound scoped exact endpoint ownership before allocation.
    fn preflight(
        &self,
        context: &ArithmeticContext,
        budget: &mut Budget,
    ) -> Result<u64, LcProjectionError> {
        budget.work(1, AT)?;
        if context.revision() != self.0.pi.arithmetic_revision() {
            return Err(LcProjectionError::Revision);
        }
        let bytes = budget
            .limits()
            .component_bits
            .div_ceil(8)
            .saturating_mul(16)
            .saturating_add(4_096);
        budget.check(BudgetKind::OutputBytes, bytes, AT)?;
        Ok(self.0.retained.saturating_add(bytes))
    }
}
