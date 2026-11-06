// SPDX-License-Identifier: AGPL-3.0-only
//! Calibration-only emptiness retains an otherwise feasible physical question and every source identity.

use super::support::interval;
use super::{TestResult, amplitude, boundary, cycles, ledger, number, phase, pi};
use nrz_bindings::{
    LcCertainty, LcClockRefinementState, LcDeterminacy, LcMeasurementImpossible,
    LcObservationValue, LcQuantity as Q, LcRelationError, LcSharedClockPhase, LcSharedMeasurements,
    LcValues,
};
use nrz_cas::Rational;

/// Original first phase observation.
const FIRST: usize = 0;
/// Original second phase observation.
const SECOND: usize = 1;

/// Each phase sample is individually possible, but their common offset requirements are disjoint.
fn refined(old: &LcSharedClockPhase) -> TestResult<LcSharedClockPhase> {
    Ok(LcSharedClockPhase::new(
        old.clock().clone(),
        vec![interval(["3/2", "3/2"])?, interval(["9/2", "9/2"])?],
        &interval(["-1/4", "1/4"])?,
        &mut ledger()?,
    )?)
}

/// Independent exact physical reconstruction exposes the false independent-offset relaxation.
fn physical_point() -> TestResult<LcValues> {
    let observations = [("1", "7/4"), ("2", "17/4")]
        .into_iter()
        .map(|(time, phase)| {
            Ok(LcObservationValue {
                time: number(time)?.into(),
                phase: Some(number(phase)?.into()),
                cycle: Some(Rational::zero()),
            })
        })
        .collect::<TestResult<Vec<_>>>()?;
    Ok(LcValues {
        inductance: number("2/5")?.into(),
        capacitance: number("2/5")?.into(),
        rate: number("5/2")?.into(),
        initial_phase: Some(number("-3/4")?.into()),
        voltage: number("1")?.into(),
        current: number("1")?.into(),
        energy: number("1/5")?.into(),
        observations,
    })
}

/// Separate sample offsets would admit this tuple, but the declared common offset excludes it.
/// ´test:integration:lc-relation-measurement-empty-independent-control´
#[test]
fn lc_relation_measurement_empty_independent_control() -> TestResult {
    let old = phase(false, None)?;
    let new = refined(&old)?;
    let values = physical_point()?;
    new.clock()
        .check_witness(values.clone(), number("0")?, &mut ledger()?)?;
    for (index, offset) in ["1/4", "-1/4"].into_iter().enumerate() {
        let physical = values.observations[index]
            .phase
            .as_ref()
            .ok_or("physical phase")?;
        let measured = physical.sub(&number(offset)?.into(), &mut ledger()?)?;
        let measured = measured.as_rational().ok_or("rational measured phase")?;
        let sample = &new.samples()[index];
        let lower = sample
            .lower()
            .and_then(nrz_bindings::ExactConstant::as_rational)
            .ok_or("rational sample lower")?;
        let upper = sample
            .upper()
            .and_then(nrz_bindings::ExactConstant::as_rational)
            .ok_or("rational sample upper")?;
        assert_ne!(
            measured.compare(lower, &mut ledger()?)?,
            std::cmp::Ordering::Less
        );
        assert_ne!(
            measured.compare(upper, &mut ledger()?)?,
            std::cmp::Ordering::Greater
        );
        assert!(
            new.check_witness(
                values.clone(),
                number("0")?,
                number(offset)?,
                &mut ledger()?
            )
            .is_err()
        );
    }
    assert!(new.impossible_measurements(&mut ledger()?).is_ok());
    Ok(())
}

