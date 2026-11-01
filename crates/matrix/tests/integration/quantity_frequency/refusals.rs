// SPDX-License-Identifier: AGPL-3.0-only
//! Exact kind, role and unsupported-factor refusals preserve original values and companions.

use super::{TestResult, convert, descriptor, fixture};
use nrz_bindings::{Budget, Descriptor, ExactConstant, Kind, Limits, Measure, Role, Scale};
use nrz_matrix::{ConversionError, OperationError, QuantityError};
use nrz_scalar::Scalar;

/// Generic conversion still refuses the crossing, and the explicit route rejects rate and point substitutions.
/// ´test:integration:quantity-frequency-kind-role-refusals´
#[test]
fn quantity_frequency_kind_role_refusals() -> TestResult {
    kinds::<f32>()?;
    kinds::<f64>()
}

fn kinds<P: Scalar + 'static>() -> TestResult {
    let mut case = fixture::<P>(true)?;
    let before = case.heap.provenance_usage();
    let stamp = case.input.view(&case.heap)?.stamp();
    let result = case.context.convert(
        &mut case.heap,
        &case.input,
        &case.target,
        &mut Budget::new(Limits::default())?,
    );
    assert!(
        matches!(result, Err(QuantityError::Operation(error)) if matches!(*error, OperationError::Conversion(ConversionError::Map(nrz_bindings::MapError::Units(nrz_bindings::UnitError::KindMismatch)))))
    );
    for target in [
        descriptor(true)?,
        Descriptor::new(Kind::rate()?, Measure::coherent(), Role::Difference)?,
        Descriptor::new(Kind::frequency()?, Measure::coherent(), Role::Point)?,
    ] {
        case.target = target;
        assert!(convert(&mut case, &mut Budget::new(Limits::default())?).is_err());
        assert_eq!(case.heap.provenance_usage(), before);
        assert_eq!(case.input.view(&case.heap)?.stamp(), stamp);
    }
    case.target = descriptor(false)?;
    convert(&mut case, &mut Budget::new(Limits::default())?)?;
    Ok(())
}

/// An exact but unsupported frequency coefficient refuses atomically, with a supported success twin.
/// ´test:integration:quantity-frequency-unsupported-factor´
#[test]
fn quantity_frequency_unsupported_factor() -> TestResult {
    unsupported::<f32>()?;
    unsupported::<f64>()
}

fn unsupported<P: Scalar + 'static>() -> TestResult {
    let mut case = fixture::<P>(true)?;
    let mut exact = Budget::new(Limits::default())?;
    let scale = ExactConstant::named(
        nrz_bindings::readings::algebra::syntax::Constant::E,
        &mut exact,
    )?;
    case.target = Descriptor::new(
        Kind::frequency()?,
        Measure::new(
            "e hertz",
            Scale::Expression(scale),
            Measure::coherent().offset().clone(),
        )?,
        Role::Difference,
    )?;
    let before = case.heap.provenance_usage();
    let stamp = case.input.view(&case.heap)?.stamp();
    let mut budget = Budget::new(Limits::default())?;
    let result = convert(&mut case, &mut budget);
    assert!(
        matches!(result, Err(QuantityError::Operation(error)) if matches!(*error, OperationError::Conversion(ConversionError::Map(nrz_bindings::MapError::Units(nrz_bindings::UnitError::UnsupportedExactConstantExport)))))
    );
    assert!(budget.usage().work > 1);
    assert_eq!(case.heap.provenance_usage(), before);
    assert_eq!(case.input.view(&case.heap)?.stamp(), stamp);
    case.target = descriptor(false)?;
    convert(&mut case, &mut Budget::new(Limits::default())?)?;
    Ok(())
}
