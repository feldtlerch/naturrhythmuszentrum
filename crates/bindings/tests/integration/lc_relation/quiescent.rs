// SPDX-License-Identifier: AGPL-3.0-only
//! Independent rest-state interval oracles retain repeated inputs and original equation checks.

mod inventory;
mod observation;

use super::support::{TestResult, exact_input, input, ledger, number};
use nrz_bindings::{
    Budget, ExactConstant, LcBranch, LcCertainty, LcDeterminacy, LcInput, LcQuantity as Q,
    LcQuiescentCover, LcQuiescentOutcome, LcRelation, LcRelationError as Error, Limits,
    LimitsOverride,
};
use nrz_cas::{Location, Rational};

/// Repeated energy declarations intersect to exactly zero, with no individual zero singleton.
fn inputs() -> TestResult<Vec<LcInput>> {
    [
        (Q::Inductance, "1/8", "1"),
        (Q::Inductance, "1/4", "2"),
        (Q::Capacitance, "1/4", "2"),
        (Q::Capacitance, "1/8", "1"),
        (Q::Energy, "-1", "0"),
        (Q::Energy, "0", "1"),
    ]
    .into_iter()
    .map(|(quantity, lower, upper)| input(quantity, Some(number(lower)?), Some(number(upper)?)))
    .collect()
}

/// All constraints retain one original allocation and one explicitly named epoch.
fn relation(inputs: Vec<LcInput>) -> TestResult<LcRelation> {
    Ok(LcRelation::new(
        "rest epoch".into(),
        0,
        inputs,
        &mut ledger()?,
    )?)
}

/// Fixture parameters are independent exact arbitrary-precision rational literals.
fn rational(value: &str) -> TestResult<Rational> {
    Ok(Rational::parse(value, &mut ledger()?)?)
}

/// Every defined coordinate has an independently specified complete interval image.
/// ´test:integration:lc-relation-quiescent-scalar-queries´
#[test]
fn lc_relation_quiescent_scalar_queries() -> TestResult {
    let original = relation(inputs()?)?;
    let cover = original.quiescent_cover(&mut ledger()?)?;
    for (quantity, lower, upper) in [
        (Q::Inductance, "1/4", "1"),
        (Q::Capacitance, "1/4", "1"),
        (Q::Product, "1/16", "1"),
        (Q::NaturalTime, "1/4", "1"),
        (Q::Rate, "1", "4"),
        (Q::AngularFrequency, "1", "4"),
        (Q::Energy, "0", "0"),
        (Q::VoltageAmplitude, "0", "0"),
        (Q::CurrentAmplitude, "0", "0"),
    ] {
        let query = cover.query(quantity, &mut ledger()?)?;
        assert_eq!(query.quantity(), quantity);
        assert!(matches!(query.outcome(), LcQuiescentOutcome::Defined(_)));
        assert_eq!(query.determinacy(), Some(LcDeterminacy::DeterminedFinite));
        assert_eq!(query.certainty(), LcCertainty::Certified);
        assert!(query.cover().relation().shares_allocation(&original));
        let interval = query.interval().ok_or("defined scalar interval")?;
        assert_eq!(interval.lower(), Some(&number(lower)?));
        assert_eq!(interval.upper(), Some(&number(upper)?));
    }
    Ok(())
}

/// Undefined phase and undefined ratio have neither a scalar interval nor scalar determinacy.
/// ´test:integration:lc-relation-quiescent-undefined-queries´
#[test]
fn lc_relation_quiescent_undefined_queries() -> TestResult {
    const ABSENT_OBSERVATION: usize = 0;
    let cover = relation(inputs()?)?.quiescent_cover(&mut ledger()?)?;
    let phase = cover.query(Q::InitialPhase, &mut ledger()?)?;
    let ratio = cover.query(Q::AmplitudeRatio, &mut ledger()?)?;
    assert!(matches!(
        phase.outcome(),
        LcQuiescentOutcome::UndefinedPhase
    ));
    assert!(matches!(
        ratio.outcome(),
        LcQuiescentOutcome::UndefinedRatio
    ));
    for query in [phase, ratio] {
        assert_eq!(query.interval(), None);
        assert_eq!(query.determinacy(), None);
        assert_eq!(query.certainty(), LcCertainty::Certified);
    }
    for quantity in [Q::Phase(ABSENT_OBSERVATION), Q::Time(ABSENT_OBSERVATION)] {
        assert!(matches!(
            cover.query(quantity, &mut ledger()?),
            Err(Error::Observation)
        ));
    }
    Ok(())
}

