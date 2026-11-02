// SPDX-License-Identifier: AGPL-3.0-only
//! Complete nonnegative zero families and exact eventual overlap, without physical values.

use super::PiAffineEndpoint;
use super::pi_interval::{rational_bytes, retained};
use crate::limits::Budget;
use crate::{Fault, InvalidCode, Location, Rational};
use std::cmp::Ordering;

/// The closed checker spends the caller's exact ledger.
const AT: Location = Location::new("pi zero family");

/// One closed interval `2*pi*n / [upper,lower]` with its exact integer identity.
#[derive(Clone, Debug)]
pub struct PiZeroBox {
    /// Nonnegative integer, including the separate zero epoch.
    index: Rational,
    /// Both closed endpoints, in increasing order.
    endpoints: [PiAffineEndpoint; 2],
}

impl PiZeroBox {
    /// Original exact integer identity.
    #[must_use]
    pub const fn index(&self) -> &Rational {
        &self.index
    }

    /// Included lower and upper endpoints.
    #[must_use]
    pub const fn endpoints(&self) -> &[PiAffineEndpoint; 2] {
        &self.endpoints
    }
}

/// All integers `n >= 0`, never a finite materialized prefix.
#[derive(Clone, Debug)]
pub struct PiZeroFamily {
    /// Positive finite rational denominator interval.
    rate: [Rational; 2],
    /// Exact nonnegative spread.
    width: Rational,
    /// Conservative mathematical ownership.
    bytes: u64,
}

impl PiZeroFamily {
    /// Check the positive domain underlying monotone division for every nonnegative integer.
    /// # Errors
    /// Refuses nonpositive/reversed endpoints and exhausted caller resources.
    pub fn check(rate: [Rational; 2], budget: &mut Budget) -> Result<Self, Fault> {
        budget.work(1, AT)?;
        for value in &rate {
            value.check(budget, AT)?;
        }
        if rate[0].is_zero()
            || rate[0].is_negative()
            || rate[0].compare(&rate[1], budget)? == Ordering::Greater
        {
            return Err(invalid());
        }
        let width = rate[1].sub(&rate[0], budget)?;
        let bytes = rate
            .iter()
            .chain(std::iter::once(&width))
            .fold(256_u64, |sum, r| sum.saturating_add(rational_bytes(r)));
        retained(bytes, budget)?;
        Ok(Self { rate, width, bytes })
    }

    /// Exact positive support, retained independently of materialization.
    #[must_use]
    pub const fn rate(&self) -> &[Rational; 2] {
        &self.rate
    }

    /// Nonnegative spread; zero proves the infinite separated-family exception.
    #[must_use]
    pub const fn width(&self) -> &Rational {
        &self.width
    }

    /// Complete index domain has no last integer.
    #[must_use]
    pub const fn last_index(&self) -> Option<&Rational> {
        None
    }

    /// Conservative exact ownership, excluding any numerical rendering.
    #[must_use]
    pub const fn retained_payload_bytes(&self) -> u64 {
        self.bytes
    }

    /// Materialize any member of the complete unbounded family.
    /// # Errors
    /// Refuses negative/fractional indices and exhausted caller resources.
    pub fn member(&self, index: Rational, budget: &mut Budget) -> Result<PiZeroBox, Fault> {
        integer(&index, budget)?;
        let twice = index.mul(&Rational::from_ratio(2, 1, budget)?, budget)?;
        let lower = twice.div(&self.rate[1], budget)?;
        let upper = twice.div(&self.rate[0], budget)?;
        let bytes = rational_bytes(&index)
            .saturating_add(rational_bytes(&lower))
            .saturating_add(rational_bytes(&upper))
            .saturating_add(256);
        retained(bytes.saturating_add(self.bytes), budget)?;
        Ok(PiZeroBox {
            index,
            endpoints: [
                PiAffineEndpoint::affine(Rational::zero(), lower),
                PiAffineEndpoint::affine(Rational::zero(), upper),
            ],
        })
    }