/// The same physical relation has a full witness while its shared calibration declarations are impossible.
/// ´test:integration:lc-relation-measurement-only-emptiness´
#[test]
fn lc_relation_measurement_only_emptiness() -> TestResult {
    let old = phase(false, None)?;
    let new = refined(&old)?;
    let previous = old.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
    let physical = previous.feasibility_witness().original().original();
    new.relation()
        .check_witness(physical.values().clone(), &mut ledger()?)?;
    let certificate = certificate(&new)?;
    let inclusion = old.refinement_inputs(&new, &mut ledger()?)?;
    let horizon = previous.forward_reported_horizon(&mut ledger()?)?;
    let attempt = previous.refine_measurement_impossible(
        &inclusion,
        &horizon,
        &certificate,
        &mut ledger()?,
    )?;
    let LcClockRefinementState::MeasurementImpossible(retained) = attempt.state() else {
        return Err("separate joint measurement contradiction".into());
    };
    assert!(
        retained
            .clock_phase()
            .ok_or("retained original")?
            .shares_allocation(&new)
    );
    assert!(
        attempt
            .previous_clock_phase()
            .ok_or("previous cover")?
            .shares_allocation(&previous)
    );
    assert!(
        attempt
            .inclusion()
            .refined_clock_phase()
            .ok_or("new question")?
            .shares_allocation(&new)
    );
    assert!(attempt.refined_horizon().is_none());
    assert!(attempt.solving_usage().is_none());
    assert_eq!(attempt.proposed_cycles(), [] as [Rational; 0]);
    assert_eq!(
        attempt.previous_horizon().extremal_parameters(),
        horizon.extremal_parameters()
    );
    Ok(())
}

/// Literal separated bounds are traced back to their original physical declarations.
fn certificate(new: &LcSharedClockPhase) -> TestResult<LcMeasurementImpossible> {
    let mut measured = ledger()?;
    let certificate = new.impossible_measurements(&mut measured)?;
    assert!(certificate.relation().shares_allocation(new.relation()));
    assert!(
        certificate
            .clock_phase()
            .ok_or("original phase question")?
            .shares_allocation(new)
    );
    assert!(certificate.shared_measurements().is_none());
    assert_eq!(certificate.determinacy(), LcDeterminacy::Impossible);
    assert_eq!(certificate.certainty(), LcCertainty::Certified);
    assert_eq!(certificate.separation(), &[number("1/4")?, number("-1/4")?]);
    for (position, expected) in certificate
        .constraints()
        .into_iter()
        .zip([Q::Phase(FIRST), Q::Phase(SECOND)])
    {
        assert_eq!(
            new.relation().inputs()[position.ok_or("original bound input")?].quantity(),
            expected
        );
    }
    assert_eq!(
        certificate.usage()[1].work - certificate.usage()[0].work,
        measured.usage().work
    );
    assert!(certificate.retained_payload_bytes() > 0);
    Ok(certificate)
}

/// Every original amplitude declaration survives the independently sufficient phase-offset contradiction.
/// ´test:integration:lc-relation-measurement-empty-amplitude´
#[test]
fn lc_relation_measurement_empty_amplitude() -> TestResult {
    let old = amplitude(false, Q::VoltageAmplitude)?;
    let new = LcSharedMeasurements::new(
        refined(old.phase())?,
        old.channel(),
        old.samples().to_vec(),
        &interval(["1/2", "3/2"])?,
        &mut ledger()?,
    )?;
    let inclusion = old.refinement_inputs(&new, &mut ledger()?)?;
    let previous = old.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
    new.relation().check_witness(
        previous
            .feasibility_witness()
            .original()
            .original()
            .original()
            .values()
            .clone(),
        &mut ledger()?,
    )?;
    let certificate = new.impossible_measurements(&mut ledger()?)?;
    assert!(certificate.clock_phase().is_none());
    assert!(
        certificate
            .shared_measurements()
            .ok_or("all original measurements")?
            .shares_allocation(&new)
    );
    let horizon = previous.forward_reported_horizon(&mut ledger()?)?;
    let attempt = previous.refine_measurement_impossible(
        &inclusion,
        &horizon,
        &certificate,
        &mut ledger()?,
    )?;
    assert!(matches!(
        attempt.state(),
        LcClockRefinementState::MeasurementImpossible(_)
    ));
    assert!(
        attempt
            .previous_shared_measurement()
            .ok_or("previous amplitude")?
            .shares_allocation(&previous)
    );
    assert!(
        attempt
            .inclusion()
            .refined_shared_measurement()
            .ok_or("refined amplitude")?
            .shares_allocation(&new)
    );
    let partial = new.phase().impossible_measurements(&mut ledger()?)?;
    assert!(matches!(
        previous.refine_measurement_impossible(&inclusion, &horizon, &partial, &mut ledger()?),
        Err(LcRelationError::Containment)
    ));
    boundary(|budget| new.impossible_measurements(budget).is_ok())?;
    boundary(|budget| {
        previous
            .refine_measurement_impossible(&inclusion, &horizon, &certificate, budget)
            .is_ok()
    })?;
    Ok(())
}

