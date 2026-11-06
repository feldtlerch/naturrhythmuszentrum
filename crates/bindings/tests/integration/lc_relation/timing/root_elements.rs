// SPDX-License-Identifier: AGPL-3.0-only
//! Nonrational product endpoints retain a complete zero-epoch timing inventory and exact element fibres.

use super::super::support::{TestResult, ledger};
use super::support::{input, number};
use nrz_bindings::{
    Budget, LcBranch, LcCertainty, LcDeterminacy, LcJointRootParameters, LcPhaseGauge,
    LcQuantity as Q, LcRelation, LcRelationError, Limits, LimitsOverride,
};
use std::cmp::Ordering;

/// Repeated original timestamp intersections force the common epoch without discarding either input.
fn question() -> TestResult<LcRelation> {
    let mut inputs = vec![
        input(Q::Inductance, ["1", "2"])?,
        input(Q::Capacitance, ["1", "3"])?,
        input(Q::InitialPhase, ["0", "0"])?,
        input(Q::Inductance, ["1/2", "3"])?,
        input(Q::Capacitance, ["1/2", "4"])?,
    ];
    for index in 0..2 {
        inputs.push(input(Q::Time(index), ["-1", "0"])?);
        inputs.push(input(Q::Time(index), ["0", "1"])?);
        inputs.push(input(Q::Phase(index), ["0", "0"])?);
    }
    Ok(LcRelation::new(
        "root element epoch".into(),
        2,
        inputs,
        &mut ledger()?,
    )?)
}

/// Rational product and conditional capacitance select one exact physical root without rounding.
fn parameters(
    product: &str,
    capacitance: &str,
    voltage: &str,
) -> TestResult<LcJointRootParameters> {
    Ok(LcJointRootParameters {
        product: number(product)?,
        capacitance: number(capacitance)?,
        voltage: number(voltage)?,
    })
}

/// A literal product rectangle has attained inverse-root endpoints and fixes every original cycle to zero.
/// ´test:integration:lc-relation-joint-root-domain´
#[test]
fn lc_relation_joint_root_domain() -> TestResult {
    let source = question()?;
    let cover = source.joint_elements_root_cover(&mut ledger()?)?;
    assert!(cover.relation().shares_allocation(&source));
    assert_eq!(cover.relation().inputs().len(), 11);
    assert_eq!(cover.product_bounds(), &[number("1")?, number("6")?]);
    assert_eq!(cover.certainty(), LcCertainty::Certified);
    assert_eq!(cover.determinacy(), LcDeterminacy::DeterminedFinite);
    assert_eq!(cover.phase_gauge(), LcPhaseGauge::Fixed);
    let [lower, upper] = cover.rate_endpoints();
    assert_eq!(lower.exact().as_rational(), None);
    assert_eq!(upper.exact().as_rational(), Some(&number("1")?));
    for (value, expected) in [
        ("-1", Ordering::Greater),
        ("0", Ordering::Greater),
        ("2/5", Ordering::Greater),
        ("1/2", Ordering::Less),
    ] {
        assert_eq!(
            lower.compare_rational(&number(value)?, &mut ledger()?)?,
            expected
        );
    }
    assert_eq!(
        upper.compare_rational(&number("1")?, &mut ledger()?)?,
        Ordering::Equal
    );
    for (endpoint, product) in cover.rate_endpoints().iter().zip(["6", "1"]) {
        check_endpoint(&source, endpoint, product)?;
    }
    Ok(())
}

