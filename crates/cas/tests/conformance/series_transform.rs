// SPDX-License-Identifier: AGPL-3.0-only
//! Coordinate substitution retains semantic axes and refuses binder capture atomically.

use crate::testing::builders::{admit, environment, expression_draft};
use crate::testing::common::{TestResult, default_budget, real};
use nrz_cas::domain::{AmbientDomain, ContextDraft, HeaderDraft, RegionDraft, RegionExpr};
use nrz_cas::limits::{Budget, BudgetKind, Limits};
use nrz_cas::series::SeriesObject;
use nrz_cas::solving::{CoefficientDomainData, SolveDraft};
use nrz_cas::syntax::{Draft, Field, Operator, Substitution};
use nrz_cas::{Cas, Handle, Location, Rational, Reference};

fn fixture() -> TestResult<(Cas, Handle, Handle, Handle, Handle)> {
    let mut cas = Cas::new(Limits::default())?;
    let header = admit(
        &mut cas,
        Draft::Header(HeaderDraft::Core(environment(Field::Real, &["x", "y"]))),
        &mut default_budget()?,
    )?;
    let nodes = cas.admit(
        &[
            Draft::Region(RegionDraft {
                header: Reference::Existing(header.clone()),
                scope: Vec::new(),
                ambient: AmbientDomain::Assignments,
                expression: RegionExpr::Ambient,
            }),
            Draft::Context(ContextDraft::Root {
                header: Reference::Existing(header.clone()),
                admissible: Reference::Local(0),
                assumptions: Vec::new(),
            }),
            Draft::Solve(SolveDraft::CoefficientDomain(
                CoefficientDomainData::Rational,
            )),
            expression_draft(
                Operator::Free("x".into()),
                real(),
                Reference::Existing(header.clone()),
                Vec::new(),
                None,
                &[],
            ),
            expression_draft(
                Operator::Free("y".into()),
                real(),
                Reference::Existing(header.clone()),
                Vec::new(),
                None,
                &[],
            ),
            expression_draft(
                Operator::Rational(Rational::zero()),
                real(),
                Reference::Existing(header.clone()),
                Vec::new(),
                None,
                &[],
            ),
            expression_draft(
                Operator::Rational(Rational::one()),
                real(),
                Reference::Existing(header.clone()),
                Vec::new(),
                None,
                &[],
            ),
        ],
        &mut default_budget()?,
    )?;
    let object = admit(
        &mut cas,
        Draft::SeriesObject(SeriesObject {
            field: Reference::Existing(nodes[2].clone()),
            centre: Reference::Existing(nodes[5].clone()),
            variable: Reference::Existing(nodes[3].clone()),
            scope: Vec::new(),
            context: Reference::Existing(nodes[1].clone()),
            order: 3,
            coefficients: vec![
                Reference::Existing(nodes[6].clone()),
                Reference::Existing(nodes[5].clone()),
                Reference::Existing(nodes[6].clone()),
            ],
        }),
        &mut default_budget()?,
    )?;
    Ok((cas, header, object, nodes[3].clone(), nodes[4].clone()))
}

/// The entire coordinate identity changes while field, centre, context, order and coefficients stay fixed.
/// ´test:integration:series-coordinate-substitution-identity´
#[test]
fn series_coordinate_substitution_identity() -> TestResult {
    let (mut cas, _, object, x, y) = fixture()?;
    let before = cas.series_object(&object)?.clone();
    let transformed = cas.substitute(
        &object,
        &[
            Substitution {
                key: "x".into(),
                replacement: y.clone(),
            },
            Substitution {
                key: "y".into(),
                replacement: x,
            },
        ],
        &mut default_budget()?,
    )?;
    let after = cas.series_object(&transformed)?;
    assert_ne!(object, transformed);
    assert_eq!(after.variable, y);
    assert_eq!(
        (
            &after.field,
            &after.centre,
            &after.context,
            after.order,
            &after.coefficients
        ),
        (
            &before.field,
            &before.centre,
            &before.context,
            before.order,
            &before.coefficients
        )
    );
    let step = nrz_cas::proof::ApplicationDraft {
        rule: nrz_cas::proof::Primitive::SeriesTruncate,
        context: before.context.clone(),
        conclusion: nrz_cas::proof::Statement::Series(Box::new(
            nrz_cas::series::SeriesStatement::Truncate {
                input: object,
                output: transformed,
            },
        )),
        premises: Vec::new(),
        substitutions: Vec::new(),
        certificate: nrz_cas::proof::Certificate::Series(Box::new(
            nrz_cas::series::SeriesCertificate::Truncate {
                form: 0,
                coefficients: vec![Rational::one(), Rational::zero(), Rational::one()],
            },
        )),
    };
    let snapshot = cas.canonical_snapshot(&mut default_budget()?)?;
    assert!(
        cas.check_proof(&before.context, &[step], &mut default_budget()?)
            .is_err()
    );
    assert_eq!(cas.canonical_snapshot(&mut default_budget()?)?, snapshot);
    Ok(())
}

