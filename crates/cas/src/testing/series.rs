// SPDX-License-Identifier: AGPL-3.0-only
//! Admitted dependencies and independently chosen formal coefficients.

use super::cases::Fixture;
use super::common::{TestResult, default_budget};
use crate::test_api::series::SeriesObject;
use crate::test_api::solving::{CoefficientDomainData, SolveDraft};
use crate::test_api::syntax::Draft;
use crate::test_api::{Handle, Reference};

/// Build exact dependencies without admitting the object under measurement.
pub(crate) fn fixture(values: &[&str]) -> TestResult<(Fixture, SeriesObject)> {
    let mut f = Fixture::new()?;
    let field = f
        .cas
        .admit(
            &[Draft::Solve(SolveDraft::CoefficientDomain(
                CoefficientDomainData::Rational,
            ))],
            &mut default_budget()?,
        )?
        .pop()
        .ok_or("field")?;
    let mut coefficients = Vec::new();
    for value in values {
        coefficients.push(Reference::Existing(f.number(value)?));
    }
    let object = SeriesObject {
        field: Reference::Existing(field),
        centre: Reference::Existing(f.zero.clone()),
        variable: Reference::Existing(f.x.clone()),
        scope: Vec::new(),
        context: Reference::Existing(f.context.clone()),
        order: u32::try_from(values.len())?,
        coefficients,
    };
    Ok((f, object))
}

/// Admission supplies no arithmetic evidence.
pub(crate) fn admit(f: &mut Fixture, value: SeriesObject) -> TestResult<Handle> {
    Ok(f.cas
        .admit(&[Draft::SeriesObject(value)], &mut default_budget()?)?
        .pop()
        .ok_or("series object")?)
}
