// SPDX-License-Identifier: AGPL-3.0-only
//! Joined local attempts describe native attachments without creating mathematical cache authority.

use super::{LcFrequencyState, LcFrequencyTarget, LcInputProduct, LcProductError};
use crate::{ExecutionAccount, LoanError};
use nrz_scalar::Scalar;
use std::sync::atomic::{AtomicU64, Ordering};
use tracing::Level;

/// Process-local observation identities are distinct even if a subscriber recycles span identifiers.
static NEXT_ATTEMPT: AtomicU64 = AtomicU64::new(1);

impl<P: Scalar + 'static, const N: usize> LcInputProduct<'_, P, N> {
    /// Observe one complete public attempt without changing its result or original ledger charges.
    pub(super) fn observe_attempt<R>(
        &self,
        cell: usize,
        consumer: &'static str,
        account: &mut ExecutionAccount,
        run: impl FnOnce(&mut ExecutionAccount) -> Result<R, LcProductError>,
        state: impl FnOnce(&R) -> LcFrequencyState<P>,
    ) -> Result<R, LcProductError> {
        let span = tracing::span!(target: "nrz::lc", Level::TRACE, "nrz.barrier",
            family = "lc", revision = 1_u64, owner = "nrz-matrix",
            question = "frequency", operation = "positive-domain", consumer,
            scope = "local-represented-inputs", account = account.identity,
            schema = self.cells.readers.inventory.input(0).map(crate::loan::lc_shape::LcInputShape::specialization_key),
            inductance = self.positions[0], capacitance = self.positions[1],
            axes = ?self.cells.readers.axes, cell, format = ?P::PRECISION,
            arithmetic_revision = self.context.revision(),
            attempt = tracing::field::Empty, work = account.usage().work,
            outcome = tracing::field::Empty, reason = tracing::field::Empty);
        if span.is_disabled() {
            return run(account);
        }
        let Ok(attempt) = NEXT_ATTEMPT.try_update(Ordering::Relaxed, Ordering::Relaxed, |next| {
            next.checked_add(1)
        }) else {
            tracing::event!(name: "nrz.observer.loss", target: "nrz::lc", parent: &span,
                Level::TRACE, owner = "nrz-matrix", reason = "attempt-identity-exhausted");
            span.record("outcome", "incomplete");
            return run(account);
        };
        span.record("attempt", attempt);
        span.in_scope(|| {
            decision(
                attempt,
                "attempted",
                "none",
                "none",
                "none",
                account.usage().work,
            );
            self.claims(attempt);
            let result = run(account);
            let (outcome, target, reason, continuation) = match &result {
                Ok(result) => outcome(state(result)),
                Err(error) => ("refused", "none", refusal(error), "none"),
            };
            span.record("work", account.usage().work);
            span.record("outcome", outcome);
            span.record("reason", reason);
            decision(
                attempt,
                outcome,
                target,
                reason,
                continuation,
                account.usage().work,
            );
            result
        })
    }

    /// Complete admitted metadata is available without inspecting any numerical plane or mask bit.
    fn claims(&self, attempt: u64) {
        for (position, reader) in self.cells.readers.readers.iter().enumerate() {
            if let Some(reader) = reader {
                tracing::event!(name: "nrz.claim", target: "nrz::lc", Level::TRACE,
                    family = "lc", attempt, claim = "admitted-input-attachment", position,
                    schema = self.cells.readers.inventory.input(position).map(crate::loan::lc_shape::LcInputShape::specialization_key),
                    account = reader.account, occupation = ?reader.bundle.identity,
                    companions = ?reader.bundle.qualified,
                    source = ?reader.bundle.origin.evidence.as_ref().map(|origin| origin.source_identity),
                    source_companions = ?reader.bundle.origin.evidence.as_ref().map(|origin| origin.attachments),
                    error_scope = ?reader.numerical_error_scope());
            }
        }
    }
}

/// Fixed event fields join attempted and terminal observations inside the same barrier span.
fn decision(
    attempt: u64,
    outcome: &'static str,
    target: &'static str,
    reason: &'static str,
    continuation: &'static str,
    work: u64,
) {
    tracing::event!(name: "nrz.decision", target: "nrz::lc", Level::TRACE,
        family = "lc", attempt, outcome, target, reason, continuation, work);
}

/// A local native outcome makes no full-feasibility or exact-equality assertion.
const fn outcome<P: Scalar>(
    state: LcFrequencyState<P>,
) -> (&'static str, &'static str, &'static str, &'static str) {
    match state {
        LcFrequencyState::Ready { .. } => ("completed", "none", "positive", "local-ready"),
        LcFrequencyState::Undecided(value) => (
            "undecided",
            target(value),
            "no-strict-positive-separation",
            "retain-unresolved-inputs",
        ),
        LcFrequencyState::Invalid(value) => (
            "invalid",
            target(value),
            "strict-negative-box",
            "invalid-domain",
        ),
    }
}

/// Targets are original positional roles or their local product/root, never numerical values.
const fn target(target: LcFrequencyTarget) -> &'static str {
    match target {
        LcFrequencyTarget::Inductance => "inductance",
        LcFrequencyTarget::Capacitance => "capacitance",
        LcFrequencyTarget::Product => "product",
        LcFrequencyTarget::Root => "root",
    }
}

/// Preserve the relevant refusal categories without formatting native readings or error payloads.
const fn refusal(error: &LcProductError) -> &'static str {
    match error {
        LcProductError::Loan(LoanError::ForeignAccount) => "foreign-account",
        LcProductError::Loan(LoanError::CoverageMismatch) => "coverage",
        LcProductError::Loan(LoanError::LayoutMismatch) => "capacity-or-layout",
        LcProductError::Loan(LoanError::Budget(_)) => "budget",
        LcProductError::Loan(LoanError::UnsupportedContract) => "unsupported-contract",
        LcProductError::Loan(LoanError::CountOverflow) => "count-overflow",
        LcProductError::Loan(_) => "loan",
        LcProductError::Kernel(_) => "arithmetic",
    }
}
