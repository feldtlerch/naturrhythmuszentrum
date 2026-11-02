// SPDX-License-Identifier: AGPL-3.0-only
//! Complete rational pair-recovery marginals retain their original joint exact cover.

use super::{AT, LcProjectionError, lower};
use nrz_bindings::readings::algebra::limits::{BudgetKind, BudgetUsage};
use nrz_bindings::{Budget, LcInterval, LcPairsCover, LcQuantity};
use nrz_scalar::{ArithmeticContext, Enclosure, LiteralLimits, Scalar};
use std::rc::Rc;

/// A numerical outer marginal; absence of an endpoint is mathematical unboundedness.
#[derive(Clone, Debug)]
pub struct LcNumericalQuantity<P: Scalar> {
    /// Original coherent physical meaning, including angle/rate distinctions.
    quantity: LcQuantity,
    /// Exact outer support with original strict domains retained by the relation.
    exact: LcInterval,
    /// Separate outward numerical error for each finite exact endpoint.
    endpoints: [Option<Enclosure<P>>; 2],
    /// Complete bounded outer box, absent if either mathematical endpoint is unbounded.
    enclosure: Option<Enclosure<P>>,
}

impl<P: Scalar> LcNumericalQuantity<P> {
    /// Original coherent quantity identity.
    #[must_use]
    pub const fn quantity(&self) -> LcQuantity {
        self.quantity
    }

    /// Original exact outer support; unbounded scale is never a floating infinity.
    #[must_use]
    pub const fn exact(&self) -> &LcInterval {
        &self.exact
    }

    /// Numerical errors of the exact endpoints, separately from physical uncertainty.
    #[must_use]
    pub const fn endpoints(&self) -> &[Option<Enclosure<P>>; 2] {
        &self.endpoints
    }

    /// Complete outward interval when both mathematical endpoints are finite.
    #[must_use]
    pub const fn enclosure(&self) -> Option<Enclosure<P>> {
        self.enclosure
    }
}

/// Atomic numerical marginals preserve the same exact joint pair/rate/scale constraints.
#[derive(Clone, Debug)]
pub struct LcPairsProjection<P: Scalar>(Rc<Data<P>>);

/// Every requested quantity publishes together, without inventing marginal independence.
#[derive(Debug)]
struct Data<P: Scalar> {
    /// Complete original relation, cycle proof and exact nonemptiness witness.
    cover: LcPairsCover,
    /// All requested numerical quantities, in the original query order.
    quantities: Vec<LcNumericalQuantity<P>>,
    /// Arithmetic revision used for every outward decoding operation.
    revision: u32,
    /// Original cumulative numerical construction snapshots.
    usage: [BudgetUsage; 2],
    /// Conservative newly retained endpoint and metadata allowance.
    bytes: u64,
}

impl<P: Scalar> LcPairsProjection<P> {
    /// Enclose every requested supported marginal on the caller's cumulative ledger.
    /// # Errors
    /// Refuses an unsupported projection, numerical decoding or resource exhaustion,
    /// without exposing a successful query prefix.
    pub fn new(
        cover: LcPairsCover,
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
                let mut endpoints = [None, None];
                for (i, value) in [exact.lower(), exact.upper()].into_iter().enumerate() {
                    if let Some(value) = value {
                        let rational = value.as_rational().ok_or(LcProjectionError::Capacity)?;
                        endpoints[i] = Some(lower::decode(rational, literals, context, budget)?);
                    }
                }
                let enclosure = if let [Some(lower), Some(upper)] = endpoints {
                    Some(Enclosure::new(lower.lower().get(), upper.upper().get())?)
                } else {
                    None
                };
                values.push(LcNumericalQuantity {
                    quantity: *quantity,
                    exact,
                    endpoints,
                    enclosure,
                });
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
    pub fn cover(&self) -> &LcPairsCover {
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