/// Both endpoint rates can remain nonrational while their original extremizers and exact order are checked.
/// ´test:integration:lc-relation-joint-root-two-radicals´
#[test]
fn lc_relation_joint_root_two_radicals() -> TestResult {
    let original = question()?;
    let mut inputs = original.inputs().to_vec();
    inputs.push(input(Q::Capacitance, ["2", "3"])?);
    let source = LcRelation::new(original.epoch().into(), 2, inputs, &mut ledger()?)?;
    let cover = source.joint_elements_root_cover(&mut ledger()?)?;
    assert_eq!(cover.product_bounds(), &[number("2")?, number("6")?]);
    for (endpoint, product) in cover.rate_endpoints().iter().zip(["6", "2"]) {
        assert_eq!(endpoint.exact().as_rational(), None);
        check_endpoint(&source, endpoint, product)?;
    }
    let upper = &cover.rate_endpoints()[1];
    assert_eq!(
        upper.compare_rational(&number("2/3")?, &mut ledger()?)?,
        Ordering::Greater
    );
    assert_eq!(
        upper.compare_rational(&number("3/4")?, &mut ledger()?)?,
        Ordering::Less
    );
    Ok(())
}

/// A collapsed element rectangle retains its exact radical rate and one original endpoint proof.
/// ´test:integration:lc-relation-joint-root-point´
#[test]
fn lc_relation_joint_root_point() -> TestResult {
    let original = question()?;
    let mut inputs = original.inputs().to_vec();
    inputs.push(input(Q::Inductance, ["1", "1"])?);
    inputs.push(input(Q::Capacitance, ["2", "2"])?);
    let source = LcRelation::new(original.epoch().into(), 2, inputs, &mut ledger()?)?;
    let cover = source.joint_elements_root_cover(&mut ledger()?)?;
    let product = number("2")?;
    assert_eq!(cover.product_bounds(), &[product.clone(), product.clone()]);
    assert_eq!(
        cover.capacitance_at_product(&product, &mut ledger()?)?,
        [product.clone(), product]
    );
    for endpoint in cover.rate_endpoints() {
        assert_eq!(endpoint.exact().as_rational(), None);
        check_endpoint(&source, endpoint, "2")?;
    }
    let witness = cover.witness(&parameters("2", "2", "3")?, &mut ledger()?)?;
    assert_eq!(witness.values().energy.as_rational(), Some(&number("9")?));
    source.check_witness(witness.values().clone(), &mut ledger()?)?;
    boundary(|budget| source.joint_elements_root_cover(budget))
}

/// Each sealed endpoint retains the unchanged original allocation and every exact observation.
fn check_endpoint(
    source: &LcRelation,
    endpoint: &nrz_bindings::LcRootRateEndpoint,
    product: &str,
) -> TestResult {
    assert_eq!(endpoint.product(), &number(product)?);
    assert_eq!(
        endpoint.exact().pow(2, &mut ledger()?)?.as_rational(),
        Some(&number("1")?.div(&number(product)?, &mut ledger()?)?)
    );
    let witness = endpoint.witness();
    assert!(witness.relation().shares_allocation(source));
    source.check_witness(witness.values().clone(), &mut ledger()?)?;
    assert_eq!(witness.branch(), LcBranch::Oscillating);
    assert_eq!(witness.values().observations.len(), 2);
    for observation in &witness.values().observations {
        assert_eq!(observation.time.as_rational(), Some(&number("0")?));
        assert_eq!(
            observation
                .phase
                .as_ref()
                .and_then(nrz_bindings::ExactConstant::as_rational),
            Some(&number("0")?)
        );
        assert_eq!(observation.cycle.as_ref(), Some(&number("0")?));
    }
    Ok(())
}