    /// Check the first overlap by `N*w >= lower` and `(N-1)*w < lower`.
    ///
    /// Positive width makes every later overlap persist. Positive finite denominators
    /// make the upper endpoints unbounded, so the connected union is `[start,infinity)`.
    /// # Errors
    /// Refuses zero spread, a false first index and exhausted caller resources.
    pub fn overlap(&self, first: Rational, budget: &mut Budget) -> Result<PiZeroTail, Fault> {
        integer(&first, budget)?;
        if first.is_zero() || self.width.is_zero() {
            return Err(invalid());
        }
        let previous = first.sub(&Rational::one(), budget)?;
        let products = [
            previous.mul(&self.width, budget)?,
            first.mul(&self.width, budget)?,
        ];
        let minimal = products[0].compare(&self.rate[0], budget)? == Ordering::Less;
        let overlaps = products[1].compare(&self.rate[0], budget)? != Ordering::Less;
        if !minimal || !overlaps {
            return Err(invalid());
        }
        let member = self.member(first.clone(), budget)?;
        let start = member.endpoints[0].clone();
        let bytes = self
            .bytes
            .saturating_add(rational_bytes(&first))
            .saturating_add(rational_bytes(&previous))
            .saturating_add(rational_bytes(&products[0]))
            .saturating_add(rational_bytes(&products[1]))
            .saturating_add(rational_bytes(start.coefficients().1))
            .saturating_add(512);
        retained(bytes, budget)?;
        Ok(PiZeroTail {
            family: self.clone(),
            first,
            previous,
            products,
            start,
            bytes,
        })
    }
}

/// Checked connected suffix; the epoch and indices `1..N-1` remain separate.
#[derive(Clone, Debug)]
pub struct PiZeroTail {
    /// The complete original mathematical family remains available after aggregation.
    family: PiZeroFamily,
    /// Minimal index in the overlapping suffix.
    first: Rational,
    /// Last separate positive index, or zero when there are no positive separate boxes.
    previous: Rational,
    /// Exact products `(N-1)*w` and `N*w`, on either side of the positive lower rate.
    products: [Rational; 2],
    /// Included exact start; there is no finite upper endpoint.
    start: PiAffineEndpoint,
    /// Complete conservative exact ownership.
    bytes: u64,
}

impl PiZeroTail {
    /// Complete original nonnegative integer family.
    #[must_use]
    pub const fn family(&self) -> &PiZeroFamily {
        &self.family
    }

    /// First overlapping index, proved minimal rather than used as a cutoff.
    #[must_use]
    pub const fn first_index(&self) -> &Rational {
        &self.first
    }

    /// Last separate index; zero names the epoch if no positive box is separate.
    #[must_use]
    pub const fn last_separate_index(&self) -> &Rational {
        &self.previous
    }

    /// The exact checked minimality and overlap products.
    #[must_use]
    pub const fn overlap_products(&self) -> &[Rational; 2] {
        &self.products
    }

    /// Included tail-start expression.
    #[must_use]
    pub const fn start(&self) -> &PiAffineEndpoint {
        &self.start
    }

    /// Conservative complete mathematical ownership.
    #[must_use]
    pub const fn retained_payload_bytes(&self) -> u64 {
        self.bytes
    }
}

/// Integer validation charges component inspection and never converts to a machine index.
fn integer(value: &Rational, budget: &mut Budget) -> Result<(), Fault> {
    let (top, bottom) = value.component_bits();
    budget.work(
        top.div_ceil(64).saturating_add(bottom.div_ceil(64)).max(1),
        AT,
    )?;
    value.check(budget, AT)?;
    if value.is_negative() || !value.is_integer() {
        return Err(invalid());
    }
    Ok(())
}

/// This closed certificate adds no wire schema or theorem code.
const fn invalid() -> Fault {
    Fault::invalid(InvalidCode::EndpointMismatch, AT)
}
