// SPDX-License-Identifier: AGPL-3.0-only
//! An invalid witness, wrong meaning or exhausted resource never proves physical impossibility.

use super::support::{self, TestResult, exact_input, input, ledger, number, reference, relation};
use nrz_bindings::{
    Budget, Descriptor, Kind, LcInput, LcInterval, LcQuantity as Q, LcRelation,
    LcRelationError as Error, Limits, LimitsOverride, Measure, Role, Scale,
};

/// Every original algebraic coordinate and cycle equation participates in witness checking.
/// ´test:integration:lc-relation-original-equation-refusals´
#[test]
fn lc_relation_original_equation_refusals() -> TestResult {
    let original = relation(Vec::new())?;
    for position in 0..10 {
        let mut values = reference()?;
        let wrong = number("3")?;
        match position {
            0 => values.inductance = wrong,
            1 => values.capacitance = wrong,
            2 => values.rate = wrong,
            3 => values.voltage = wrong,
            4 => values.current = wrong,
            5 => values.energy = wrong,
            6 => values.initial_phase = Some(wrong),
            7 => values.observations[1].time = wrong,
            8 => values.observations[1].phase = Some(wrong),
            9 => values.observations[2].cycle = Some(support::integer(0)?),
            _ => return Err("invalid test position".into()),
        }
        assert!(matches!(
            original.check_witness(values, &mut ledger()?),
            Err(Error::Equation)
        ));
    }
    let mut values = reference()?;
    values.observations[2].cycle = Some(nrz_cas::Rational::parse("1/2", &mut ledger()?)?);
    assert!(matches!(
        original.check_witness(values, &mut ledger()?),
        Err(Error::Cycle)
    ));
    Ok(())
}

/// Positivity excludes signed amplitudes and the negative root without weakening the equations.
/// ´test:integration:lc-relation-positive-domain-refusals´
#[test]
fn lc_relation_positive_domain_refusals() -> TestResult {
    let original = relation(Vec::new())?;
    for coordinate in [
        Q::Rate,
        Q::VoltageAmplitude,
        Q::CurrentAmplitude,
        Q::Inductance,
    ] {
        let mut values = reference()?;
        match coordinate {
            Q::Rate => values.rate = number("-2000")?,
            Q::VoltageAmplitude => values.voltage = number("-5")?,
            Q::CurrentAmplitude => values.current = number("-1/10")?,
            Q::Inductance => values.inductance = number("0")?,
            _ => return Err("invalid domain test".into()),
        }
        assert!(matches!(
            original.check_witness(values, &mut ledger()?),
            Err(Error::Positivity)
        ));
    }
    Ok(())
}

/// Repeated constraints are all checked, and a failed candidate does not poison a valid attempt.
/// ´test:integration:lc-relation-complete-input-refusals´
#[test]
fn lc_relation_complete_input_refusals() -> TestResult {
    for quantity in [
        Q::AmplitudeRatio,
        Q::Energy,
        Q::Product,
        Q::InitialPhase,
        Q::Time(1),
    ] {
        let constraints = vec![
            input(quantity, None, None)?,
            exact_input(quantity, number("7")?)?,
        ];
        let original = relation(constraints)?;
        assert_eq!(original.inputs().len(), 2);
        assert!(matches!(
            original.check_witness(reference()?, &mut ledger()?),
            Err(Error::Containment)
        ));
    }
    let original = relation(Vec::new())?;
    let mut wrong = reference()?;
    wrong.energy = number("0")?;
    assert!(original.check_witness(wrong, &mut ledger()?).is_err());
    assert!(original.check_witness(reference()?, &mut ledger()?).is_ok());
    Ok(())
}