/// Corner and interior tuples satisfy all retained originals; altered equations refuse.
/// ´test:integration:lc-relation-quiescent-original-witnesses´
#[test]
fn lc_relation_quiescent_original_witnesses() -> TestResult {
    let supplied = inputs()?;
    let original = relation(supplied.clone())?;
    let cover = original.quiescent_cover(&mut ledger()?)?;
    assert_eq!(cover.relation().inputs(), supplied);
    assert_eq!(cover.relation().epoch(), "rest epoch");
    assert!(cover.feasible().relation().shares_allocation(&original));
    for (inductance, capacitance, rate) in [
        ("1/4", "1/4", "4"),
        ("1/4", "1", "2"),
        ("1", "1/4", "2"),
        ("1", "1", "1"),
        ("1/2", "1/2", "2"),
    ] {
        let witness = cover.witness(
            &rational(inductance)?,
            &rational(capacitance)?,
            &mut ledger()?,
        )?;
        assert_eq!(witness.branch(), LcBranch::Quiescent);
        assert!(witness.relation().shares_allocation(&original));
        assert_eq!(witness.coordinate(Q::Rate), Some(&number(rate)?));
        assert_eq!(witness.coordinate(Q::InitialPhase), None);
        assert_eq!(witness.coordinate(Q::AmplitudeRatio), None);
        assert_eq!(witness.coordinate(Q::Energy), Some(&number("0")?));
        let mut wrong = witness.values().clone();
        wrong.rate = number("7")?;
        assert!(matches!(
            original.check_witness(wrong, &mut ledger()?),
            Err(Error::Equation)
        ));
    }
    for (inductance, capacitance) in [("1/8", "1/2"), ("1/2", "2"), ("0", "1")] {
        assert!(
            cover
                .witness(
                    &rational(inductance)?,
                    &rational(capacitance)?,
                    &mut ledger()?
                )
                .is_err()
        );
    }
    let mut wrong = cover.feasible().values().clone();
    wrong.initial_phase = Some(ExactConstant::zero());
    assert!(matches!(
        original.check_witness(wrong, &mut ledger()?),
        Err(Error::QuiescentPhase)
    ));
    Ok(())
}

/// No original constraint is silently omitted or interpreted as a marginal approximation.
/// ´test:integration:lc-relation-quiescent-original-refusals´
#[test]
fn lc_relation_quiescent_original_refusals() -> TestResult {
    for quantity in [
        Q::InitialPhase,
        Q::AmplitudeRatio,
        Q::Rate,
        Q::VoltageAmplitude,
    ] {
        let mut supplied = inputs()?;
        supplied.push(exact_input(quantity, number("0")?)?);
        assert!(matches!(
            relation(supplied)?.quiescent_cover(&mut ledger()?),
            Err(Error::Projection)
        ));
    }
    for (quantity, lower, upper) in [
        (Q::Energy, "1", "2"),
        (Q::Inductance, "2", "3"),
        (Q::Capacitance, "0", "0"),
    ] {
        let mut supplied = inputs()?;
        supplied.push(input(quantity, Some(number(lower)?), Some(number(upper)?))?);
        assert!(relation(supplied)?.quiescent_cover(&mut ledger()?).is_err());
    }
    for energy in [None, Some(["0", "1"]), Some(["1", "1"])] {
        let mut supplied = inputs()?;
        supplied.retain(|value| value.quantity() != Q::Energy);
        if let Some([lower, upper]) = energy {
            supplied.push(input(
                Q::Energy,
                Some(number(lower)?),
                Some(number(upper)?),
            )?);
        }
        assert!(matches!(
            relation(supplied)?.quiescent_cover(&mut ledger()?),
            Err(Error::Projection)
        ));
    }
    let observed = LcRelation::new("rest epoch".into(), 1, inputs()?, &mut ledger()?)?;
    assert!(matches!(
        observed.quiescent_cover(&mut ledger()?),
        Err(Error::Projection)
    ));
    let mut supplied = inputs()?;
    supplied.retain(|value| value.quantity() != Q::Capacitance);
    supplied.push(input(Q::Capacitance, None, None)?);
    assert!(matches!(
        relation(supplied)?.quiescent_cover(&mut ledger()?),
        Err(Error::Projection)
    ));
    Ok(())
}

/// Construction, scalar/undefined queries and reconstruction share exact N/N-1 Work boundaries.
/// ´test:integration:lc-relation-quiescent-cumulative-work´
#[test]
fn lc_relation_quiescent_cumulative_work() -> TestResult {
    let original = relation(inputs()?)?;
    let mut measured = ledger()?;
    let cover = original.quiescent_cover(&mut measured)?;
    assert_eq!(
        cover.usage()[1].work - cover.usage()[0].work,
        measured.usage().work
    );
    boundary(measured.usage().work, |budget| {
        Ok(original.quiescent_cover(budget).is_ok())
    })?;
    for quantity in [Q::Rate, Q::InitialPhase, Q::AmplitudeRatio] {
        let mut measured = ledger()?;
        cover.query(quantity, &mut measured)?;
        boundary(measured.usage().work, |budget| {
            Ok(cover.query(quantity, budget).is_ok())
        })?;
    }
    witness_work(&cover)?;
    assert!(original.quiescent_cover(&mut ledger()?).is_ok());
    Ok(())
}

/// Resource refusals leave the shared original owner reusable for exact witness reconstruction.
fn witness_work(cover: &LcQuiescentCover) -> TestResult {
    let parameter = rational("1/2")?;
    let mut measured = ledger()?;
    cover.witness(&parameter, &parameter, &mut measured)?;
    boundary(measured.usage().work, |budget| {
        Ok(cover.witness(&parameter, &parameter, budget).is_ok())
    })
}

/// A consumed caller prefix is charged on both the success and exhaustion attempts.
fn boundary(work: u64, operation: impl Fn(&mut Budget) -> TestResult<bool>) -> TestResult {
    let prefix = 17;
    assert!(work > 0);
    for (limit, succeeds) in [(work + prefix, true), (work + prefix - 1, false)] {
        let limits = Limits::default().resolve(&LimitsOverride {
            work: Some(limit),
            ..LimitsOverride::default()
        })?;
        let mut budget = Budget::new(limits)?;
        budget.work(prefix, Location::new("quiescent test prefix"))?;
        assert_eq!(operation(&mut budget)?, succeeds);
        assert!(budget.usage().work >= prefix);
    }
    Ok(())
}
