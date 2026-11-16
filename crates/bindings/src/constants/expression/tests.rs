// SPDX-License-Identifier: AGPL-3.0-only
//! # Test index
//!
//! | Test | Area | Claim |
//! |------|------|-------|
//! | [`expression_enclosure_completion`] | bindings | A retained earlier bound cannot hide incomplete width, interruption or resource exhaustion. |
//! | [`expression_enclosure_inventory`] | bindings | Evidence traversal has exact and one-below Work boundaries including logarithm records. |

use super::*;
use nrz_cas::{
    Reference,
    domain::{AmbientDomain, ContextDraft, HeaderDraft, RegionDraft, RegionExpr},
    syntax::{AtomicSort, Draft, Environment, ExpressionDraft, Field, Sort},
};

/// Produce genuine checked evidence for the private report classifier, not an expression export.
fn bound() -> Result<EnclosureReport, Fault> {
    bound_with_log(false)
}

/// A logarithm adds genuine logarithm records to the traversal fixture.
fn bound_with_log(log: bool) -> Result<EnclosureReport, Fault> {
    let mut b = Budget::new(nrz_cas::limits::Limits {
        work: 2_000_000_000,
        arithmetic_steps: 50_000_000,
        integer_exponent: 4096,
        literal_bytes: 4096,
        ..nrz_cas::limits::Limits::default()
    })?;
    let mut cas = Cas::new(b.limits().clone())?;
    let mut drafts = vec![
        Draft::Header(HeaderDraft::Core(Environment::default())),
        Draft::Region(RegionDraft {
            header: Reference::Local(0),
            scope: vec![],
            ambient: AmbientDomain::Assignments,
            expression: RegionExpr::Ambient,
        }),
        Draft::Context(ContextDraft::Root {
            header: Reference::Local(0),
            admissible: Reference::Local(1),
            assumptions: vec![],
        }),
        Draft::Expression(ExpressionDraft {
            space: None,
            operator: Operator::Rational(if log {
                Rational::from_ratio(2, 1, &mut b)?
            } else {
                Rational::one()
            }),
            sort: Sort::Value(AtomicSort::Scalar(Field::Real)),
            header: Reference::Local(0),
            scope: vec![],
            binding: None,
            operands: vec![],
        }),
    ];
    if log {
        drafts.push(Draft::Expression(ExpressionDraft {
            space: None,
            operator: Operator::NaturalLog,
            sort: Sort::Value(AtomicSort::Scalar(Field::Real)),
            header: Reference::Local(0),
            scope: vec![],
            binding: None,
            operands: vec![Reference::Local(3)],
        }));
    }
    let handles = cas.admit(&drafts, &mut b)?;
    cas.enclose(
        &EnclosureRequest {
            target: handles[if log { 4 } else { 3 }].clone(),
            context: handles[2].clone(),
            coordinates: vec![],
            goal: EnclosureGoal::Width,
        },
        RefinementPolicy {
            version: 1,
            precision: 160,
            grid_ceiling: 256,
            rounds: 1,
            reduction_ceiling: 4,
            term_ceiling: 128,
            checking_work: 0,
        },
        &mut b,
        &mut |_| false,
    )
}

/// A retained earlier bound cannot hide incomplete width, interruption or resource exhaustion.
/// # Panics
/// Panics only if a single changed report discriminator loses its typed refusal.
/// ´claim:bindings:expression-enclosure-completion´
/// ´test:unit:expression-enclosure-completion´
#[test]
fn expression_enclosure_completion() -> Result<(), Box<dyn std::error::Error>> {
    assert!(completed(bound()?).is_ok());
    let mut report = bound()?;
    report.precision_met = false;
    assert!(matches!(
        completed(report),
        Err(ExpressionEnclosureError::TargetNotCertifiable)
    ));
    let mut report = bound()?;
    report.outcome = EnclosureOutcome::Interrupted;
    assert!(matches!(
        completed(report),
        Err(ExpressionEnclosureError::Interrupted)
    ));
    let mut report = bound()?;
    let fault = Fault::exhausted(BudgetKind::Work, 5, 6, AT);
    report.resource_fault = Some(fault.clone());
    assert!(
        matches!(completed(report), Err(ExpressionEnclosureError::Exact(found)) if found == fault)
    );
    let mut report = bound()?;
    report.checked = None;
    assert!(matches!(
        completed(report),
        Err(ExpressionEnclosureError::TargetNotCertifiable)
    ));
    Ok(())
}

/// Evidence traversal has exact and one-below Work boundaries including logarithm records.
/// # Panics
/// Panics only if the transcript fixture or its exact traversal charge changes.
/// ´claim:bindings:expression-enclosure-inventory´
/// ´test:unit:expression-enclosure-inventory´
#[test]
fn expression_enclosure_inventory() -> Result<(), Box<dyn std::error::Error>> {
    let checked = completed(bound_with_log(true)?)?;
    let proof = checked.certificate();
    assert!(!proof.records.is_empty() && !proof.logarithms.is_empty());
    let visits = (proof.records.len() + proof.logarithms.len()) as u64;
    for (limit, expected) in [(visits, true), (visits - 1, false)] {
        let mut b = Budget::new(nrz_cas::limits::Limits {
            work: limit,
            ..nrz_cas::limits::Limits::default()
        })?;
        let result = inventory(&checked, &mut b);
        assert_eq!(
            result.is_ok(),
            expected,
            "evidence traversal Work boundary: {result:?}"
        );
        if expected {
            assert_eq!(b.usage().work, visits);
        } else {
            assert!(matches!(
                result,
                Err(Fault {
                    kind: nrz_cas::FaultKind::Exhausted {
                        budget: BudgetKind::Work,
                        ..
                    },
                    ..
                })
            ));
        }
    }
    Ok(())
}
