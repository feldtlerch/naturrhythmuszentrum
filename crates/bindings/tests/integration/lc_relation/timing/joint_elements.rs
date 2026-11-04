// SPDX-License-Identifier: AGPL-3.0-only
//! Simultaneous bounded element supports keep their exact conditional fibre beside original timing.

use super::super::support::{TestResult, ledger};
use super::support::{cycles, input, number, pi};
use nrz_bindings::{
    Budget, LcCertainty, LcJointElementsParameters, LcPhaseGauge, LcQuantity as Q, LcRelation,
    LcRelationError, LcSampleTimes, Limits, LimitsOverride,
};

/// Repeated bounded element restrictions coexist with exact original sample coordinates and phase intervals.
fn question(known: bool) -> TestResult<LcSampleTimes> {
    let mut inputs = vec![input(Q::Rate, ["1", "3"])?];
    if known {
        inputs.push(input(Q::InitialPhase, ["0", "0"])?);
    }
    for quantity in [Q::Inductance, Q::Capacitance] {
        inputs.push(input(quantity, ["1/8", "1"])?);
        inputs.push(input(quantity, ["1/4", "1/2"])?);
    }
    for (index, (time, phase)) in [("1", ["7/4", "9/4"]), ("2", ["15/4", "17/4"])]
        .into_iter()
        .enumerate()
    {
        inputs.push(input(Q::Time(index), [time, time])?);
        inputs.push(input(Q::Phase(index), phase)?);
    }
    let relation = LcRelation::new(
        "joint bounded element epoch".into(),
        2,
        inputs,
        &mut ledger()?,
    )?;
    Ok(LcSampleTimes::new(
        relation,
        vec![number("1")?, number("2")?],
        &mut ledger()?,
    )?)
}

/// Original parameters name the conditional capacitance; inductance follows the exact product equation.
fn parameters(
    rate: &str,
    alpha: &str,
    capacitance: &str,
    voltage: &str,
) -> TestResult<LcJointElementsParameters> {
    Ok(LcJointElementsParameters {
        rate: number(rate)?,
        initial_phase: number(alpha)?,
        gauge: number("0")?,
        capacitance: number(capacitance)?,
        voltage: number(voltage)?,
    })
}

/// Independent product bounds clip timing and leave a nonempty simultaneous conditional C fibre at every rate.
/// ´test:integration:lc-relation-joint-elements-domain´
#[test]
fn lc_relation_joint_elements_domain() -> TestResult {
    for known in [false, true] {
        let source = question(known)?;
        let cover = source.joint_elements_cover(cycles(), &pi()?, &mut ledger()?)?;
        assert!(cover.timing().shares_allocation(&source));
        assert!(cover.relation().shares_allocation(source.relation()));
        assert_eq!(cover.certainty(), LcCertainty::Certified);
        assert_eq!(
            cover.rate(),
            &[number("2")?, number(if known { "17/8" } else { "5/2" })?]
        );
        assert_eq!(
            cover.phase_gauge(),
            if known {
                LcPhaseGauge::Fixed
            } else {
                LcPhaseGauge::AllIntegers
            }
        );
        assert_eq!(
            cover.original_elements(),
            &[
                [number("1/4")?, number("1/2")?],
                [number("1/4")?, number("1/2")?]
            ]
        );
        assert_eq!(
            cover.capacitance_at_rate(&number("2")?, &mut ledger()?)?,
            [number("1/2")?, number("1/2")?]
        );
        for rate in cover.rate() {
            for alpha in cover.phase_at_rate(rate, &mut ledger()?)? {
                for capacitance in cover.capacitance_at_rate(rate, &mut ledger()?)? {
                    let point = LcJointElementsParameters {
                        rate: rate.clone(),
                        initial_phase: alpha.clone(),
                        gauge: number("0")?,
                        capacitance,
                        voltage: number("1")?,
                    };
                    let witness = cover.witness(&point, &mut ledger()?)?;
                    source
                        .relation()
                        .check_witness(witness.values().clone(), &mut ledger()?)?;
                }
            }
        }
    }
    Ok(())
}

