// SPDX-License-Identifier: AGPL-3.0-only
//! Original independent physical boxes and a separate exact Machin certificate.

use crate::kernel_oracle::Rational as Oracle;
use nrz_bindings::readings::algebra::{Rational, pi_bounds::PiBoundCertificate};
use nrz_bindings::{
    Budget, LcForwardCover, LcInput, LcInterval, LcQuantity as Q, LcRelation, Limits,
};
use nrz_scalar::{Enclosure, Scalar};

/// Every test propagates typed production failures.
pub(super) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

/// Existing exact defaults, without raised pins or ceilings.
pub(super) fn ledger() -> TestResult<Budget> {
    Ok(Budget::new(Limits::default())?)
}

/// Independent exact rational spelling.
pub(super) fn number(text: &str) -> TestResult<Rational> {
    Ok(Rational::parse(text, &mut ledger()?)?)
}

/// Mathematical integers remain separate from floating numerical storage.
pub(super) fn integer(value: i64) -> TestResult<Rational> {
    Ok(Rational::from_ratio(value, 1, &mut ledger()?)?)
}

/// Original deterministic support in the required coherent quantity kind.
pub(super) fn input(quantity: Q, bounds: [&str; 2]) -> TestResult<LcInput> {
    let interval = LcInterval::new(
        Some(number(bounds[0])?.into()),
        Some(number(bounds[1])?.into()),
        &mut ledger()?,
    )?;
    Ok(LcInput::new(
        quantity,
        &quantity.descriptor()?,
        interval,
        &mut ledger()?,
    )?)
}

/// Original forward fixture declares positive energy explicitly, independently of element values.
pub(super) fn inputs() -> TestResult<Vec<LcInput>> {
    Ok(vec![
        input(Q::Inductance, ["1/4", "1"])?,
        input(Q::Capacitance, ["1/4", "1"])?,
        input(Q::InitialPhase, ["0", "0"])?,
        input(Q::Energy, ["1", "1"])?,
    ])
}

/// Query times always use this explicitly declared epoch.
pub(super) fn relation(inputs: Vec<LcInput>) -> TestResult<LcRelation> {
    Ok(LcRelation::new(
        "forward epoch".into(),
        0,
        inputs,
        &mut ledger()?,
    )?)
}

/// Weak exact bounds suffice for these branch proofs and are checked independently by CAS.
pub(super) fn pi_certificate() -> TestResult<PiBoundCertificate> {
    Ok(PiBoundCertificate {
        version: 1,
        terms: [2, 1],
        sums: [number("74/375")?, number("1/239")?],
        lower: number("31/10")?,
        upper: number("22/7")?,
    })
}

/// The proposed index range is certified from the original derived extrema.
pub(super) fn cover(time: [&str; 2], cycles: [i64; 2]) -> TestResult<LcForwardCover> {
    Ok(relation(inputs()?)?.forward_cover(
        input(Q::Time(0), time)?,
        [integer(cycles[0])?, integer(cycles[1])?],
        &pi_certificate()?,
        &mut ledger()?,
    )?)
}

/// Decode native endpoints independently before exact containment comparisons.
pub(super) fn contains<P: Scalar>(enclosure: Enclosure<P>, value: Oracle) -> TestResult {
    let lower = Oracle::binary(enclosure.lower().get().to_f64())?;
    let upper = Oracle::binary(enclosure.upper().get().to_f64())?;
    assert!(lower.at_most(value)?);
    assert!(value.at_most(upper)?);
    Ok(())
}
