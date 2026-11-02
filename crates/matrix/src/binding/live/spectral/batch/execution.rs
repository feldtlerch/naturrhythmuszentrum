// SPDX-License-Identifier: AGPL-3.0-only
//! Each admitted invariant executes once; independent successes survive later failure.

use super::{
    AT, ArithmeticContext, Budget, Heap, HeapError, LcSpectralBatch, LcSpectralEvaluation,
    LcSpectralStatus as Status, Scalar, SpectralError,
};
use nrz_bindings::readings::algebra::limits::BudgetKind;

impl<P: Scalar + 'static> LcSpectralBatch<P> {
    /// Attempt the closed invariant stratum once on the caller's cumulative ledger.
    ///
    /// Ordinary node refusals remain in the complete row table. A resource refusal
    /// blocks the unexecuted suffix, while earlier independent successes remain.
    /// There are no undeclared retries, default branches or fresh child ledgers.
    /// # Errors
    /// Refuses a second attempt or a foreign owner/arithmetic context. These preflight
    /// refusals consume the owner's execution attempt without erasing earlier state.
    pub fn execute(
        &mut self,
        heap: &Heap,
        context: &ArithmeticContext,
        budget: &mut Budget,
    ) -> Result<(), SpectralError> {
        if self.attempted {
            return Err(SpectralError::Identity);
        }
        self.attempted = true;
        let before = budget.usage();
        if let Err(cause) = self.check_owner(heap, context) {
            self.fail_coefficients(cause.clone());
            self.usage = Some([before, budget.usage()]);
            return Err(cause);
        }
        let resident = self.resident_bytes()?;
        self.prepare_coefficients(context, budget, resident);
        if self.coefficients.values.is_none() {
            self.usage = Some([before, budget.usage()]);
            return Ok(());
        }
        for index in 0..self.entries.len() {
            let result = budget.with_retained_payload(resident, AT, |budget| {
                budget.check(
                    BudgetKind::OutputBytes,
                    self.inventory.observed_capacity_bytes,
                    AT,
                )?;
                budget.work(1, AT)?;
                let constants = self
                    .coefficients
                    .values
                    .as_ref()
                    .ok_or(SpectralError::Identity)?;
                heap.evaluate_lc_spectrum_from_constants(
                    &self.entries[index].plan,
                    constants,
                    context,
                    budget,
                )
            });
            match result {
                Ok(value) => {
                    self.entries[index].result = Some(value);
                    self.entries[index].status = Status::Successful;
                }
                Err(cause) if resource(&cause) => {
                    self.stop(index, cause);
                    break;
                }
                Err(cause) => {
                    self.entries[index].status = classify(&cause);
                    self.entries[index].failure = Some(cause);
                }
            }
        }
        self.usage = Some([before, budget.usage()]);
        Ok(())
    }

    /// Return the original complete producer only after current live and active-limit validation.
    ///
    /// Dependent adapters must consume this evaluation's original `plan()`; an independently
    /// rebound equal input does not become that producer's original publication identity.
    /// # Errors
    /// Refuses missing or unsuccessful rows, foreign/stale inputs and exhausted active limits.
    pub fn result<'a>(
        &'a self,
        row: usize,
        heap: &Heap,
        context: &ArithmeticContext,
        budget: &mut Budget,
    ) -> Result<&'a LcSpectralEvaluation<P>, SpectralError> {
        self.check_owner(heap, context)?;
        budget.work(1, AT)?;
        let row = self.rows.get(row).ok_or(SpectralError::Identity)?;
        let entry = &self.entries[row.binding];
        let result = entry.result.as_ref().ok_or(SpectralError::Identity)?;
        budget.with_retained_payload(self.resident_bytes()?, AT, |budget| {
            budget.check(
                BudgetKind::OutputBytes,
                self.inventory.observed_capacity_bytes,
                AT,
            )?;
            budget.work(2, AT)?;
            result.validate(heap, context)
        })?;
        Ok(result)
    }

    /// No heap occupation or numerical arithmetic context crosses this evaluation scope.
    fn check_owner(&self, heap: &Heap, context: &ArithmeticContext) -> Result<(), SpectralError> {
        if self.header.scope != heap.scope {
            return Err(HeapError::ForeignHandle.into());
        }
        if self.header.revision != context.revision() || self.header.precision != P::PRECISION {
            return Err(SpectralError::Identity);
        }
        Ok(())
    }

    /// Retain the actual refused node and name it as the cause of every unexecuted successor.
    fn stop(&mut self, index: usize, cause: SpectralError) {
        if let Some(entry) = self.entries.get_mut(index) {
            entry.status = classify(&cause);
            entry.failure = Some(cause);
            for entry in self.entries.iter_mut().skip(index + 1) {
                entry.status = Status::Blocked;
                entry.blocked_by = Some(index);
            }
        }
    }
}

/// Exact resource exhaustion and heap capacity stop the shared schedule without local fake failures.
const fn resource(cause: &SpectralError) -> bool {
    matches!(
        cause,
        SpectralError::Exact(nrz_bindings::readings::algebra::Fault {
            kind: nrz_bindings::readings::algebra::FaultKind::Exhausted { .. },
            ..
        }) | SpectralError::Heap(HeapError::Capacity | HeapError::ExtentOverflow)
    )
}

/// Domain, live-binding and numerical separation failures remain distinct historical outcomes.
pub(super) const fn classify(cause: &SpectralError) -> Status {
    if resource(cause) {
        return Status::ResourceRefused;
    }
    match cause {
        SpectralError::NonpositiveInput { .. } => Status::DomainUnavailable,
        SpectralError::Arithmetic(_)
        | SpectralError::InputNotSeparated { .. }
        | SpectralError::NotSeparated => Status::NumericallyFailed,
        SpectralError::Operator(_)
        | SpectralError::Law(_)
        | SpectralError::Reading(_)
        | SpectralError::Live(_)
        | SpectralError::Derivative(_)
        | SpectralError::Joint(_)
        | SpectralError::Heap(_)
        | SpectralError::Exact(_)
        | SpectralError::Identity => Status::BindingRefused,
    }
}
