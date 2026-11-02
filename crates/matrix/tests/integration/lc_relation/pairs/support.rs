// SPDX-License-Identifier: AGPL-3.0-only
//! Original measured phases and exact timestamps are independent of the recovery solver.

use super::super::support::{TestResult, input, integer, ledger, number, pi_certificate};
use nrz_bindings::readings::algebra::syntax::Constant;
use nrz_bindings::{ExactConstant, LcInput, LcInterval, LcPairsCover, LcQuantity as Q, LcRelation};

/// Exact centers retain the full turn instead of inserting a decimal phase.
pub(super) fn inputs(known: bool, radius: &str) -> TestResult<Vec<LcInput>> {
    let mut inputs = vec![input(Q::AngularFrequency, ["1000", "3000"])?];
    let turn = ExactConstant::named(Constant::Pi, &mut ledger()?)?
        .mul(&number("2")?.into(), &mut ledger()?)?;
    let radius = ExactConstant::from(number(radius)?);
    for (i, (time, center, cycle)) in [
        ("1/1000", "2", "0"),
        ("1/400", "5", "0"),
        ("2/625", "32/5", "1"),
    ]
    .into_iter()
    .enumerate()
    {
        inputs.push(input(Q::Time(i), [time, time])?);
        let center = ExactConstant::from(number(center)?).sub(
            &turn.mul(&number(cycle)?.into(), &mut ledger()?)?,
            &mut ledger()?,
        )?;
        let phase = LcInterval::new(
            Some(center.sub(&radius, &mut ledger()?)?),
            Some(center.add(&radius, &mut ledger()?)?),
            &mut ledger()?,
        )?;
        inputs.push(LcInput::new(
            Q::Phase(i),
            &Q::Phase(i).descriptor()?,
            phase,
            &mut ledger()?,
        )?);
    }
    if known {
        inputs.push(input(Q::InitialPhase, ["0", "0"])?);
    }
    Ok(inputs)
}

/// Every original timestamp has the same explicit epoch.
pub(super) fn relation(inputs: Vec<LcInput>) -> TestResult<LcRelation> {
    Ok(LcRelation::new(
        "recovery epoch".into(),
        3,
        inputs,
        &mut ledger()?,
    )?)
}

/// Complete base branch is proved against the supplied range, never selected by proximity.
pub(super) fn cover(known: bool, radius: &str) -> TestResult<LcPairsCover> {
    Ok(relation(inputs(known, radius)?)?.pairs_cover(
        vec![integer(0)?, integer(0)?, integer(1)?],
        &pi_certificate()?,
        &mut ledger()?,
    )?)
}
