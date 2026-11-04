// SPDX-License-Identifier: AGPL-3.0-only
//! Native timestamp images retain the whole independently checked original rest inventory.

use super::{
    LcQuiescentCover, LcQuiescentOutcome, LcQuiescentProjection, LcRelation, Q, TestResult,
};
use super::{contains, input, ledger, number, work_cover};
use nrz_scalar::{ArithmeticContext, LiteralLimits, Scalar};

/// Independent expected scalar endpoints include nonbinary values in every observation.
const TIMES: [[&str; 2]; 3] = [["0", "2/3"], ["1/3", "2"], ["-2/3", "-1/3"]];

/// Repeated declarations and all observations remain retained in the source authority.
fn cover() -> TestResult<LcQuiescentCover> {
    let mut inputs = vec![
        input(Q::Inductance, ["1/4", "1"])?,
        input(Q::Capacitance, ["1/4", "1"])?,
        input(Q::Energy, ["-1", "0"])?,
        input(Q::Energy, ["0", "1"])?,
    ];
    for (index, endpoints) in TIMES.iter().enumerate() {
        inputs.push(input(Q::Time(index), ["-3", "3"])?);
        inputs.push(input(Q::Time(index), *endpoints)?);
    }
    Ok(LcRelation::new(
        "native rest inventory".into(),
        TIMES.len(),
        inputs,
        &mut ledger()?,
    )?
    .quiescent_cover(&mut ledger()?)?)
}

/// Both formats preserve all timestamp images and explicitly undefined observed phases.
/// ´test:integration:lc-relation-quiescent-native-inventory´
#[test]
fn lc_relation_quiescent_native_inventory() -> TestResult {
    let cover = cover()?;
    render::<f32>(&cover)?;
    render::<f64>(&cover)
}

/// Hand-specified rational endpoints are compared through the independent binary oracle.
fn render<P: Scalar>(cover: &LcQuiescentCover) -> TestResult {
    let context = ArithmeticContext::new()?;
    for (index, endpoints) in TIMES.iter().enumerate() {
        let exact = cover.query(Q::Time(index), &mut ledger()?)?;
        let native = LcQuiescentProjection::<P>::new(
            exact,
            LiteralLimits::default(),
            &context,
            &mut ledger()?,
        )?;
        assert!(
            native
                .exact()
                .cover()
                .relation()
                .shares_allocation(cover.relation())
        );
        assert_eq!(
            native
                .exact()
                .cover()
                .feasible()
                .values()
                .observations
                .len(),
            TIMES.len()
        );
        let scalar = native.scalar().ok_or("defined inventory timestamp")?;
        for (enclosure, expected) in scalar.endpoints().iter().zip(endpoints) {
            contains(
                enclosure.ok_or("finite inventory endpoint")?,
                &number(expected)?,
            )?;
        }
        cover.relation().check_witness(
            native.exact().cover().feasible().values().clone(),
            &mut ledger()?,
        )?;
        let exact = cover.query(Q::Phase(index), &mut ledger()?)?;
        let native = LcQuiescentProjection::<P>::new(
            exact,
            LiteralLimits::default(),
            &context,
            &mut ledger()?,
        )?;
        assert!(native.scalar().is_none());
        assert!(matches!(
            native.exact().outcome(),
            LcQuiescentOutcome::UndefinedPhase
        ));
    }
    Ok(())
}

/// Every native inventory query has cumulative exact-limit success and one-below refusal.
/// ´test:integration:lc-relation-quiescent-native-inventory-work´
#[test]
fn lc_relation_quiescent_native_inventory_work() -> TestResult {
    let cover = cover()?;
    work_cover::<f32>(&cover)?;
    work_cover::<f64>(&cover)
}
