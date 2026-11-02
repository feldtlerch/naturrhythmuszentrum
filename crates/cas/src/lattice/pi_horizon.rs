// SPDX-License-Identifier: AGPL-3.0-only
//! Eventual full-turn thresholds of a checked nondecreasing affine width.

use super::PiAffineEndpoint;
use super::pi_interval::{rational_bytes, retained};
use crate::{Fault, InvalidCode, Location, Rational, limits::Budget, pi_bounds::CheckedPiBounds};
use std::cmp::Ordering;

/// All arithmetic and coefficient inspection spend the caller's cumulative allowance.
const AT: Location = Location::new("affine full-turn horizon");

/// Exact eventual threshold, distinct from an inverse connected suffix.
#[derive(Clone, Debug)]
pub enum PiHorizonValue {
    /// Included first time at or after the declared future reference.
    Finite(PiAffineEndpoint),
    /// Constant width is proved strictly smaller than one turn.
    Infinite,
    /// The supplied pi evidence does not settle full coverage at the future reference.
    Undecided,
}

/// A mathematical certificate for `width(t)=offset+slope*t`, restricted to `t>=reference`.
#[derive(Clone, Debug)]
pub struct PiWidthHorizon {
    /// Original rational offset and nonnegative slope.
    width: [Rational; 2],
    /// Declared future domain; an earlier full hull is not this question's threshold.
    reference: Rational,
    /// Derived exact width at the reference, proved nonnegative.
    initial_width: Rational,
    /// Exact finite, infinite or unresolved result.
    value: PiHorizonValue,
    /// Independently checked constant evidence, reused from its preparing owner.
    pi: CheckedPiBounds,
    /// Conservative exact ownership.
    bytes: u64,
}

impl PiWidthHorizon {
    /// Derive the eventual threshold of a nonnegative nondecreasing future width.
    ///
    /// A positive slope and initial width below a turn give `(2*pi-offset)/slope`.
    /// Zero slope below a turn gives infinity. An initially full turn gives the reference.
    /// # Errors
    /// Refuses a negative slope/initial width and exhausted caller resources.
    pub fn check(
        width: [Rational; 2],
        reference: Rational,
        pi: CheckedPiBounds,
        budget: &mut Budget,
    ) -> Result<Self, Fault> {
        budget.work(1, AT)?;
        for value in width.iter().chain(std::iter::once(&reference)) {
            value.check(budget, AT)?;
        }
        let initial_width = width[0].add(&width[1].mul(&reference, budget)?, budget)?;
        if width[1].is_negative() || initial_width.is_negative() {
            return Err(Fault::invalid(InvalidCode::EndpointMismatch, AT));
        }
        let value = threshold(&width, &reference, &initial_width, &pi, budget)?;
        let mut bytes = width
            .iter()
            .chain([&reference, &initial_width])
            .fold(pi.retained_bytes().saturating_add(512), |sum, v| {
                sum.saturating_add(rational_bytes(v))
            });
        if let PiHorizonValue::Finite(endpoint) = &value {
            let (offset, coefficient) = endpoint.coefficients();
            bytes = bytes
                .saturating_add(rational_bytes(offset))
                .saturating_add(rational_bytes(coefficient));
        }
        retained(bytes, budget)?;
        Ok(Self {
            width,
            reference,
            initial_width,
            value,
            pi,
            bytes,
        })
    }

    /// Original affine width coefficients, in offset/slope order.
    #[must_use]
    pub const fn width(&self) -> &[Rational; 2] {
        &self.width
    }

    /// Beginning of the future domain for which eventual coverage is certified.
    #[must_use]
    pub const fn reference(&self) -> &Rational {
        &self.reference
    }

    /// Nonnegative width at the declared future reference.
    #[must_use]
    pub const fn initial_width(&self) -> &Rational {
        &self.initial_width
    }

    /// Exact threshold or explicit infinite/unresolved outcome.
    #[must_use]
    pub const fn value(&self) -> &PiHorizonValue {
        &self.value
    }

    /// Shared prepared mathematical constant evidence.
    #[must_use]
    pub const fn pi_bounds(&self) -> &CheckedPiBounds {
        &self.pi
    }

    /// Conservative retained exact ownership.
    #[must_use]
    pub const fn retained_payload_bytes(&self) -> u64 {
        self.bytes
    }
}

/// Exact comparisons distinguish the earliest future threshold from a transient earlier hull.
fn threshold(
    width: &[Rational; 2],
    reference: &Rational,
    initial: &Rational,
    pi: &CheckedPiBounds,
    budget: &mut Budget,
) -> Result<PiHorizonValue, Fault> {
    let two = Rational::from_ratio(2, 1, budget)?;
    let below = initial.compare(&pi.contract().lower.mul(&two, budget)?, budget)? == Ordering::Less;
    if below {
        return if width[1].is_zero() {
            Ok(PiHorizonValue::Infinite)
        } else {
            Ok(PiHorizonValue::Finite(PiAffineEndpoint::affine(
                Rational::zero()
                    .sub(&width[0], budget)?
                    .div(&width[1], budget)?,
                two.div(&width[1], budget)?,
            )))
        };
    }
    let full = initial.compare(&pi.contract().upper.mul(&two, budget)?, budget)? != Ordering::Less;
    Ok(if full {
        PiHorizonValue::Finite(PiAffineEndpoint::affine(
            reference.clone(),
            Rational::zero(),
        ))
    } else {
        PiHorizonValue::Undecided
    })
}
