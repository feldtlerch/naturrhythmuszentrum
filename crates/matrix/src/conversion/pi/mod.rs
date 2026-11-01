// SPDX-License-Identifier: AGPL-3.0-only
//! Shared immutable pi regions and explicitly metered certified descriptor lowering.

mod coefficient;
pub(crate) use coefficient::decode as enclose_pi_factor;
mod evidence;
mod lower;
mod region;
mod unit;

pub use evidence::PiConstantEvidence;
pub(crate) use lower::CertifiedMap;
pub use lower::PiConversionUse;
pub use region::PiConstant;
pub use unit::CertifiedCoefficient;

use super::ConversionError;
use nrz_bindings::{
    Budget, Limits, LimitsOverride, MapError,
    readings::algebra::{Location, limits::BudgetKind},
};

/// Check logical byte policy without mutating the cumulative ledger.
pub(super) fn check(
    budget: &Budget,
    kind: BudgetKind,
    bytes: usize,
) -> Result<(), ConversionError> {
    let count = u64::try_from(bytes).map_err(|_| ConversionError::ResourceLimit)?;
    budget
        .check(kind, count, Location::new("certified pi lowering"))
        .map_err(MapError::from)?;
    Ok(())
}

/// Prepay a schedule or reconcile a bounded child receipt on either outcome.
pub(super) fn charge(budget: &mut Budget, work: u64) -> Result<(), ConversionError> {
    budget
        .work(work, Location::new("certified pi lowering"))
        .map_err(MapError::from)?;
    Ok(())
}

/// Bound exact export by this caller's remaining Work and retain its receipt on refusal.
pub(crate) fn exact_attempt<T>(
    exact: &LimitsOverride,
    budget: &mut Budget,
    export: impl FnOnce(&LimitsOverride, &mut u64) -> Result<T, MapError>,
) -> Result<T, ConversionError> {
    let remaining = budget.limits().work.saturating_sub(budget.usage().work);
    let mut policy = exact.clone();
    policy.work = Some(
        policy
            .work
            .unwrap_or_else(|| Limits::default().work)
            .min(remaining),
    );
    let mut work = 0;
    let result = export(&policy, &mut work);
    charge(budget, work)?;
    Ok(result?)
}
