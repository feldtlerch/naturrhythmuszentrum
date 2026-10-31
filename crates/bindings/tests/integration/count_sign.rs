// SPDX-License-Identifier: AGPL-3.0-only
//! Nonnegative domains read the exact canonical sign after representation and integrality checks.

use nrz_bindings::{Budget, CountDomain, Limits, UnitError};
use nrz_cas::{Fault, FaultKind, Rational, limits::BudgetKind};

/// These storage-neutral checks need no numerical heap or decision fixture.
type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

/// Exact fixtures are constructed outside the measured domain admission.
fn ledger() -> Result<Budget, Fault> {
    Budget::new(Limits::default())
}

/// Literal success and one-below limits distinguish admission from a free sign query.
/// ´test:integration:count-sign-work-boundaries´
/// ´claim:bindings:count-sign-work-boundaries´ Fresh and repeated nonnegative count checks cost one Work, refuse at zero Work and preserve exact canonical bytes.
#[test]
fn count_sign_work_boundaries() -> TestResult {
    for warm in [false, true] {
        for admitted in [false, true] {
            let value = Rational::from_ratio(3, 1, &mut ledger()?)?;
            let original = value.canonical_bytes(&mut ledger()?)?;
            if warm {
                CountDomain::NONNEGATIVE.check_with_budget(&value, &mut ledger()?)?;
            }
            let mut budget = Budget::new(Limits {
                work: u64::from(admitted),
                ..Limits::default()
            })?;
            let result = CountDomain::NONNEGATIVE.check_with_budget(&value, &mut budget);
            if admitted {
                result?;
                assert_eq!(budget.usage().work, 1);
            } else {
                let Err(UnitError::Arithmetic(fault)) = result else {
                    panic!("zero admission Work must refuse before querying the sign");
                };
                assert!(matches!(
                    fault.kind,
                    FaultKind::Exhausted {
                        budget: BudgetKind::Work,
                        ..
                    }
                ));
                assert_eq!(budget.usage().work, 0);
            }
            assert_eq!(value.canonical_bytes(&mut ledger()?)?, original);
        }
    }
    Ok(())
}

/// Negative values and nonintegers cannot acquire count-domain authority from the shortcut.
/// ´test:integration:count-sign-refusal-twins´
/// ´claim:bindings:count-sign-refusal-twins´ Negative and fractional values still refuse after one admission Work, while zero and large representable nonnegative integers are accepted exactly.
#[test]
fn count_sign_refusal_twins() -> TestResult {
    for (spelling, accepted) in [
        ("-1", false),
        ("-340282366920938463463374607431768211456", false),
        ("1/2", false),
        ("0", true),
        ("340282366920938463463374607431768211456", true),
    ] {
        let value = Rational::parse(spelling, &mut ledger()?)?;
        let original = value.canonical_bytes(&mut ledger()?)?;
        let mut budget = Budget::new(Limits {
            work: 18,
            ..Limits::default()
        })?;
        budget.work(17, nrz_cas::Location::new("prior count work"))?;
        let result = CountDomain::NONNEGATIVE.check_with_budget(&value, &mut budget);
        if accepted {
            result?;
        } else {
            let Err(fault) = result else {
                panic!("negative or fractional count must retain its domain refusal");
            };
            assert_eq!(fault, UnitError::CountValue);
        }
        assert_eq!(budget.usage().work, 18);
        assert_eq!(value.canonical_bytes(&mut ledger()?)?, original);
    }
    Ok(())
}

/// Smaller representation ceilings still run before any exact sign-based acceptance.
/// ´test:integration:count-sign-representation-refusal´
/// ´claim:bindings:count-sign-representation-refusal´ A positive integer exceeding the caller component ceiling refuses before the sign route can admit it.
#[test]
fn count_sign_representation_refusal() -> TestResult {
    let value = Rational::from_ratio(256, 1, &mut ledger()?)?;
    let mut budget = Budget::new(Limits {
        component_bits: 8,
        ..Limits::default()
    })?;
    let Err(UnitError::Arithmetic(fault)) =
        CountDomain::NONNEGATIVE.check_with_budget(&value, &mut budget)
    else {
        panic!("oversized exact count representation must refuse");
    };
    assert!(matches!(
        fault.kind,
        FaultKind::Exhausted {
            budget: BudgetKind::ComponentBits,
            ..
        }
    ));
    assert_eq!(budget.usage().work, 1);
    Ok(())
}

/// Explicit nonzero endpoints retain exact ordered comparisons and inclusive bounds.
/// ´test:integration:count-sign-bounded-endpoints´
/// ´claim:bindings:count-sign-bounded-endpoints´ Optimizing a zero lower bound preserves all bounded-domain acceptance and refusal verdicts against the existing exact comparison route.
#[test]
fn count_sign_bounded_endpoints() -> TestResult {
    for domain in [
        CountDomain::new(Some(0), Some(4))?,
        CountDomain::new(Some(2), Some(4))?,
        CountDomain::new(Some(-4), Some(0))?,
        CountDomain::new(Some(-4), Some(4))?,
    ] {
        for numerator in -5..=5 {
            let value = Rational::from_ratio(numerator, 1, &mut ledger()?)?;
            let accepted = domain.check(&value).is_ok();
            let result = domain.check_with_budget(&value, &mut ledger()?);
            if accepted {
                result?;
            } else {
                let Err(fault) = result else {
                    panic!("a value outside the inclusive count bounds must refuse");
                };
                assert_eq!(fault, UnitError::CountValue);
            }
        }
    }
    Ok(())
}
