// SPDX-License-Identifier: AGPL-3.0-only
//! Formal object admission retains exact coefficients and semantic axes.

use crate::testing::common::{TestResult, default_budget};
use crate::testing::series::{admit, fixture};
use nrz_cas::limits::{Budget, BudgetKind};
use nrz_cas::solving::{CoefficientDomainData, SolveDraft};
use nrz_cas::syntax::{Draft, Operator, Substitution};
use nrz_cas::{FaultKind, InvalidCode, Reference};

/// Fractional and large coefficients retain every position, including zero tails.
/// ´test:integration:series-object-exact-identity´
#[test]
fn series_object_exact_identity() -> TestResult {
    for values in [
        &["0"][..],
        &["1", "2", "0"],
        &["-1/2", "18446744073709551617", "0"],
    ] {
        let (mut f, object) = fixture(values)?;
        let handle = admit(&mut f, object.clone())?;
        assert_eq!(handle, admit(&mut f, object.clone())?);
        let retained = f.cas.series_object(&handle)?;
        assert_eq!(usize::try_from(retained.order)?, values.len());
        for (coefficient, expected) in retained.coefficients.iter().zip(values) {
            let Operator::Rational(value) = f.cas.expression(coefficient)?.0 else {
                return Err("coefficient category".into());
            };
            assert_eq!(value.exact_string(&mut default_budget()?)?, *expected);
        }
        let described = f.cas.describe_syntax(&handle, &mut default_budget()?)?;
        assert_eq!(described.dependencies.len(), values.len() + 4);
        if values.len() > 1 {
            let mut shorter = object;
            shorter.order = 1;
            shorter.coefficients.truncate(1);
            assert_ne!(handle, admit(&mut f, shorter)?);
        }
    }
    Ok(())
}

/// Malformed extents and unavailable interpretations publish no partial object.
/// ´test:integration:series-object-admission-refusals´
#[test]
fn series_object_admission_refusals() -> TestResult {
    for change in 0..6 {
        let (mut f, mut object) = fixture(&["1", "2", "0"])?;
        match change {
            0 => object.order = 0,
            1 => {
                object.coefficients.pop();
            }
            2 => object.variable = Reference::Existing(f.one.clone()),
            3 => object.centre = Reference::Existing(f.x.clone()),
            4 => object.coefficients[0] = Reference::Existing(f.x.clone()),
            _ => {
                let field = f
                    .cas
                    .admit(
                        &[Draft::Solve(SolveDraft::CoefficientDomain(
                            CoefficientDomainData::Gaussian,
                        ))],
                        &mut default_budget()?,
                    )?
                    .pop()
                    .ok_or("field")?;
                object.field = Reference::Existing(field);
            }
        }
        let snapshot = f.cas.canonical_snapshot(&mut default_budget()?)?;
        assert!(matches!(
            f.cas
                .admit(&[Draft::SeriesObject(object)], &mut default_budget()?)
                .err()
                .ok_or("accepted mutation")?
                .kind,
            FaultKind::Invalid(_)
        ));
        assert_eq!(f.cas.canonical_snapshot(&mut default_budget()?)?, snapshot);
    }
    Ok(())
}

/// Substitution visits the coordinate but cannot make it a composite increment.
/// ´test:integration:series-object-substitution´
#[test]
fn series_object_substitution() -> TestResult {
    let (mut f, object) = fixture(&["1", "2", "0"])?;
    let handle = admit(&mut f, object)?;
    assert_eq!(
        f.cas.substitute(&handle, &[], &mut default_budget()?)?,
        handle
    );
    let replacement = f.term(Operator::Add, &[f.x.clone(), f.one.clone()])?;
    let snapshot = f.cas.canonical_snapshot(&mut default_budget()?)?;
    let refusal = f
        .cas
        .substitute(
            &handle,
            &[Substitution {
                key: "x".into(),
                replacement,
            }],
            &mut default_budget()?,
        )
        .err()
        .ok_or("composite coordinate accepted")?;
    assert_eq!(
        refusal.kind,
        FaultKind::Invalid(InvalidCode::UnsupportedFragment)
    );
    assert_eq!(f.cas.canonical_snapshot(&mut default_budget()?)?, snapshot);
    Ok(())
}

/// Existing jet ceilings bound exponent and count before payload copying.
/// ´test:integration:series-object-order-entry-ceilings´
#[test]
fn series_object_order_entry_ceilings() -> TestResult {
    for kind in [BudgetKind::JetOrder, BudgetKind::JetEntries] {
        for succeeds in [true, false] {
            let (mut f, object) = fixture(&["1", "2", "0"])?;
            let snapshot = f.cas.canonical_snapshot(&mut default_budget()?)?;
            let mut limits = default_budget()?.limits().clone();
            match kind {
                BudgetKind::JetOrder => limits.jet_order = if succeeds { 2 } else { 1 },
                BudgetKind::JetEntries => limits.jet_entries = if succeeds { 3 } else { 2 },
                _ => return Err("counter".into()),
            }
            let mut budget = Budget::new(limits)?;
            let result = f.cas.admit(&[Draft::SeriesObject(object)], &mut budget);
            if succeeds {
                result?;
                assert_eq!(budget.usage().jet_order, 2);
                assert_eq!(budget.usage().jet_entries, 3);
            } else {
                assert!(
                    matches!(result.err().ok_or("ceiling accepted")?.kind, FaultKind::Exhausted { budget: actual, .. } if actual == kind)
                );
                assert_eq!(budget.usage().work, 0);
                assert_eq!(f.cas.canonical_snapshot(&mut default_budget()?)?, snapshot);
            }
        }
    }
    Ok(())
}
