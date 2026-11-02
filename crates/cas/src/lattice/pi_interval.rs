// SPDX-License-Identifier: AGPL-3.0-only
//! Closed rational-interval images in the exact 2 pi floor lattice, without heap values.

mod cuts;

use crate::limits::{Budget, BudgetKind};
use crate::pi_bounds::{CheckedPiBounds, PiBoundCertificate};
use crate::{Fault, InvalidCode, Location, Rational};

/// This closed certificate owns mathematical endpoints and integer domains only.
const AT: Location = Location::new("pi lattice interval cover");

/// An exact affine-pi endpoint; its separate coefficients support outward numerical lowering.
#[derive(Clone, Debug)]
pub struct PiAffineEndpoint {
    /// Rational radian-coordinate offset.
    rational: Rational,
    /// Exact rational coefficient of mathematical pi.
    pi: Rational,
}

impl PiAffineEndpoint {
    /// Mathematical coefficients created by the closed lattice checkers.
    pub(super) const fn affine(rational: Rational, pi: Rational) -> Self {
        Self { rational, pi }
    }

    /// Rational and pi coefficients, without a rounded spelling of the circle constant.
    #[must_use]
    pub const fn coefficients(&self) -> (&Rational, &Rational) {
        (&self.rational, &self.pi)
    }
}

/// One nonempty arc of a complete floor representative family in [0, 2 pi).
#[derive(Clone, Debug)]
pub struct PiIntervalArc {
    /// Original exact integer lift, never a floating cycle counter.
    index: Rational,
    /// Included lower endpoint.
    lower: PiAffineEndpoint,
    /// Upper endpoint, whose face ownership is separately retained.
    upper: PiAffineEndpoint,
    /// Internal full-turn faces belong to the following cell rather than this arc.
    upper_included: bool,
}

impl PiIntervalArc {
    /// Exact cycle index of this arc.
    #[must_use]
    pub const fn index(&self) -> &Rational {
        &self.index
    }

    /// Exact lower and upper endpoints; the lower endpoint is always included.
    #[must_use]
    pub const fn endpoints(&self) -> (&PiAffineEndpoint, &PiAffineEndpoint) {
        (&self.lower, &self.upper)
    }

    /// Whether the upper endpoint belongs to this original floor representative.
    #[must_use]
    pub const fn upper_included(&self) -> bool {
        self.upper_included
    }
}

/// A checked complete finite cover of a closed rational interval by 2 pi floor cells.
///
/// The exact endpoint indices are proved, not adopted as search bounds. Every integer
/// between them is retained. This closed mathematical certificate does not introduce
/// native case-membership judgments or authenticate any physical source or heap value.
#[derive(Clone, Debug)]
pub struct PiIntervalCover {
    /// Original closed rational interval, before reduction modulo a full turn.
    interval: [Rational; 2],
    /// Inclusive complete first/last cell indices.
    indices: [Rational; 2],
    /// Independently checked Machin instance used for both floor inequalities.
    pi: CheckedPiBounds,
    /// Conservative retained exact certificate ownership.
    bytes: u64,
}

impl PiIntervalCover {
    /// Verify ordered rational endpoints and the complete proposed floor-cell range.
    /// # Errors
    /// Refuses reversed intervals, noninteger indices, insufficient floor evidence,
    /// false pi certificates and exhausted caller resources, with no partial cover.
    pub fn check(
        interval: [Rational; 2],
        indices: [Rational; 2],
        pi: &PiBoundCertificate,
        budget: &mut Budget,
    ) -> Result<Self, Fault> {
        budget.work(1, AT)?;
        let pi = pi.check(budget)?;
        let bytes = interval
            .iter()
            .chain(&indices)
            .fold(256_u64, |bytes, value| {
                bytes.saturating_add(rational_bytes(value))
            })
            .saturating_add(pi.retained_bytes());
        retained(bytes, budget)?;
        budget.with_retained_payload(bytes, AT, |budget| {
            cuts::cover(&interval, &indices, &pi, budget)
        })?;
        Ok(Self {
            interval,
            indices,
            pi,
            bytes,
        })
    }

    /// Original closed interval, never replaced by its wrapped hull.
    #[must_use]
    pub const fn interval(&self) -> &[Rational; 2] {
        &self.interval
    }

    /// Inclusive endpoints of the complete mathematical integer index domain.
    #[must_use]
    pub const fn indices(&self) -> &[Rational; 2] {
        &self.indices
    }

    /// Independently checked original constant evidence for the floor inequalities.
    #[must_use]
    pub const fn pi_bounds(&self) -> &CheckedPiBounds {
        &self.pi
    }

    /// Conservative exact ownership of this closed certificate.
    #[must_use]
    pub const fn retained_payload_bytes(&self) -> u64 {
        self.bytes
    }

    /// Materialize one exact range member, without choosing a finite prefix as the answer.
    /// # Errors
    /// Refuses noninteger/out-of-range indices and exhausted caller resources.
    pub fn arc(&self, index: Rational, budget: &mut Budget) -> Result<PiIntervalArc, Fault> {
        budget.with_retained_payload(self.bytes, AT, |budget| {
            cuts::arc(&self.interval, &self.indices, index, budget)
        })
    }
}

/// Conservative exact limb storage, separate from any numerical materialization.
pub(super) fn rational_bytes(value: &Rational) -> u64 {
    let (numerator, denominator) = value.component_bits();
    numerator
        .div_ceil(8)
        .saturating_add(denominator.div_ceil(8))
        .saturating_add(128)
}

/// A complete checked record must fit both exact output and staged-payload ceilings.
pub(super) fn retained(bytes: u64, budget: &Budget) -> Result<(), Fault> {
    budget.check(BudgetKind::OutputBytes, bytes, AT)?;
    budget.check(BudgetKind::StagedPayloadBytes, bytes, AT)
}

/// Unproved or false floor coverage never manufactures a mathematical branch.
const fn invalid(code: InvalidCode) -> Fault {
    Fault::invalid(code, AT)
}