/// An equal-valued certificate from another calibration allocation cannot replace the refined question.
/// ´test:integration:lc-relation-measurement-empty-authority´
#[test]
fn lc_relation_measurement_empty_authority() -> TestResult {
    let old = phase(false, None)?;
    let new = refined(&old)?;
    let inclusion = old.refinement_inputs(&new, &mut ledger()?)?;
    let previous = old.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
    let horizon = previous.forward_reported_horizon(&mut ledger()?)?;
    let foreign = refined(&old)?.impossible_measurements(&mut ledger()?)?;
    assert!(foreign.relation().shares_allocation(new.relation()));
    assert!(matches!(
        previous.refine_measurement_impossible(&inclusion, &horizon, &foreign, &mut ledger()?),
        Err(LcRelationError::Containment)
    ));
    let foreign_cover = old.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
    let foreign_horizon = foreign_cover.forward_reported_horizon(&mut ledger()?)?;
    let certificate = new.impossible_measurements(&mut ledger()?)?;
    assert!(matches!(
        previous.refine_measurement_impossible(
            &inclusion,
            &foreign_horizon,
            &certificate,
            &mut ledger()?
        ),
        Err(LcRelationError::Containment)
    ));
    let attempt = previous.refine(&inclusion, &horizon, cycles(), &mut ledger()?)?;
    assert!(matches!(
        attempt.state(),
        LcClockRefinementState::Undecided(_)
    ));
    Ok(())
}

/// Overlapping and merely touching calibration supports do not prove emptiness; checking spends Work.
/// ´test:integration:lc-relation-measurement-empty-refusals-work´
#[test]
fn lc_relation_measurement_empty_refusals_work() -> TestResult {
    let old = phase(false, None)?;
    assert!(matches!(
        old.impossible_measurements(&mut ledger()?),
        Err(LcRelationError::Projection)
    ));
    let touching = LcSharedClockPhase::new(
        old.clock().clone(),
        vec![interval(["2", "2"])?, interval(["9/2", "9/2"])?],
        &interval(["-1/4", "1/4"])?,
        &mut ledger()?,
    )?;
    touching.check_witness(
        physical_point()?,
        number("0")?,
        number("-1/4")?,
        &mut ledger()?,
    )?;
    assert!(matches!(
        touching.impossible_measurements(&mut ledger()?),
        Err(LcRelationError::Projection)
    ));
    let new = refined(&old)?;
    boundary(|budget| new.impossible_measurements(budget).is_ok())?;
    let certificate = new.impossible_measurements(&mut ledger()?)?;
    let inclusion = old.refinement_inputs(&new, &mut ledger()?)?;
    let previous = old.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
    let horizon = previous.forward_reported_horizon(&mut ledger()?)?;
    boundary(|budget| {
        previous
            .refine_measurement_impossible(&inclusion, &horizon, &certificate, budget)
            .is_ok()
    })?;
    assert!(new.impossible_measurements(&mut ledger()?).is_ok());
    Ok(())
}

/// Broad early declarations never hide later original phase restrictions from the contradiction proof.
/// ´test:integration:lc-relation-measurement-empty-repeated-originals´
#[test]
fn lc_relation_measurement_empty_repeated_originals() -> TestResult {
    let old = phase(false, None)?;
    let mut inputs = vec![
        super::input(Q::Phase(FIRST), ["-10", "10"])?,
        super::input(Q::Phase(SECOND), ["-10", "10"])?,
    ];
    inputs.extend_from_slice(old.relation().inputs());
    let relation = nrz_bindings::LcRelation::new(
        old.relation().epoch().into(),
        2,
        inputs.clone(),
        &mut ledger()?,
    )?;
    let clock = nrz_bindings::LcSharedClock::new(
        relation,
        old.clock().nominal_times().to_vec(),
        &interval(["-1/4", "1/4"])?,
        &mut ledger()?,
    )?;
    let question = LcSharedClockPhase::new(
        clock,
        old.samples().to_vec(),
        &interval(["-1/4", "1/4"])?,
        &mut ledger()?,
    )?;
    let new = refined(&question)?;
    let certificate = certificate(&new)?;
    assert_eq!(certificate.relation().inputs(), inputs);
    new.clock()
        .check_witness(physical_point()?, number("0")?, &mut ledger()?)?;
    Ok(())
}
