// SPDX-License-Identifier: AGPL-3.0-only
//! Scalar endpoint errors retain the exact original rest question and explicit undefinedness.

use super::{AT, LcNumericalQuantity, LcProjectionError, pairs};
use nrz_bindings::readings::algebra::limits::{BudgetKind, BudgetUsage};
use nrz_bindings::{Budget, LcQuiescentOutcome, LcQuiescentQuery};
use nrz_scalar::{ArithmeticContext, LiteralLimits, Scalar};

/// One quiescent query with separate native endpoint errors, or explicit physical undefinedness.
#[derive(Clone, Debug)]
pub struct LcQuiescentProjection<P: Scalar> {
    /// Exact query retaining every original constraint and a full nonemptiness witness.
    exact: LcQuiescentQuery,
    /// Absent only when the exact outcome is undefined phase or undefined ratio.
    scalar: Option<LcNumericalQuantity<P>>,
    /// Arithmetic revision used for every endpoint decoding operation.
    revision: u32,
    /// Caller counters before and after the complete rendering.
    usage: [BudgetUsage; 2],
    /// Conservative new endpoint and metadata ownership.
    bytes: u64,
}

impl<P: Scalar> LcQuiescentProjection<P> {
    /// Enclose one exact scalar image without replacing undefined outcomes by zero.
    /// # Errors
    /// Refuses unsupported endpoint decoding and exhausted caller resources.
    pub fn new(
        exact: LcQuiescentQuery,
        literals: LiteralLimits,
        context: &ArithmeticContext,
        budget: &mut Budget,
    ) -> Result<Self, LcProjectionError> {
        let before = budget.usage();
        budget.work(1, AT)?;
        let bytes = budget
            .limits()
            .component_bits
            .div_ceil(8)
            .saturating_mul(8)
            .saturating_add(4_096);
        let retained = bytes
            .saturating_add(exact.cover().retained_payload_bytes())
            .saturating_add(exact.cover().relation().retained_payload_bytes());
        budget.check(BudgetKind::OutputBytes, bytes, AT)?;
        budget.check(BudgetKind::StagedPayloadBytes, bytes, AT)?;
        let scalar =
            budget.with_retained_payload(retained, AT, |budget| match exact.outcome() {
                LcQuiescentOutcome::Defined(interval) => {
                    Ok::<_, LcProjectionError>(Some(pairs::lower_quantity(
                        exact.quantity(),
                        interval.clone(),
                        literals,
                        context,
                        budget,
                    )?))
                }
                LcQuiescentOutcome::UndefinedPhase | LcQuiescentOutcome::UndefinedRatio => Ok(None),
            })?;
        Ok(Self {
            exact,
            scalar,
            revision: context.revision(),
            usage: [before, budget.usage()],
            bytes,
        })
    }

    /// Original joint source, quantity and distinct defined/undefined verdict.
    #[must_use]
    pub const fn exact(&self) -> &LcQuiescentQuery {
        &self.exact
    }

    /// Native scalar endpoints only when the exact physical quantity is defined.
    #[must_use]
    pub const fn scalar(&self) -> Option<&LcNumericalQuantity<P>> {
        self.scalar.as_ref()
    }

    /// Arithmetic revision retained independently of the exact physical evidence.
    #[must_use]
    pub const fn arithmetic_revision(&self) -> u32 {
        self.revision
    }

    /// Cumulative construction snapshots, excluding previously prepared exact owners.
    #[must_use]
    pub const fn usage(&self) -> [BudgetUsage; 2] {
        self.usage
    }

    /// Conservative newly owned endpoint and metadata payload.
    #[must_use]
    pub const fn owned_bytes(&self) -> u64 {
        self.bytes
    }
}
