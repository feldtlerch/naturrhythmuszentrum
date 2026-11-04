// SPDX-License-Identifier: AGPL-3.0-only
//! One original timestamp remains free while physical phase and cycle are explicitly absent.

use super::{Error, Q, TestResult, boundary, exact_input, input, inputs, ledger, number, rational};
use nrz_bindings::{ExactConstant, LcBranch, LcInput, LcQuiescentOutcome, LcRelation};

/// The only original observation has a timestamp but no measured numeric phase.
const SAMPLE: usize = 0;

/// Repeated time declarations intersect independently of the original element rectangle.
fn supplied() -> TestResult<Vec<LcInput>> {
    let mut values = inputs()?;
    values.push(input(
        Q::Time(SAMPLE),
        Some(number("-1")?),
        Some(number("2")?),
    )?);
    values.push(input(
        Q::Time(SAMPLE),
        Some(number("-1/2")?),
        Some(number("3")?),
    )?);
    Ok(values)
}

/// Observation count is part of the original common-epoch declaration.
fn original(values: Vec<LcInput>) -> TestResult<LcRelation> {
    Ok(LcRelation::new(
        "observed rest epoch".into(),
        1,
        values,
        &mut ledger()?,
    )?)
}

/// Every timestamp in the original intersection has a full rest witness and undefined phase.
/// ´test:integration:lc-relation-quiescent-observation´
#[test]
fn lc_relation_quiescent_observation() -> TestResult {
    const ABSENT: usize = 1;
    let values = supplied()?;
    let relation = original(values.clone())?;
    let cover = relation.quiescent_cover(&mut ledger()?)?;
    assert_eq!(cover.relation().inputs(), values);
    assert_eq!(cover.relation().epoch(), "observed rest epoch");
    let time = cover.query(Q::Time(SAMPLE), &mut ledger()?)?;
    let range = time.interval().ok_or("defined original timestamp")?;
    assert_eq!(range.lower(), Some(&number("-1/2")?));
    assert_eq!(range.upper(), Some(&number("2")?));
    let phase = cover.query(Q::Phase(SAMPLE), &mut ledger()?)?;
    assert!(matches!(
        phase.outcome(),
        LcQuiescentOutcome::UndefinedPhase
    ));
    assert_eq!(phase.interval(), None);
    assert_eq!(phase.determinacy(), None);
    for time in ["-1/2", "0", "2"] {
        for element in ["1/4", "1/2", "1"] {
            let witness = cover.witness_at(
                &rational(element)?,
                &rational(element)?,
                &rational(time)?,
                &mut ledger()?,
            )?;
            assert!(witness.relation().shares_allocation(&relation));
            assert_eq!(witness.branch(), LcBranch::Quiescent);
            assert_eq!(witness.coordinate(Q::Time(SAMPLE)), Some(&number(time)?));
            assert_eq!(witness.coordinate(Q::Phase(SAMPLE)), None);
            assert_eq!(witness.values().observations[SAMPLE].cycle, None);
            relation.check_witness(witness.values().clone(), &mut ledger()?)?;
        }
    }
    for quantity in [Q::Time(ABSENT), Q::Phase(ABSENT)] {
        assert!(matches!(
            cover.query(quantity, &mut ledger()?),
            Err(Error::Observation)
        ));
    }
    Ok(())
}

/// Numeric phase, missing/unbounded inventory entries and crossed originals refuse.
/// ´test:integration:lc-relation-quiescent-observation-refusals´
#[test]
fn lc_relation_quiescent_observation_refusals() -> TestResult {
    for quantity in [Q::Phase(SAMPLE), Q::InitialPhase] {
        let mut values = supplied()?;
        values.push(exact_input(quantity, number("0")?)?);
        assert!(matches!(
            original(values)?.quiescent_cover(&mut ledger()?),
            Err(Error::Projection)
        ));
    }
    for time in [None, Some(input(Q::Time(SAMPLE), None, None)?)] {
        let mut values = inputs()?;
        values.extend(time);
        assert!(matches!(
            original(values)?.quiescent_cover(&mut ledger()?),
            Err(Error::Projection)
        ));
    }
    let extra = LcRelation::new("observed rest epoch".into(), 2, supplied()?, &mut ledger()?)?;
    assert!(matches!(
        extra.quiescent_cover(&mut ledger()?),
        Err(Error::Projection)
    ));
    let original = original(supplied()?)?;
    let cover = original.quiescent_cover(&mut ledger()?)?;
    let element = rational("1/2")?;
    for time in ["-1", "3"] {
        assert!(
            cover
                .witness_at(&element, &element, &rational(time)?, &mut ledger()?)
                .is_err()
        );
    }
    for phase in [true, false] {
        let mut wrong = cover.feasible().values().clone();
        if phase {
            wrong.observations[SAMPLE].phase = Some(ExactConstant::zero());
        } else {
            wrong.observations[SAMPLE].cycle = Some(rational("0")?);
        }
        assert!(matches!(
            original.check_witness(wrong, &mut ledger()?),
            Err(Error::QuiescentPhase)
        ));
    }
    let mut wrong = cover.feasible().values().clone();
    wrong.observations[SAMPLE].time = number("3")?;
    assert!(matches!(
        original.check_witness(wrong, &mut ledger()?),
        Err(Error::Containment)
    ));
    let mut wrong = cover.feasible().values().clone();
    wrong.rate = number("7")?;
    assert!(matches!(
        original.check_witness(wrong, &mut ledger()?),
        Err(Error::Equation)
    ));
    Ok(())
}

/// Observation construction, queries and arbitrary-time reconstruction spend cumulative Work.
/// ´test:integration:lc-relation-quiescent-observation-work´
#[test]
fn lc_relation_quiescent_observation_work() -> TestResult {
    let original = original(supplied()?)?;
    let mut measured = ledger()?;
    let cover = original.quiescent_cover(&mut measured)?;
    boundary(measured.usage().work, |budget| {
        Ok(original.quiescent_cover(budget).is_ok())
    })?;
    for quantity in [Q::Time(SAMPLE), Q::Phase(SAMPLE)] {
        let mut measured = ledger()?;
        cover.query(quantity, &mut measured)?;
        boundary(measured.usage().work, |budget| {
            Ok(cover.query(quantity, budget).is_ok())
        })?;
    }
    let element = rational("1/2")?;
    let time = rational("2")?;
    let mut measured = ledger()?;
    cover.witness_at(&element, &element, &time, &mut measured)?;
    boundary(measured.usage().work, |budget| {
        Ok(cover.witness_at(&element, &element, &time, budget).is_ok())
    })?;
    assert!(original.quiescent_cover(&mut ledger()?).is_ok());
    Ok(())
}
