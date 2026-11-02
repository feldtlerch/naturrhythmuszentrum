// SPDX-License-Identifier: AGPL-3.0-only
//! Immutable numerical endpoint regions retain the original exact LC forward cover.

mod lower;

use crate::PiConstant;
use nrz_bindings::readings::algebra::{
    Fault, Location, Rational,
    limits::{BudgetKind, BudgetUsage},
};
use nrz_bindings::{Budget, LcForwardArc, LcForwardCover, LcRelationError};
use nrz_scalar::{ArithmeticContext, ArithmeticFailure, Enclosure, LiteralLimits, Scalar};
use std::rc::Rc;

/// Numerical lowering uses the caller's exact/numerical aggregate ledger.
const AT: Location = Location::new("LC forward numerical cover");

/// A refused complete rendering never exposes a truncated branch family.
#[derive(Debug, thiserror::Error)]
pub enum LcProjectionError {
    /// Exact cover materialization or declaration checking refused.
    #[error("LC forward declaration: {0}")]
    Declaration(#[from] LcRelationError),
    /// Cumulative exact or numerical owner resources refused.
    #[error("LC forward resources: {0}")]
    Resources(#[from] Fault),
    /// Outward arithmetic or exact literal decoding refused.
    #[error("LC forward arithmetic: {0}")]
    Arithmetic(#[from] ArithmeticFailure),
    /// A complete exact family cannot be represented in this finite materialization.
    #[error("LC forward materialization capacity exhausted")]
    Capacity,
    /// The supplied pi region belongs to a different arithmetic revision.
    #[error("LC forward arithmetic revision mismatch")]
    Revision,
}

/// One exact branch and independently enclosed numerical endpoints.
#[derive(Clone, Debug)]
pub struct LcNumericalArc<P: Scalar> {
    /// Exact endpoint expressions, integer identity and half-open face ownership.
    exact: LcForwardArc,
    /// Separate numerical error enclosures for the two exact endpoints.
    endpoints: [Enclosure<P>; 2],
    /// Outer box including both endpoint errors; exact face semantics remain in `exact`.
    enclosure: Enclosure<P>,
}

impl<P: Scalar> LcNumericalArc<P> {
    /// Original branch association and exact [0, 2 pi) convention.
    #[must_use]
    pub const fn exact(&self) -> &LcForwardArc {
        &self.exact
    }

    /// Complete numerical error enclosure for each exact endpoint.
    #[must_use]
    pub const fn endpoints(&self) -> &[Enclosure<P>; 2] {
        &self.endpoints
    }

    /// Outward numerical box; the exact endpoint inclusion flags remain authoritative.
    #[must_use]
    pub const fn enclosure(&self) -> Enclosure<P> {
        self.enclosure
    }
}

/// A complete immutable numerical rendering, with shared exact and pi authorities.
///
/// Numerical values live in this matrix-owned heap allocation with endpoint errors.
/// Inputs are immutable exact declarations; this owner does not authenticate mutable
/// `Heap` readings or provide a publication route for them.
#[derive(Clone, Debug)]
pub struct LcForwardProjection<P: Scalar>(Rc<Data<P>>);

/// All native endpoint boxes are published together only after every branch succeeds.
#[derive(Debug)]
struct Data<P: Scalar> {
    /// Complete exact original relation, parameter map and cycle cover.
    cover: LcForwardCover,
    /// Shared scalar-certified pi region, without a duplicate circle computation.
    pi: PiConstant<P>,
    /// Complete numerical family; no successful prefix can escape a failed constructor.
    arcs: Vec<LcNumericalArc<P>>,
    /// Native arithmetic revision for all endpoint operations.
    revision: u32,
    /// Caller ledger before and after this complete lowering.
    usage: [BudgetUsage; 2],
    /// Conservative newly owned native and exact-arc payload.
    bytes: u64,
}

impl<P: Scalar + 'static> LcForwardProjection<P> {
    /// Enclose every exact arc using scalar arithmetic and the supplied shared pi owner.
    ///
    /// A family beyond finite capacity remains available through `cover`; no artificial
    /// cycle cutoff is inserted. Literal decoding and failed attempts spend the same ledger.
    /// # Errors
    /// Refuses arithmetic revision changes, finite materialization capacity, exact or
    /// numerical resource exhaustion, and outward arithmetic failures, without a partial owner.
    pub fn new(
        cover: LcForwardCover,
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
        let count = count(&cover, budget)?;
        let bytes = payload::<P>(count, budget)?;
        let retained = bytes
            .saturating_add(cover.retained_payload_bytes())
            .saturating_add(cover.relation().retained_payload_bytes())
            .saturating_add(
                u64::try_from(pi.owned_bytes()).map_err(|_| LcProjectionError::Capacity)?,
            );
        let arcs = budget.with_retained_payload(retained, AT, |budget| {
            lower::family(&cover, &pi, count, literals, context, budget)
        })?;
        Ok(Self(Rc::new(Data {
            cover,
            pi,
            arcs,
            revision: context.revision(),
            usage: [before, budget.usage()],
            bytes,
        })))
    }

    /// Original complete exact relation and certified covering family.
    #[must_use]
    pub fn cover(&self) -> &LcForwardCover {
        &self.0.cover
    }

    /// Every branch in the declared inclusive integer range, in index order.
    #[must_use]
    pub fn arcs(&self) -> &[LcNumericalArc<P>] {
        &self.0.arcs
    }

    /// Shared original pi region and its independent scalar enclosure evidence.
    #[must_use]
    pub fn pi(&self) -> &PiConstant<P> {
        &self.0.pi
    }

    /// Arithmetic revision retained with all native endpoint errors.
    #[must_use]
    pub fn arithmetic_revision(&self) -> u32 {
        self.0.revision
    }

    /// Before/after caller counters, excluding construction of already shared owners.
    #[must_use]
    pub fn usage(&self) -> [BudgetUsage; 2] {
        self.0.usage
    }

    /// Conservative new ownership, excluding the shared exact cover and pi region.
    #[must_use]
    pub fn owned_bytes(&self) -> u64 {
        self.0.bytes
    }
}

/// Exact index subtraction determines the complete finite materialization size.
fn count(cover: &LcForwardCover, budget: &mut Budget) -> Result<usize, LcProjectionError> {
    let width = cover.cycles()[1]
        .sub(&cover.cycles()[0], budget)?
        .add(&Rational::one(), budget)?;
    let (count, denominator) = width
        .small_ratio(budget)?
        .ok_or(LcProjectionError::Capacity)?;
    if denominator != 1 {
        return Err(LcProjectionError::Capacity);
    }
    usize::try_from(count).map_err(|_| LcProjectionError::Capacity)
}

/// Preflight the complete native allocation and conservative exact rational arc payload.
fn payload<P: Scalar>(count: usize, budget: &mut Budget) -> Result<u64, LcProjectionError> {
    let count = u64::try_from(count).map_err(|_| LcProjectionError::Capacity)?;
    let limbs = budget
        .limits()
        .component_bits
        .div_ceil(8)
        .saturating_mul(10);
    let per_arc = u64::try_from(size_of::<LcNumericalArc<P>>())
        .map_err(|_| LcProjectionError::Capacity)?
        .saturating_add(limbs)
        .saturating_add(2_048);
    let bytes = count.saturating_mul(per_arc).saturating_add(1_024);
    budget.check(BudgetKind::OutputBytes, bytes, AT)?;
    budget.check(BudgetKind::StagedPayloadBytes, bytes, AT)?;
    budget.work(count, AT)?;
    Ok(bytes)
}
