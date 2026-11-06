// SPDX-License-Identifier: AGPL-3.0-only
//! One shared inductance calibration retains the simultaneous original capacitance and timing fibres.

use super::super::support::{TestResult, ledger};
use super::support::{cycles, input, number, pi};
use nrz_bindings::{
    Budget, LcCalibratedElements, LcCalibratedElementsParameters, LcElementSamples, LcInterval,
    LcJointElementsParameters, LcQuantity as Q, LcRelation, LcRelationError, LcSampleTimes, Limits,
    LimitsOverride,
};

/// Original interval scales are independent of every measurement declaration.
fn timing() -> TestResult<LcSampleTimes> {
    let mut inputs = vec![
        input(Q::Rate, ["1", "3"])?,
        input(Q::Inductance, ["1/8", "1/2"])?,
        input(Q::Capacitance, ["1/4", "1/2"])?,
    ];
    for (index, (time, phase)) in [("1", ["7/4", "9/4"]), ("2", ["15/4", "17/4"])]
        .into_iter()
        .enumerate()
    {
        inputs.push(input(Q::Time(index), [time, time])?);
        inputs.push(input(Q::Phase(index), phase)?);
    }
    let relation = LcRelation::new(
        "coupled calibrated elements epoch".into(),
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

/// The selected input's descriptor supplies an ordinary coherent element interval.
fn interval(bounds: [&str; 2]) -> TestResult<LcInterval> {
    Ok(input(Q::Inductance, bounds)?.interval().clone())
}

/// Each uncertainty layer alone and their joint inventory have the same physical L image here.
fn question(layer: usize) -> TestResult<LcCalibratedElements> {
    let samples = if layer == 2 {
        vec![interval(["1/4", "1/4"])?]
    } else {
        vec![interval(["1/8", "1/2"])?, interval(["1/4", "3/4"])?]
    };
    let gain = interval(if layer == 1 { ["1", "1"] } else { ["1", "2"] })?;
    Ok(LcCalibratedElements::new(
        timing()?,
        samples,
        &gain,
        &mut ledger()?,
    )?)
}

/// Joint physical coordinates do not choose the calibration from a detached marginal.
fn parameters(
    rate: &str,
    alpha: &str,
    capacitance: &str,
    voltage: &str,
    calibration: &str,
) -> TestResult<LcCalibratedElementsParameters> {
    Ok(LcCalibratedElementsParameters {
        physical: LcJointElementsParameters {
            rate: number(rate)?,
            initial_phase: number(alpha)?,
            gauge: number("0")?,
            capacitance: number(capacitance)?,
            voltage: number(voltage)?,
        },
        calibration: number(calibration)?,
    })
}

/// Both uncertainty layers retain bounded simultaneous L/C with a complete timing and calibration converse.
/// ´test:integration:lc-relation-coupled-element-calibration-layers´
#[test]
fn lc_relation_coupled_element_calibration_layers() -> TestResult {
    for layer in 0..3 {
        let source = question(layer)?;
        let cover = source.cover(cycles(), &pi()?, &mut ledger()?)?;
        assert!(cover.question().shares_allocation(&source));
        assert!(cover.relation().shares_allocation(source.relation()));
        assert_eq!(source.inductance(), &[number("1/4")?, number("1/2")?]);
        assert_eq!(cover.rate(), &[number("2")?, number("5/2")?]);
        assert_eq!(
            cover.capacitance_at_rate(&number("5/2")?, &mut ledger()?)?,
            [number("8/25")?, number("1/2")?]
        );
        let witness = cover.feasibility_witness();
        assert!(witness.question().shares_allocation(&source));
        source.check_witness(
            witness.original().values().clone(),
            witness.calibration().clone(),
            &mut ledger()?,
        )?;
    }
    let source = question(0)?;
    assert!(
        LcElementSamples::new(
            source.timing().clone(),
            Q::Inductance,
            source.samples().to_vec(),
            &interval(["1", "2"])?,
            &mut ledger()?
        )
        .is_err()
    );
    Ok(())
}

/// Literal original tuples independently check all physical scales, readings and nonzero common gauges.
/// ´test:integration:lc-relation-coupled-element-calibration-physics´
#[test]
fn lc_relation_coupled_element_calibration_physics() -> TestResult {
    let source = question(0)?;
    let cover = source.cover(cycles(), &pi()?, &mut ledger()?)?;
    for (rate, alpha, capacitance, voltage, gain, inductance, current, energy) in [
        ("2", "0", "1/2", "1", "2", "1/2", "1", "1/4"),
        ("5/2", "-3/4", "1/2", "2", "32/25", "8/25", "5/2", "1"),
        ("5/2", "-3/4", "8/25", "1", "1", "1/2", "4/5", "4/25"),
        ("9/4", "-3/8", "4/9", "3", "4/3", "4/9", "3", "2"),
    ] {
        let mut parameters = parameters(rate, alpha, capacitance, voltage, gain)?;
        for gauge in ["0", "-1", "1"] {
            parameters.physical.gauge = number(gauge)?;
            check_physical(
                &source,
                &cover,
                &parameters,
                [inductance, current, energy],
                gain,
            )?;
        }
    }
    Ok(())
}

/// Full original substitution and rational reading controls are independent of the projected image.
fn check_physical(
    source: &LcCalibratedElements,
    cover: &nrz_bindings::LcCalibratedElementsCover,
    parameters: &LcCalibratedElementsParameters,
    expected: [&str; 3],
    gain: &str,
) -> TestResult {
    let [inductance, current, energy] = expected;
    let witness = cover.witness(parameters, &mut ledger()?)?;
    assert!(witness.question().shares_allocation(source));
    assert!(
        witness
            .original()
            .relation()
            .shares_allocation(source.relation())
    );
    for (quantity, expected) in [
        (Q::Inductance, inductance),
        (Q::CurrentAmplitude, current),
        (Q::Energy, energy),
    ] {
        assert_eq!(
            witness
                .original()
                .coordinate(quantity)
                .and_then(nrz_bindings::ExactConstant::as_rational),
            Some(&number(expected)?)
        );
    }
    let reading = number(inductance)?.div(&number(gain)?, &mut ledger()?)?;
    for sample in source.samples() {
        let low = sample
            .lower()
            .and_then(nrz_bindings::ExactConstant::as_rational)
            .ok_or("lower")?;
        let high = sample
            .upper()
            .and_then(nrz_bindings::ExactConstant::as_rational)
            .ok_or("upper")?;
        assert_ne!(
            reading.compare(low, &mut ledger()?)?,
            std::cmp::Ordering::Less
        );
        assert_ne!(
            reading.compare(high, &mut ledger()?)?,
            std::cmp::Ordering::Greater
        );
    }
    Ok(())
}

/// Conditional gains cannot be crossed with physical L/C marginals or replaced by independent gains.
/// ´test:integration:lc-relation-coupled-element-calibration-fibres´
#[test]
fn lc_relation_coupled_element_calibration_fibres() -> TestResult {
    let source = question(0)?;
    let cover = source.cover(cycles(), &pi()?, &mut ledger()?)?;
    assert_eq!(
        source.calibration_at_inductance(&number("8/25")?, &mut ledger()?)?,
        [number("1")?, number("32/25")?]
    );
    assert_eq!(
        source.calibration_at_inductance(&number("1/2")?, &mut ledger()?)?,
        [number("1")?, number("2")?]
    );
    assert!(
        cover
            .witness(&parameters("5/2", "-3/4", "1/2", "1", "2")?, &mut ledger()?)
            .is_err()
    );
    assert!(
        cover
            .witness(&parameters("5/2", "-3/4", "1/4", "1", "1")?, &mut ledger()?)
            .is_err()
    );
    // Separate gains two and one would each fit L=1/2; the common-gain question cannot.
    assert_eq!(
        number("2")?.mul(&number("1/4")?, &mut ledger()?)?,
        number("1/2")?
    );
    assert_eq!(
        number("1")?.mul(&number("1/2")?, &mut ledger()?)?,
        number("1/2")?
    );
    assert!(
        LcCalibratedElements::new(
            timing()?,
            vec![interval(["1/4", "1/4"])?, interval(["1/2", "1/2"])?],
            &interval(["1", "2"])?,
            &mut ledger()?
        )
        .is_err()
    );
    Ok(())
}

/// Unsupported scales, incomplete branches and invalid positive domains remain refusals with source intact.
/// ´test:integration:lc-relation-coupled-element-calibration-refusals´
#[test]
fn lc_relation_coupled_element_calibration_refusals() -> TestResult {
    let source = question(0)?;
    assert!(
        source
            .cover(vec![number("0")?, number("1")?], &pi()?, &mut ledger()?)
            .is_err()
    );
    assert!(
        LcCalibratedElements::new(
            timing()?,
            Vec::new(),
            &interval(["1", "2"])?,
            &mut ledger()?
        )
        .is_err()
    );
    assert!(
        LcCalibratedElements::new(
            timing()?,
            vec![interval(["1/4", "1/2"])?],
            &interval(["0", "2"])?,
            &mut ledger()?
        )
        .is_err()
    );
    let mut inputs = source.relation().inputs().to_vec();
    inputs.push(input(Q::Energy, ["1", "2"])?);
    let relation = LcRelation::new(source.relation().epoch().into(), 2, inputs, &mut ledger()?)?;
    let changed = LcSampleTimes::new(
        relation,
        source.timing().nominal_times().to_vec(),
        &mut ledger()?,
    )?;
    assert!(
        LcCalibratedElements::new(
            changed,
            source.samples().to_vec(),
            &interval(["1", "2"])?,
            &mut ledger()?
        )
        .is_err()
    );
    assert_eq!(source.samples().len(), 2);
    Ok(())
}

/// Complete admission, recovery, conditional calibration and original reconstruction spend cumulative Work.
/// ´test:integration:lc-relation-coupled-element-calibration-work´
#[test]
fn lc_relation_coupled_element_calibration_work() -> TestResult {
    let source = question(0)?;
    let gain = interval(["1", "2"])?;
    boundary(|budget| {
        LcCalibratedElements::new(
            source.timing().clone(),
            source.samples().to_vec(),
            &gain,
            budget,
        )
    })?;
    let pi = pi()?;
    boundary(|budget| source.cover(cycles(), &pi, budget))?;
    let inductance = number("8/25")?;
    boundary(|budget| source.calibration_at_inductance(&inductance, budget))?;
    let cover = source.cover(cycles(), &pi, &mut ledger()?)?;
    let parameters = parameters("5/2", "-3/4", "1/2", "2", "32/25")?;
    boundary(|budget| cover.witness(&parameters, budget))
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
        budget.work(17, nrz_cas::Location::new("coupled element prefix"))?;
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
