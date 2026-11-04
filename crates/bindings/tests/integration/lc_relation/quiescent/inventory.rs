// SPDX-License-Identifier: AGPL-3.0-only
//! Every independent timestamp is retained and checked before a rest scalar can be published.

use super::{Error, Q, TestResult, boundary, exact_input, input, inputs, ledger, number, rational};
use nrz_bindings::{ExactConstant, LcBranch, LcInput, LcQuiescentOutcome, LcRelation};

/// Three independent original timestamp intersections, including signed nonbinary endpoints.
const TIMES: [[&str; 2]; 3] = [["-1/2", "2"], ["3", "5"], ["-2/3", "1/3"]];
/// The final observation must be inspected even after earlier positions have passed.
const LAST: usize = TIMES.len() - 1;

/// Repeated originals are supplied in interleaved order, with a wider redundant timestamp.
fn supplied() -> TestResult<Vec<LcInput>> {
    let mut supplied = inputs()?;
    for (index, endpoints) in TIMES.iter().enumerate().rev() {
        supplied.push(input(
            Q::Time(index),
            Some(number("-7")?),
            Some(number("7")?),
        )?);
        supplied.push(input(
            Q::Time(index),
            Some(number(endpoints[0])?),
            Some(number(endpoints[1])?),
        )?);
    }
    Ok(supplied)
}

/// Observation positions remain part of the original common-epoch question.
fn original(supplied: Vec<LcInput>) -> TestResult<LcRelation> {
    Ok(LcRelation::new(
        "rest inventory".into(),
        TIMES.len(),
        supplied,
        &mut ledger()?,
    )?)
}

/// All timestamp extrema are attainable independently, while every phase and cycle stays absent.
/// ´test:integration:lc-relation-quiescent-inventory´
#[test]
fn lc_relation_quiescent_inventory() -> TestResult {
    let supplied = supplied()?;
    let relation = original(supplied.clone())?;
    let cover = relation.quiescent_cover(&mut ledger()?)?;
    assert_eq!(cover.relation().inputs(), supplied);
    assert_eq!(cover.relation().observations(), TIMES.len());
    for (index, endpoints) in TIMES.iter().enumerate() {
        let time = cover.query(Q::Time(index), &mut ledger()?)?;
        let range = time.interval().ok_or("original timestamp image")?;
        assert_eq!(range.lower(), Some(&number(endpoints[0])?));
        assert_eq!(range.upper(), Some(&number(endpoints[1])?));
        let phase = cover.query(Q::Phase(index), &mut ledger()?)?;
        assert!(matches!(
            phase.outcome(),
            LcQuiescentOutcome::UndefinedPhase
        ));
        assert_eq!(phase.interval(), None);
        assert_eq!(phase.determinacy(), None);
    }
    let element = rational("1/2")?;
    for first in TIMES[0] {
        for second in TIMES[1] {
            for third in TIMES[LAST] {
                let times = [rational(first)?, rational(second)?, rational(third)?];
                let witness = cover.witness_times(&element, &element, &times, &mut ledger()?)?;
                assert!(witness.relation().shares_allocation(&relation));
                assert_eq!(witness.branch(), LcBranch::Quiescent);
                for (index, time) in times.iter().enumerate() {
                    assert_eq!(
                        witness.coordinate(Q::Time(index)),
                        Some(&time.clone().into())
                    );
                    assert_eq!(witness.coordinate(Q::Phase(index)), None);
                    assert_eq!(witness.values().observations[index].cycle, None);
                }
                relation.check_witness(witness.values().clone(), &mut ledger()?)?;
            }
        }
    }
    let witness = cover.witness(&element, &element, &mut ledger()?)?;
    for (index, endpoints) in TIMES.iter().enumerate() {
        assert_eq!(
            witness.coordinate(Q::Time(index)),
            Some(&number(endpoints[0])?)
        );
    }
    Ok(())
}

