// SPDX-License-Identifier: AGPL-3.0-only
//! Numerical physical marginals retain their original shared-clock question.

use super::{AT, LcNumericalQuantity, LcProjectionError, pairs};
use nrz_bindings::readings::algebra::limits::{BudgetKind, BudgetUsage};
use nrz_bindings::{Budget, LcClockCover, LcQuantity};
use nrz_scalar::{ArithmeticContext, LiteralLimits, Scalar};
use std::rc::Rc;

/// Atomic numerical marginals preserve the same exact joint pair/rate/scale constraints.
#[derive(Clone, Debug)]
pub struct LcClockProjection<P: Scalar>(Rc<Data<P>>);

/// Every requested quantity publishes together, without inventing marginal independence.
#[derive(Debug)]
struct Data<P: Scalar> {
    /// Complete original relation, cycle proof and exact nonemptiness witness.
    cover: LcClockCover,
    /// All requested numerical quantities, in the original query order.
    quantities: Vec<LcNumericalQuantity<P>>,
    /// Arithmetic revision used for every outward decoding operation.
    revision: u32,
    /// Original cumulative numerical construction snapshots.
    usage: [BudgetUsage; 2],
    /// Conservative newly retained endpoint and metadata allowance.
    bytes: u64,
}

impl<P: Scalar> LcClockProjection<P> {
    /// Enclose every requested supported marginal on the caller's cumulative ledger.
    /// # Errors
    /// Refuses an unsupported projection, numerical decoding or resource exhaustion,
    /// without exposing a successful query prefix.
    pub fn new(
        cover: LcClockCover,
        quantities: &[LcQuantity],
        literals: LiteralLimits,
        context: &ArithmeticContext,
        budget: &mut Budget,
    ) -> Result<Self, LcProjectionError> {
        let before = budget.usage();
        budget.work(1, AT)?;
        let count = u64::try_from(quantities.len()).map_err(|_| LcProjectionError::Capacity)?;
        let per = budget
            .limits()
            .component_bits
            .div_ceil(8)
            .saturating_mul(8)
            .saturating_add(4_096);
        let bytes = count.saturating_mul(per).saturating_add(1_024);
        budget.check(BudgetKind::OutputBytes, bytes, AT)?;
        budget.check(BudgetKind::StagedPayloadBytes, bytes, AT)?;
        let retained = bytes
            .saturating_add(cover.retained_payload_bytes())
            .saturating_add(cover.relation().retained_payload_bytes());
        let values = budget.with_retained_payload(retained, AT, |budget| {
            let mut values = Vec::with_capacity(quantities.len());
            for quantity in quantities {
                budget.work(1, AT)?;
                let exact = cover.bounds(*quantity, budget)?;
                values.push(pairs::lower_quantity(
                    *quantity, exact, literals, context, budget,
                )?);
            }
            Ok::<_, LcProjectionError>(values)
        })?;
        Ok(Self(Rc::new(Data {
            cover,
            quantities: values,
            revision: context.revision(),
            usage: [before, budget.usage()],
            bytes,
        })))
    }

    /// Original exact authority and retained correlations for all marginals.
    #[must_use]
    pub fn cover(&self) -> &LcClockCover {
        &self.0.cover
    }

    /// Complete requested quantity list, in original order.
    #[must_use]
    pub fn quantities(&self) -> &[LcNumericalQuantity<P>] {
        &self.0.quantities
    }

    /// Native arithmetic revision, retained independently of exact admission.
    #[must_use]
    pub fn arithmetic_revision(&self) -> u32 {
        self.0.revision
    }

    /// Before/after cumulative caller snapshots for this numerical lowering.
    #[must_use]
    pub fn usage(&self) -> [BudgetUsage; 2] {
        self.0.usage
    }

    /// Conservative new numerical and exact marginal ownership.
    #[must_use]
    pub fn owned_bytes(&self) -> u64 {
        self.0.bytes
    }
}