/// A replacement from a binder frame cannot become a captured series coordinate.
/// ´test:integration:series-coordinate-capture-refusal´
#[test]
fn series_coordinate_capture_refusal() -> TestResult {
    let (mut cas, header, object, _, _) = fixture()?;
    let bound = admit(
        &mut cas,
        expression_draft(
            Operator::Bound { depth: 0, slot: 0 },
            real(),
            Reference::Existing(header),
            vec![vec![nrz_cas::syntax::AtomicSort::Scalar(Field::Real)]],
            None,
            &[],
        ),
        &mut default_budget()?,
    )?;
    let before = cas.canonical_snapshot(&mut default_budget()?)?;
    assert!(
        cas.substitute(
            &object,
            &[Substitution {
                key: "x".into(),
                replacement: bound
            }],
            &mut default_budget()?
        )
        .is_err()
    );
    assert_eq!(cas.canonical_snapshot(&mut default_budget()?)?, before);
    Ok(())
}

fn attempt(kind: BudgetKind, amount: u64) -> TestResult<(bool, u64)> {
    let (mut cas, _, object, _, y) = fixture()?;
    let mut limits = crate::testing::half_pi_limits::ceiling(kind, amount);
    match kind {
        BudgetKind::JetOrder => limits.jet_order = amount,
        BudgetKind::JetEntries => limits.jet_entries = amount,
        _ => {}
    }
    let mut budget = Budget::new(limits)?;
    budget.work(37, Location::new("prior series substitution"))?;
    let before = cas.canonical_snapshot(&mut default_budget()?)?;
    let result = budget.with_retained_payload(
        113,
        Location::new("retained series substitution"),
        |budget| {
            cas.substitute(
                &object,
                &[Substitution {
                    key: "x".into(),
                    replacement: y.clone(),
                }],
                budget,
            )
        },
    );
    if let Err(fault) = &result {
        assert!(
            matches!(fault.kind,nrz_cas::FaultKind::Exhausted {budget,..} if budget==kind),
            "{fault:?}"
        );
        assert_eq!(cas.canonical_snapshot(&mut default_budget()?)?, before);
    } else {
        assert_eq!(
            cas.series_object(result.as_ref().map_err(|_| "substitution")?)?
                .variable,
            y
        );
    }
    Ok((result.is_ok(), budget.usage().work))
}

const DIMENSIONS: [BudgetKind; 8] = [
    BudgetKind::Work,
    BudgetKind::StagedPayloadBytes,
    BudgetKind::PayloadBytes,
    BudgetKind::Vertices,
    BudgetKind::Edges,
    BudgetKind::Depth,
    BudgetKind::JetOrder,
    BudgetKind::JetEntries,
];

/// Each independent complete ceiling includes prior usage and preserves the refused snapshot.
/// ´test:integration:series-substitution-resource-boundaries´
#[test]
fn series_substitution_resource_boundaries() -> TestResult {
    for ((kind, ceiling), prefix) in DIMENSIONS
        .into_iter()
        .zip([27_356, 24_001, 5544, 12, 27, 1, 2, 3])
        .zip([
            27_278, 24_381, 27_356, 27_356, 27_356, 26_840, 24_637, 24_637,
        ])
    {
        assert_eq!(attempt(kind, ceiling)?, (true, 27_356));
        assert_eq!(attempt(kind, ceiling - 1)?, (false, prefix));
    }
    Ok(())
}

/// Discovery measures complete independent operations before literal boundary enforcement.
/// ´test:integration:series-substitution-measurement´
#[test]
#[ignore = "explicit complete-operation measurement"]
#[expect(
    clippy::print_stdout,
    reason = "Explicit ignored discovery emits complete measured rows for literal boundary tests"
)]
fn series_substitution_measurement() -> TestResult {
    for kind in DIMENSIONS {
        let (mut lo, mut hi) = (
            if kind == BudgetKind::Work { 37 } else { 0 },
            default_budget()?.limits().get(kind),
        );
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            if attempt(kind, mid)?.0 {
                hi = mid;
            } else {
                lo = mid + 1;
            }
        }
        println!(
            "SUBSTITUTION {kind:?} {lo} {:?} {:?}",
            attempt(kind, lo)?,
            if lo > 0 {
                attempt(kind, lo - 1)?
            } else {
                (false, 0)
            }
        );
    }
    Ok(())
}
