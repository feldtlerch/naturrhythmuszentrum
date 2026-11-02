// SPDX-License-Identifier: AGPL-3.0-only
//! A strict exact same-time phase gap excludes every original frequency alias.

use super::super::support::{TestResult, integer, ledger, number, pi_certificate};
use super::support;
use nrz_bindings::readings::algebra::syntax::Constant;
use nrz_bindings::{
    ExactConstant, LcCertainty, LcDeterminacy, LcInput, LcInterval, LcQuantity as Q, LcRelation,
};

/// The contradictory duplicate observation is impossible for every alias, not an unsuccessful fit.
/// ´test:integration:lc-relation-impossible-pairs´
#[test]
fn lc_relation_impossible_pairs() -> TestResult {
    const LAST: usize = 2;
    const EXTRA: usize = 3;
    let mut inputs = support::inputs(false, "1/20")?;
    inputs.retain(|input| input.quantity() != Q::AngularFrequency);
    let time = LcInterval::exact(number("2/625")?.into(), &mut ledger()?)?;
    inputs.push(LcInput::new(
        Q::Time(EXTRA),
        &Q::Time(EXTRA).descriptor()?,
        time,
        &mut ledger()?,
    )?);
    let turn = ExactConstant::named(Constant::Pi, &mut ledger()?)?
        .mul(&number("2")?.into(), &mut ledger()?)?;
    let center = ExactConstant::from(number("34/5")?).sub(&turn, &mut ledger()?)?;
    let radius = ExactConstant::from(number("1/20")?);
    let phase = LcInterval::new(
        Some(center.sub(&radius, &mut ledger()?)?),
        Some(center.add(&radius, &mut ledger()?)?),
        &mut ledger()?,
    )?;
    inputs.push(LcInput::new(
        Q::Phase(EXTRA),
        &Q::Phase(EXTRA).descriptor()?,
        phase,
        &mut ledger()?,
    )?);
    let relation = LcRelation::new("impossible epoch".into(), 4, inputs, &mut ledger()?)?;
    let impossible = relation.impossible_pair(
        [LAST, EXTRA],
        [integer(1)?, integer(1)?],
        &pi_certificate()?,
        &mut ledger()?,
    )?;
    assert_eq!(impossible.determinacy(), LcDeterminacy::Impossible);
    assert_eq!(impossible.certainty(), LcCertainty::Certified);
    assert_eq!(impossible.gap(), &[number("3/10")?, number("1/2")?]);
    assert!(impossible.relation().shares_allocation(&relation));
    assert!(
        relation
            .impossible_pair(
                [LAST, EXTRA],
                [integer(0)?, integer(1)?],
                &pi_certificate()?,
                &mut ledger()?
            )
            .is_err()
    );
    Ok(())
}