/// Equal dimensions cannot exchange rate and angular-frequency meaning or affine role.
/// ´test:integration:lc-relation-kind-and-inventory-refusals´
#[test]
fn lc_relation_kind_and_inventory_refusals() -> TestResult {
    const OUTSIDE_OBSERVATIONS: usize = 3;
    for descriptor in [
        Q::AngularFrequency.descriptor()?,
        Descriptor::new(Kind::frequency()?, Measure::coherent(), Role::Difference)?,
        Descriptor::new(Kind::rate()?, Measure::coherent(), Role::Point)?,
    ] {
        assert!(matches!(
            LcInput::new(
                Q::Rate,
                &descriptor,
                LcInterval::unbounded(),
                &mut ledger()?
            ),
            Err(Error::Meaning)
        ));
    }
    let bad_index = exact_input(Q::Phase(OUTSIDE_OBSERVATIONS), number("0")?)?;
    assert!(relation(vec![bad_index]).is_err());
    assert!(matches!(
        LcRelation::new(String::new(), 0, Vec::new(), &mut ledger()?),
        Err(Error::Epoch)
    ));
    assert!(matches!(
        LcInterval::new(Some(number("2")?), Some(number("1")?), &mut ledger()?),
        Err(Error::UnprovedOrder)
    ));
    let original = relation(Vec::new())?;
    let mut values = reference()?;
    values.observations.pop();
    assert!(matches!(
        original.check_witness(values, &mut ledger()?),
        Err(Error::Observation)
    ));
    Ok(())
}

/// Coherent unit names may differ, while nonunit scales require a separate conversion.
/// ´test:integration:lc-relation-coherent-measure-admission´
#[test]
fn lc_relation_coherent_measure_admission() -> TestResult {
    for (scale, expected) in [(1, true), (2, false)] {
        let descriptor = Descriptor::new(
            Q::Inductance.descriptor()?.kind().clone(),
            Measure::new(
                "henry",
                Scale::Exact(support::integer(scale)?),
                support::integer(0)?,
            )?,
            Role::Difference,
        )?;
        assert_eq!(
            LcInput::new(
                Q::Inductance,
                &descriptor,
                LcInterval::unbounded(),
                &mut ledger()?
            )
            .is_ok(),
            expected
        );
    }
    Ok(())
}

/// Zero energy cannot admit a supplied numeric phase or a manufactured 0/0 ratio.
/// ´test:integration:lc-relation-quiescent-domain-refusals´
#[test]
fn lc_relation_quiescent_domain_refusals() -> TestResult {
    for quantity in [Q::InitialPhase, Q::AmplitudeRatio] {
        let original = LcRelation::new(
            "rest epoch".into(),
            0,
            vec![exact_input(quantity, number("0")?)?],
            &mut ledger()?,
        )?;
        assert!(matches!(
            original.check_witness(support::quiescent()?, &mut ledger()?),
            Err(Error::QuiescentPhase)
        ));
    }
    let original = LcRelation::new("rest epoch".into(), 0, Vec::new(), &mut ledger()?)?;
    let mut values = support::quiescent()?;
    values.initial_phase = Some(number("0")?);
    assert!(matches!(
        original.check_witness(values, &mut ledger()?),
        Err(Error::QuiescentPhase)
    ));
    Ok(())
}

/// The caller's consumed prefix survives exact N/N-1 witness checking boundaries.
/// ´test:integration:lc-relation-witness-cumulative-work´
#[test]
fn lc_relation_witness_cumulative_work() -> TestResult {
    let original = relation(support::observations("1/20")?)?;
    let values = reference()?;
    let mut measured = ledger()?;
    let witness = original.check_witness(values.clone(), &mut measured)?;
    let work = measured.usage().work;
    assert_eq!(witness.usage()[1].work - witness.usage()[0].work, work);
    let prefix = 17;
    for (limit, success) in [(work + prefix, true), (work + prefix - 1, false)] {
        let limits = Limits::default().resolve(&LimitsOverride {
            work: Some(limit),
            ..LimitsOverride::default()
        })?;
        let mut budget = Budget::new(limits)?;
        budget.work(prefix, nrz_cas::Location::new("LC test prefix"))?;
        assert_eq!(
            original.check_witness(values.clone(), &mut budget).is_ok(),
            success
        );
        assert!(budget.usage().work >= prefix);
    }
    assert!(original.check_witness(values, &mut ledger()?).is_ok());
    Ok(())
}
