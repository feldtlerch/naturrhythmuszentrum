// SPDX-License-Identifier: AGPL-3.0-only
//! A forward certainty horizon is a separately enclosed duration, never an inferred cutoff.

use super::{AT, LcProjectionError, lower};
use crate::PiConstant;
use nrz_bindings::readings::algebra::limits::{BudgetKind, BudgetUsage};
use nrz_bindings::{Budget, LcHorizon, LcHorizonValue};
use nrz_scalar::{ArithmeticContext, Enclosure, LiteralLimits, Scalar};

/// Exact joint-source derivation with a separate both-format numerical duration enclosure.
#[derive(Clone, Debug)]
pub struct LcHorizonProjection<P: Scalar> {
    /// Original relation, future domain and finite/infinite/unresolved exact value.
    exact: LcHorizon,
    /// One shared numerical circle authority, reused for the affine endpoint.
    pi: PiConstant<P>,
    /// Present only for a finite exact value; no floating infinity is fabricated.
    enclosure: Option<Enclosure<P>>,
    /// Cumulative before/after exact and numerical lowering work.
    usage: [BudgetUsage; 2],
    /// Conservative newly owned metadata and endpoint values.
    bytes: u64,
}

impl<P: Scalar + 'static> LcHorizonProjection<P> {
    /// Enclose a finite horizon or retain its explicit infinite/undecided exact outcome.
    /// # Errors
    /// Refuses a changed arithmetic revision and exhausted numerical/exact resources.
    pub fn new(
        exact: LcHorizon,
        pi: PiConstant<P>,
        literals: LiteralLimits,
        context: &ArithmeticContext,
        budget: &mut Budget,
    ) -> Result<Self, LcProjectionError> {
        let before = budget.usage();
        budget.work(1, AT)?;
        if context.revision() != pi.arithmetic_revision() {
            return Err(LcProjectionError::Revision);
        }
        let bytes = u64::try_from(size_of::<Self>())
            .map_err(|_| LcProjectionError::Capacity)?
            .saturating_add(1_024);
        let retained = bytes
            .saturating_add(exact.retained_payload_bytes())
            .saturating_add(exact.source_payload_bytes())
            .saturating_add(
                u64::try_from(pi.owned_bytes()).map_err(|_| LcProjectionError::Capacity)?,
            );
        budget.check(BudgetKind::OutputBytes, bytes, AT)?;
        let enclosure =
            budget.with_retained_payload(retained, AT, |budget| match exact.value() {
                LcHorizonValue::Finite(endpoint) => Ok::<_, LcProjectionError>(Some(
                    lower::endpoint(endpoint, &pi, literals, context, budget)?,
                )),
                LcHorizonValue::Infinite | LcHorizonValue::Undecided => Ok(None),
            })?;
        Ok(Self {
            exact,
            pi,
            enclosure,
            usage: [before, budget.usage()],
            bytes,
        })
    }

    /// Original relation, future reference and exact threshold derivation.
    #[must_use]
    pub const fn exact(&self) -> &LcHorizon {
        &self.exact
    }

    /// Shared scalar-certified circle constant and its arithmetic revision.
    #[must_use]
    pub const fn pi(&self) -> &PiConstant<P> {
        &self.pi
    }

    /// Separate error enclosure for a finite duration; inspect `exact` for other outcomes.
    #[must_use]
    pub const fn enclosure(&self) -> Option<Enclosure<P>> {
        self.enclosure
    }

    /// Work spent only on this numerical lowering, excluding the already prepared owners.
    #[must_use]
    pub const fn usage(&self) -> [BudgetUsage; 2] {
        self.usage
    }

    /// Conservative newly owned metadata, excluding shared source and pi authorities.
    #[must_use]
    pub const fn owned_bytes(&self) -> u64 {
        self.bytes
    }
}