/// Independent squared physical identities verify radical interior points and conditional endpoint fibres.
/// ´test:integration:lc-relation-joint-root-physics´
#[test]
fn lc_relation_joint_root_physics() -> TestResult {
    let source = question()?;
    let cover = source.joint_elements_root_cover(&mut ledger()?)?;
    for (product, expected) in [
        ("1", ["1", "1"]),
        ("2", ["1", "2"]),
        ("3", ["3/2", "3"]),
        ("6", ["3", "3"]),
    ] {
        assert_eq!(
            cover.capacitance_at_product(&number(product)?, &mut ledger()?)?,
            [number(expected[0])?, number(expected[1])?]
        );
    }
    for (product, capacitance, voltage, inductance, current_squared, energy) in [
        ("1", "1", "1", "1", "1", "1/2"),
        ("6", "3", "1", "2", "3/2", "3/2"),
        ("2", "1", "2", "2", "2", "2"),
        ("3", "3", "1", "1", "3", "3/2"),
    ] {
        let witness = cover.witness(&parameters(product, capacitance, voltage)?, &mut ledger()?)?;
        assert!(witness.relation().shares_allocation(&source));
        for (quantity, expected) in [
            (Q::Inductance, inductance),
            (Q::Capacitance, capacitance),
            (Q::Energy, energy),
            (Q::Product, product),
        ] {
            assert_eq!(
                witness
                    .coordinate(quantity)
                    .and_then(nrz_bindings::ExactConstant::as_rational),
                Some(&number(expected)?)
            );
        }
        assert_eq!(
            witness
                .values()
                .current
                .pow(2, &mut ledger()?)?
                .as_rational(),
            Some(&number(current_squared)?)
        );
        source.check_witness(witness.values().clone(), &mut ledger()?)?;
    }
    Ok(())
}

/// Crossed element marginals, nonzero timing, absent declarations and additional physical scales refuse.
/// ´test:integration:lc-relation-joint-root-refusals´
#[test]
fn lc_relation_joint_root_refusals() -> TestResult {
    const SECOND_OBSERVATION: usize = 1;
    let source = question()?;
    let cover = source.joint_elements_root_cover(&mut ledger()?)?;
    for point in [
        parameters("6", "1", "1")?,
        parameters("1/2", "1", "1")?,
        parameters("7", "3", "1")?,
        parameters("2", "1", "0")?,
    ] {
        assert!(cover.witness(&point, &mut ledger()?).is_err());
    }
    for quantity in [
        Q::Time(SECOND_OBSERVATION),
        Q::Phase(SECOND_OBSERVATION),
        Q::InitialPhase,
        Q::Capacitance,
    ] {
        let mut inputs = source.inputs().to_vec();
        inputs.retain(|value| value.quantity() != quantity);
        let missing = LcRelation::new(source.epoch().into(), 2, inputs, &mut ledger()?)?;
        assert!(missing.joint_elements_root_cover(&mut ledger()?).is_err());
    }
    for (quantity, range) in [
        (Q::Time(SECOND_OBSERVATION), ["1", "1"]),
        (Q::Rate, ["1/4", "2"]),
        (Q::Energy, ["1", "2"]),
    ] {
        let mut inputs = source.inputs().to_vec();
        inputs.retain(|value| value.quantity() != quantity);
        inputs.push(input(quantity, range)?);
        let changed = LcRelation::new(source.epoch().into(), 2, inputs, &mut ledger()?)?;
        assert!(changed.joint_elements_root_cover(&mut ledger()?).is_err());
    }
    for coordinate in 0..3 {
        let mut values = cover.rate_endpoints()[0].witness().values().clone();
        match coordinate {
            0 => values.observations[SECOND_OBSERVATION].cycle = Some(number("1")?),
            1 => values.observations[SECOND_OBSERVATION].time = number("1")?.into(),
            _ => values.rate = number("1")?.into(),
        }
        assert!(source.check_witness(values, &mut ledger()?).is_err());
    }
    Ok(())
}

/// Admission, exact endpoint order, conditional fibres and radical reconstruction preserve cumulative Work.
/// ´test:integration:lc-relation-joint-root-work´
#[test]
fn lc_relation_joint_root_work() -> TestResult {
    let source = question()?;
    boundary(|budget| source.joint_elements_root_cover(budget))?;
    let cover = source.joint_elements_root_cover(&mut ledger()?)?;
    let product = number("3")?;
    boundary(|budget| cover.capacitance_at_product(&product, budget))?;
    let value = number("2/5")?;
    boundary(|budget| cover.rate_endpoints()[0].compare_rational(&value, budget))?;
    let parameters = parameters("2", "1", "2")?;
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
        budget.work(17, nrz_cas::Location::new("root element prefix"))?;
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