/// Literal physical solutions check all restored elements, amplitudes and conserved energy independently.
/// ´test:integration:lc-relation-joint-elements-physics´
#[test]
fn lc_relation_joint_elements_physics() -> TestResult {
    let source = question(false)?;
    let cover = source.joint_elements_cover(cycles(), &pi()?, &mut ledger()?)?;
    assert_eq!(
        cover.capacitance_at_rate(&number("5/2")?, &mut ledger()?)?,
        [number("8/25")?, number("1/2")?]
    );
    for (rate, alpha, capacitance, voltage, inductance, current, energy) in [
        ("2", "0", "1/2", "1", "1/2", "1", "1/4"),
        ("5/2", "-3/4", "8/25", "1", "1/2", "4/5", "4/25"),
        ("5/2", "-3/4", "1/2", "2", "8/25", "5/2", "1"),
        ("9/4", "-3/8", "4/9", "3", "4/9", "3", "2"),
    ] {
        let witness = cover.witness(
            &parameters(rate, alpha, capacitance, voltage)?,
            &mut ledger()?,
        )?;
        for (quantity, expected) in [
            (Q::Inductance, inductance),
            (Q::Capacitance, capacitance),
            (Q::VoltageAmplitude, voltage),
            (Q::CurrentAmplitude, current),
            (Q::Energy, energy),
        ] {
            assert_eq!(
                witness.coordinate(quantity),
                Some(&number(expected)?.into())
            );
        }
        assert!(witness.relation().shares_allocation(source.relation()));
    }
    for gauge in ["-1", "1"] {
        let mut point = parameters("2", "0", "1/2", "1")?;
        point.gauge = number(gauge)?;
        let witness = cover.witness(&point, &mut ledger()?)?;
        source
            .relation()
            .check_witness(witness.values().clone(), &mut ledger()?)?;
    }
    Ok(())
}

/// Crossed element marginals, missing declarations, unsupported root endpoints and false cycles refuse.
/// ´test:integration:lc-relation-joint-elements-refusals´
#[test]
fn lc_relation_joint_elements_refusals() -> TestResult {
    let source = question(false)?;
    let cover = source.joint_elements_cover(cycles(), &pi()?, &mut ledger()?)?;
    for point in [
        parameters("5/2", "-3/4", "1/4", "1")?,
        parameters("1", "0", "1/2", "1")?,
        parameters("2", "0", "1/2", "0")?,
    ] {
        assert!(cover.witness(&point, &mut ledger()?).is_err());
    }
    assert!(
        source
            .joint_elements_cover(vec![number("0")?, number("1")?], &pi()?, &mut ledger()?)
            .is_err()
    );
    let mut missing = source.relation().inputs().to_vec();
    missing.retain(|value| value.quantity() != Q::Capacitance);
    assert!(
        changed(&source, missing.clone())?
            .joint_elements_cover(cycles(), &pi()?, &mut ledger()?)
            .is_err()
    );
    missing.push(input(Q::Capacitance, ["1/4", "3/4"])?);
    assert!(
        changed(&source, missing)?
            .joint_elements_cover(cycles(), &pi()?, &mut ledger()?)
            .is_err()
    );
    let mut extra = source.relation().inputs().to_vec();
    extra.push(input(Q::Energy, ["1/4", "1/4"])?);
    assert!(
        changed(&source, extra)?
            .joint_elements_cover(cycles(), &pi()?, &mut ledger()?)
            .is_err()
    );
    Ok(())
}

/// Refused domains are complete new original questions, not modifications of a published owner.
fn changed(
    source: &LcSampleTimes,
    inputs: Vec<nrz_bindings::LcInput>,
) -> TestResult<LcSampleTimes> {
    let relation = LcRelation::new(
        source.relation().epoch().into(),
        source.relation().observations(),
        inputs,
        &mut ledger()?,
    )?;
    Ok(LcSampleTimes::new(
        relation,
        source.nominal_times().to_vec(),
        &mut ledger()?,
    )?)
}

/// Admission, complete branch checking, conditional fibres and original reconstruction spend exact caller Work.
/// ´test:integration:lc-relation-joint-elements-work´
#[test]
fn lc_relation_joint_elements_work() -> TestResult {
    let source = question(false)?;
    let pi = pi()?;
    boundary(|budget| source.joint_elements_cover(cycles(), &pi, budget))?;
    let cover = source.joint_elements_cover(cycles(), &pi, &mut ledger()?)?;
    let rate = number("5/2")?;
    boundary(|budget| cover.capacitance_at_rate(&rate, budget))?;
    let parameters = parameters("2", "0", "1/2", "1")?;
    boundary(|budget| cover.witness(&parameters, budget))?;
    Ok(())
}

/// A consumed prefix remains charged at exact success and one-short refusal.
fn boundary<T>(operation: impl Fn(&mut Budget) -> Result<T, LcRelationError>) -> TestResult {
    let mut measured = ledger()?;
    assert!(operation(&mut measured).is_ok());
    for (limit, expected) in [
        (measured.usage().work + 17, true),
        (measured.usage().work + 16, false),
    ] {
        let mut budget = Budget::new(Limits::default().resolve(&LimitsOverride {
            work: Some(limit),
            ..LimitsOverride::default()
        })?)?;
        budget.work(17, nrz_cas::Location::new("joint element prefix"))?;
        let outcome = operation(&mut budget);
        assert_eq!(outcome.is_ok(), expected);
        if !expected {
            assert!(matches!(
                outcome,
                Err(LcRelationError::Algebra(nrz_cas::Fault {
                    kind: nrz_cas::FaultKind::Exhausted {
                        budget: nrz_cas::limits::BudgetKind::Work,
                        ..
                    },
                    ..
                }))
            ));
        }
    }
    Ok(())
}
