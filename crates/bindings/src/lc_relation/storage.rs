// SPDX-License-Identifier: AGPL-3.0-only
//! Original declaration and witness ownership stay within the caller's existing limits.

use super::{AT, LcRelationError};
use crate::{Budget, ExactConstant};
use nrz_cas::{Rational, limits::BudgetKind};

/// Machine capacities never become mathematical domains.
pub(super) fn size(value: usize) -> Result<u64, LcRelationError> {
    u64::try_from(value).map_err(|_| LcRelationError::Capacity)
}

/// Exact rational limb envelope, with conservative record metadata.
pub(super) fn rational(value: &Rational) -> u64 {
    let (numerator, denominator) = value.component_bits();
    numerator
        .div_ceil(8)
        .saturating_add(denominator.div_ceil(8))
        .saturating_add(128)
}

/// Exact constants retain their own CAS graphs; no heap values enter the symbolic owner.
pub(super) fn constant(value: &ExactConstant) -> u64 {
    value.as_rational().map_or_else(
        || {
            value.graph().map_or(512, |(cas, _)| {
                cas.counts()
                    .payload_bytes
                    .saturating_mul(2)
                    .saturating_add(512)
            })
        },
        rational,
    )
}

/// Check complete retained metadata before returning an immutable authority.
pub(super) fn retained(bytes: u64, budget: &Budget) -> Result<(), LcRelationError> {
    budget.check(BudgetKind::StagedPayloadBytes, bytes, AT)?;
    budget.check(BudgetKind::OutputBytes, bytes, AT)?;
    Ok(())
}
