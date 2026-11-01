// SPDX-License-Identifier: AGPL-3.0-only
//! Every cyclic and cross-covariance parameter retains its original ordered target and unit map.

use super::super::angular::check::{channel, identity, unit_map};
use super::{AT, Error, LcAngularFrequencyDefinition, LcFrequencySymbols, ReadingError};
use crate::{
    Descriptor,
    readings::{CompanionReadingKind, LcAngularFrequencySymbols},
};
use nrz_cas::{
    Cas, Handle,
    limits::{Budget, BudgetKind},
};

/// Reuse the angular channel validator, then check the entire connected symbol closure.
pub(super) fn symbols(
    cas: &Cas,
    angular: &LcAngularFrequencyDefinition,
    symbols: &LcFrequencySymbols,
    descriptor: &Descriptor,
    budget: &mut Budget,
) -> Result<(), Error> {
    let cyclic = LcAngularFrequencySymbols {
        value: symbols.cyclic.clone(),
        companions: symbols.companions.clone(),
        covariance_error: symbols.covariance_error.clone(),
        covariance_error_value: symbols.covariance_error_value.clone(),
    };
    channel(cas, angular.law(), &cyclic, descriptor, budget)?;
    let context = angular.symbols().value.reading().family().context();
    if symbols.cyclic.reading().family().context() != context
        || symbols.cross_error_value.reading().family().context() != context
    {
        return Err(Error::Structure);
    }
    cross(angular, symbols, budget)?;
    let mut names = channel_names(angular.symbols()).to_vec();
    names.extend(channel_names(&cyclic));
    names.extend([
        symbols.cross_covariance.parameters().node(),
        symbols.cross_covariance.symbol(),
        symbols.cross_error_value.reading().family().root(),
        symbols.cross_error_value.reading().symbol(),
        symbols.cross_error.node(),
    ]);
    for (index, name) in names.iter().enumerate() {
        budget.work(u64::try_from(index).map_err(|_| ReadingError::Shape)?, AT)?;
        if names[..index].contains(name) {
            return Err(Error::Structure);
        }
    }
    Ok(())
}

/// The cross block is explicitly ordered; transpose or self-covariance substitution refuses.
fn cross(
    angular: &LcAngularFrequencyDefinition,
    symbols: &LcFrequencySymbols,
    budget: &mut Budget,
) -> Result<(), Error> {
    budget.work(1, AT)?;
    let cross = &symbols.cross_covariance;
    let error = &symbols.cross_error;
    let expected = [
        angular.symbols().value.reading().clone(),
        symbols.cyclic.reading().clone(),
    ];
    if cross.kind() != CompanionReadingKind::Covariance
        || cross.targets() != expected
        || error.target().node() != cross.node()
        || error.source_target() != cross.parameters().node()
        || error.source() != symbols.cross_error_value.reading().family().root()
    {
        return Err(Error::Structure);
    }
    unit_map(cross.map(), budget)?;
    unit_map(error.map(), budget)?;
    let [pairing] = error.units() else {
        return Err(Error::Structure);
    };
    identity(
        &symbols.cross_error_value,
        pairing
            .uncertainty_descriptor()
            .ok_or(ReadingError::Meaning)?,
        budget,
    )
}

/// Every scalar channel owns distinct base, view and parameter declarations.
const fn channel_names(symbols: &LcAngularFrequencySymbols) -> [&Handle; 11] {
    [
        symbols.value.reading().family().root(),
        symbols.value.reading().symbol(),
        symbols.covariance_error_value.reading().family().root(),
        symbols.covariance_error_value.reading().symbol(),
        symbols.covariance_error.node(),
        symbols.companions[0].parameters().node(),
        symbols.companions[0].symbol(),
        symbols.companions[1].parameters().node(),
        symbols.companions[1].symbol(),
        symbols.companions[2].parameters().node(),
        symbols.companions[2].symbol(),
    ]
}

/// Original owners are shared; new witnesses, parameter handles and metadata receive an allowance.
pub(super) fn storage(symbols: &LcFrequencySymbols, budget: &mut Budget) -> Result<u64, Error> {
    let mut bytes = u64::try_from(size_of::<super::Definition>() + 2 * size_of::<usize>())
        .map_err(|_| ReadingError::Shape)?;
    for value in [
        &symbols.cyclic,
        &symbols.covariance_error_value,
        &symbols.cross_error_value,
    ] {
        budget.work(1, AT)?;
        let witness =
            u64::try_from(value.witness().exported_byte_len()).map_err(|_| ReadingError::Shape)?;
        bytes = bytes
            .checked_add(witness.checked_mul(8).ok_or(ReadingError::Shape)?)
            .ok_or(ReadingError::Shape)?;
    }
    let parameters = 4 * size_of::<crate::readings::CompanionReading>()
        + 2 * size_of::<crate::readings::ParameterErrorReading>();
    bytes = bytes
        .checked_add(u64::try_from(parameters * 8).map_err(|_| ReadingError::Shape)?)
        .ok_or(ReadingError::Shape)?;
    budget.check(BudgetKind::StagedPayloadBytes, bytes, AT)?;
    Ok(bytes)
}