/// Missing, unbounded, disjoint or numeric-phase declarations anywhere refuse the whole cover.
/// ´test:integration:lc-relation-quiescent-inventory-refusals´
#[test]
fn lc_relation_quiescent_inventory_refusals() -> TestResult {
    for index in 0..TIMES.len() {
        for missing in [true, false] {
            let mut values = supplied()?;
            values.retain(|value| value.quantity() != Q::Time(index));
            if !missing {
                values.push(input(Q::Time(index), None, None)?);
            }
            assert!(matches!(
                original(values)?.quiescent_cover(&mut ledger()?),
                Err(Error::Projection)
            ));
        }
        let mut values = supplied()?;
        values.push(exact_input(Q::Phase(index), ExactConstant::zero())?);
        assert!(matches!(
            original(values)?.quiescent_cover(&mut ledger()?),
            Err(Error::Projection)
        ));
        let mut values = supplied()?;
        values.push(exact_input(Q::Time(index), number("6")?)?);
        assert!(original(values)?.quiescent_cover(&mut ledger()?).is_err());
    }
    let relation = original(supplied()?)?;
    let cover = relation.quiescent_cover(&mut ledger()?)?;
    let element = rational("1/2")?;
    for times in [
        vec![],
        vec![rational("0")?],
        vec![rational("0")?; TIMES.len() + 1],
    ] {
        assert!(matches!(
            cover.witness_times(&element, &element, &times, &mut ledger()?),
            Err(Error::Observation)
        ));
    }
    assert!(matches!(
        cover.witness_at(&element, &element, &element, &mut ledger()?),
        Err(Error::Observation)
    ));
    for quantity in [Q::Time(TIMES.len()), Q::Phase(TIMES.len())] {
        assert!(matches!(
            cover.query(quantity, &mut ledger()?),
            Err(Error::Observation)
        ));
    }
    for index in 0..TIMES.len() {
        let mut times = TIMES
            .iter()
            .map(|endpoints| rational(endpoints[0]))
            .collect::<TestResult<Vec<_>>>()?;
        times[index] = rational("6")?;
        assert!(matches!(
            cover.witness_times(&element, &element, &times, &mut ledger()?),
            Err(Error::Containment)
        ));
        let mut wrong = cover.feasible().values().clone();
        wrong.observations[index].time = number("6")?;
        assert!(matches!(
            relation.check_witness(wrong, &mut ledger()?),
            Err(Error::Containment)
        ));
        for phase in [true, false] {
            let mut wrong = cover.feasible().values().clone();
            if phase {
                wrong.observations[index].phase = Some(ExactConstant::zero());
            } else {
                wrong.observations[index].cycle = Some(rational("0")?);
            }
            assert!(matches!(
                relation.check_witness(wrong, &mut ledger()?),
                Err(Error::QuiescentPhase)
            ));
        }
    }
    Ok(())
}

/// Complete construction, all timestamp/phase queries and reconstruction charge cumulative Work.
/// ´test:integration:lc-relation-quiescent-inventory-work´
#[test]
fn lc_relation_quiescent_inventory_work() -> TestResult {
    let relation = original(supplied()?)?;
    let mut measured = ledger()?;
    let cover = relation.quiescent_cover(&mut measured)?;
    boundary(measured.usage().work, |budget| {
        Ok(relation.quiescent_cover(budget).is_ok())
    })?;
    for index in 0..TIMES.len() {
        for quantity in [Q::Time(index), Q::Phase(index)] {
            let mut measured = ledger()?;
            cover.query(quantity, &mut measured)?;
            boundary(measured.usage().work, |budget| {
                Ok(cover.query(quantity, budget).is_ok())
            })?;
        }
    }
    let element = rational("1/2")?;
    let times = TIMES
        .iter()
        .map(|endpoints| rational(endpoints[1]))
        .collect::<TestResult<Vec<_>>>()?;
    let mut measured = ledger()?;
    cover.witness_times(&element, &element, &times, &mut measured)?;
    boundary(measured.usage().work, |budget| {
        Ok(cover
            .witness_times(&element, &element, &times, budget)
            .is_ok())
    })?;
    assert!(relation.quiescent_cover(&mut ledger()?).is_ok());
    Ok(())
}
