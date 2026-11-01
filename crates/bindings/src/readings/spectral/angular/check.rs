// SPDX-License-Identifier: AGPL-3.0-only
//! Exact scalar inventories retain every ordered covariance and parameter-error target.

use super::{AT, Error, LcAngularFrequencySymbols, LcSpectralLaw, ReadingError};
use crate::readings::{CompanionReadingKind, Representation, RepresentationKind};
use crate::{CompanionKind, Descriptor, Kind, Measure, Role};
use nrz_cas::{
    Cas, Rational,
    limits::{Budget, BudgetKind},
    readings::AffineMap,
    syntax::{AtomicSort, Field, Sort},
};

/// Fresh identity readings and their complete target-bound parameters form one sealed output.
pub(super) fn symbols(
    cas: &Cas,
    law: &LcSpectralLaw,
    symbols: &LcAngularFrequencySymbols,
    budget: &mut Budget,
) -> Result<(), Error> {
    let expected = Descriptor::new(
        Kind::angular_frequency().map_err(ReadingError::from)?,
        Measure::coherent(),
        Role::Difference,
    )
    .map_err(ReadingError::from)?;
    channel(cas, law, symbols, &expected, budget)
}

/// Shared scalar channel checks preserve the angular path's exact admission schedule.
pub(in crate::readings::spectral) fn channel(
    cas: &Cas,
    law: &LcSpectralLaw,
    symbols: &LcAngularFrequencySymbols,
    expected: &Descriptor,
    budget: &mut Budget,
) -> Result<(), Error> {
    let value = symbols.value.reading();
    let error = symbols.covariance_error_value.reading();
    identity(&symbols.value, expected, budget)?;
    super::super::output_context::check(cas, law, value.family().context(), budget)?;
    if value.family().context() != error.family().context()
        || value.family().root() == error.family().root()
    {
        return Err(Error::Structure);
    }
    companions(symbols, budget)?;
    let parameter_error = &symbols.covariance_error;
    let covariance = &symbols.companions[2];
    if parameter_error.target().node() != covariance.node()
        || parameter_error.source_target() != covariance.parameters().node()
        || error.family().root() != parameter_error.source()
    {
        return Err(Error::Structure);
    }
    let [pairing] = parameter_error.units() else {
        return Err(Error::Structure);
    };
    let descriptor = pairing
        .uncertainty_descriptor()
        .ok_or(ReadingError::Meaning)?;
    identity(&symbols.covariance_error_value, descriptor, budget)?;
    unit_map(parameter_error.map(), budget)?;
    distinct(symbols, budget)
}

/// A defined scalar view reads exactly one original coherent coordinate with coefficient one.
pub(in crate::readings::spectral) fn identity(
    output: &Representation,
    descriptor: &Descriptor,
    budget: &mut Budget,
) -> Result<(), Error> {
    let reading = output.reading();
    if output.kind() != RepresentationKind::Defined
        || !output.witness().is_unconditional()
        || reading.family().node() != reading.family().root()
        || reading.family().original_entries().len() != 1
        || reading.entries().len() != 1
        || reading.selection().indices().len() != 1
        || reading.sort() != &Sort::Value(AtomicSort::Scalar(Field::Real))
    {
        return Err(Error::Structure);
    }
    unit_map(reading.map(), budget)?;
    for entry in reading
        .entries()
        .iter()
        .chain(reading.family().original_entries())
    {
        budget.work(1, AT)?;
        if entry.descriptor() != descriptor
            || entry.magnitude().as_rational() != Some(&Rational::one())
        {
            return Err(ReadingError::Meaning.into());
        }
    }
    Ok(())
}

/// Original ordered targets and symbols are checked independently of equal parameter numbers.
fn companions(symbols: &LcAngularFrequencySymbols, budget: &mut Budget) -> Result<(), Error> {
    let value = symbols.value.reading();
    let kinds = [
        CompanionReadingKind::Radius(CompanionKind::DeterministicBound),
        CompanionReadingKind::Radius(CompanionKind::NumericalError),
        CompanionReadingKind::Covariance,
    ];
    for (index, (companion, kind)) in symbols.companions.iter().zip(kinds).enumerate() {
        budget.work(1, AT)?;
        let count = if index == 2 { 2 } else { 1 };
        if companion.kind() != kind
            || companion.targets().len() != count
            || companion.targets().iter().any(|target| target != value)
        {
            return Err(Error::Structure);
        }
        unit_map(companion.map(), budget)?;
    }
    Ok(())
}

/// Every parameter expression in this closed one-coordinate route has an exact identity map.
pub(in crate::readings::spectral) fn unit_map(
    map: &AffineMap,
    budget: &mut Budget,
) -> Result<(), Error> {
    budget.work(1, AT)?;
    if map.inputs() != 1
        || map.outputs() != 1
        || map.coefficients() != [Rational::one()]
        || map.offsets() != [Rational::zero()]
    {
        return Err(Error::Structure);
    }
    Ok(())
}

/// No value, source parameter or output parameter may silently alias another declaration.
fn distinct(symbols: &LcAngularFrequencySymbols, budget: &mut Budget) -> Result<(), Error> {
    let value = symbols.value.reading();
    let error = symbols.covariance_error_value.reading();
    let names = [
        value.family().root(),
        value.symbol(),
        error.family().root(),
        error.symbol(),
        symbols.covariance_error.node(),
        symbols.companions[0].parameters().node(),
        symbols.companions[0].symbol(),
        symbols.companions[1].parameters().node(),
        symbols.companions[1].symbol(),
        symbols.companions[2].parameters().node(),
        symbols.companions[2].symbol(),
    ];
    for (index, name) in names.iter().enumerate() {
        budget.work(u64::try_from(index).map_err(|_| ReadingError::Shape)?, AT)?;
        if names[..index].contains(name) {
            return Err(Error::Structure);
        }
    }
    Ok(())
}

/// Existing witness envelopes bound owned maps and meanings; original theorem owners stay shared.
pub(super) fn storage(
    symbols: &LcAngularFrequencySymbols,
    budget: &mut Budget,
) -> Result<u64, Error> {
    let mut bytes = u64::try_from(size_of::<super::Definition>() + 2 * size_of::<usize>())
        .map_err(|_| ReadingError::Shape)?;
    for output in [&symbols.value, &symbols.covariance_error_value] {
        budget.work(1, AT)?;
        let witness =
            u64::try_from(output.witness().exported_byte_len()).map_err(|_| ReadingError::Shape)?;
        bytes = bytes
            .checked_add(witness.checked_mul(8).ok_or(ReadingError::Shape)?)
            .ok_or(ReadingError::Shape)?;
    }
    // Closed companion maps have one exact unit coefficient and zero offset; their handles
    // share CAS storage and the original coordinate meanings rather than copying graph nodes.
    let parameters = 3 * size_of::<crate::readings::CompanionReading>()
        + size_of::<crate::readings::ParameterErrorReading>();
    bytes = bytes
        .checked_add(u64::try_from(parameters * 8).map_err(|_| ReadingError::Shape)?)
        .ok_or(ReadingError::Shape)?;
    budget.check(BudgetKind::StagedPayloadBytes, bytes, AT)?;
    Ok(bytes)
}
