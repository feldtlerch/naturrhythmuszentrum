// SPDX-License-Identifier: AGPL-3.0-only
//! One source-free producer precedes the binding-wide root stratum.

use super::{
    AT, ArithmeticContext, Budget, LcSpectralBatch, LcSpectralStatus as Status, Scalar,
    SpectralError, execution::classify,
};
use nrz_bindings::readings::algebra::limits::{BudgetKind, BudgetUsage};
use nrz_scalar::DecodedLiteral;

/// Fixed coefficient/result/status storage is included in the new batch owner's inventory.
#[derive(Debug)]
pub(super) struct Coefficients<P: Scalar> {
    /// Original source-free node outcome; no root executes until it succeeds.
    pub(super) status: Status,
    /// One immutable fixed array, shared by borrow across all actual bindings.
    pub(super) values: Option<[DecodedLiteral<P>; 3]>,
    /// Original cause of every root blocked by this node.
    pub(super) failure: Option<SpectralError>,
    /// Complete source-free attempted prefix on the same caller ledger.
    pub(super) usage: Option<[BudgetUsage; 2]>,
}

impl<P: Scalar> Coefficients<P> {
    /// Reserve an unavailable result slot before executing the finite schedule.
    pub(super) const fn pending() -> Self {
        Self {
            status: Status::Pending,
            values: None,
            failure: None,
            usage: None,
        }
    }
}

impl<P: Scalar> LcSpectralBatch<P> {
    /// Every unexecuted root explicitly depends on the refused original source-free producer.
    pub(super) fn fail_coefficients(&mut self, cause: SpectralError) {
        self.coefficients.status = classify(&cause);
        self.coefficients.failure = Some(cause);
        for entry in &mut self.entries {
            entry.status = Status::Blocked;
        }
    }

    /// Exact constants, literal policy and arithmetic revision are shared over the complete family.
    pub(super) fn prepare_coefficients(
        &mut self,
        context: &ArithmeticContext,
        budget: &mut Budget,
        resident: u64,
    ) {
        let before = budget.usage();
        let result = budget.with_retained_payload(resident, AT, |budget| {
            budget.check(
                BudgetKind::OutputBytes,
                self.inventory.observed_capacity_bytes,
                AT,
            )?;
            let temporary = u64::try_from(size_of::<[DecodedLiteral<P>; 3]>())
                .map_err(|_| super::HeapError::ExtentOverflow)?;
            budget.check(BudgetKind::StagedPayloadBytes, temporary, AT)?;
            budget.work(1, AT)?;
            super::super::numeric::schema_constants(&self.header.schema, context, budget)
        });
        self.coefficients.usage = Some([before, budget.usage()]);
        match result {
            Ok(values) => {
                self.coefficients.values = Some(values);
                self.coefficients.status = Status::Successful;
            }
            Err(cause) => self.fail_coefficients(cause),
        }
    }
}
