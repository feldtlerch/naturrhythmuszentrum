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
    fixture_with_twins(false)
}

fn fixture_with_twins(twins: bool) -> TestResult<(Cas, Handle, Handle, Handle, Handle)> {
    let mut cas = Cas::new(Limits::default())?;
    let mut declarations = environment(Field::Real, &["x", "y"]);
    if twins {
        for declaration in &mut declarations.declarations {
            declaration.display_name = "x".into();
        }
    }
    let header = admit(
        &mut cas,
        Draft::Header(HeaderDraft::Core(declarations)),
        &mut default_budget()?,
    )?;
    let nodes = cas.admit(
        &[
            Draft::Region(RegionDraft {
                header: Reference::Existing(header.clone()),
                scope: Vec::new(),
                ambient: if twins {
                    AmbientDomain::Real("x".into())
                } else {
                    AmbientDomain::Assignments
                },
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

/// Equal-spelled declarations remain distinct through calculus and capture-safe object substitution.
/// ´test:integration:series-calculus-coordinate-twins´
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "The coordinate identity control keeps both hand certificates and rollback together"
)]
fn series_calculus_coordinate_twins() -> TestResult {
    use crate::testing::series_proof::{defined, rational};
    use nrz_cas::proof::{ApplicationDraft, Certificate, Statement};
    use nrz_cas::series::{
        SeriesCertificate as C, SeriesDerivativeRow, SeriesIntegralRow, SeriesOperation as O,
        SeriesStatement as S,
    };
    for integrate in [false, true] {
        let (mut cas, header, input, x, y) = fixture_with_twins(true)?;
        let original = cas.series_object(&input)?.clone();
        let domain = cas.admissible_region(&original.context)?;
        let source = admit(
            &mut cas,
            Draft::Source {
                key: "calculus-transform".into(),
                text: "formal coordinate fixture".into(),
            },
            &mut default_budget()?,
        )?;
        let mut base = crate::testing::cases::Fixture {
            cas,
            header,
            context: original.context.clone(),
            domain,
            source,
            x,
            zero: original.coefficients[1].clone(),
            one: original.coefficients[0].clone(),
        };
        let mut steps = Vec::new();
        let mut proofs = Vec::new();
        let (operation, certificate) = if integrate {
            let constant = base.zero.clone();
            steps.push(defined(&mut base, &constant)?);
            proofs.push(
                base.cas
                    .check_proof(&base.context, &steps, &mut default_budget()?)?
                    .proof()
                    .clone(),
            );
            let mut rows = Vec::new();
            for (index, integer, inverse, input, result) in [
                (0, "1", "1", "1", "1"),
                (1, "2", "1/2", "0", "0"),
                (2, "3", "1/3", "1", "1/3"),
            ] {
                rows.push(SeriesIntegralRow {
                    index,
                    integer: rational(integer)?,
                    inverse: rational(inverse)?,
                    input: rational(input)?,
                    result: rational(result)?,
                });
            }
            (
                O::Integrate { input, constant },
                C::Integrate { form: 0, rows },
            )
        } else {
            let mut rows = Vec::new();
            for (index, input, multiplier, result) in [(0, "0", "1", "0"), (1, "1", "2", "2")] {
                rows.push(SeriesDerivativeRow {
                    index,
                    input: rational(input)?,
                    multiplier: rational(multiplier)?,
                    result: rational(result)?,
                });
            }
            (O::Derive { input }, C::Derive { form: 0, rows })
        };
        let witness = base
            .cas
            .compute_series(&operation, &proofs, &mut default_budget()?)?;
        let Statement::Series(mut statement) = witness.judgment().statement.clone() else {
            return Err("series".into());
        };
        let (S::Derive { output, .. } | S::Integrate { output, .. }) = statement.as_mut() else {
            return Err("calculus".into());
        };
        let changed = base.cas.substitute(
            output,
            &[Substitution {
                key: "x".into(),
                replacement: y.clone(),
            }],
            &mut default_budget()?,
        )?;
        assert_eq!(base.cas.series_object(&changed)?.variable, y);
        assert_ne!(output, &changed);
        *output = changed;
        steps.push(ApplicationDraft {
            rule: statement.primitive(),
            context: base.context.clone(),
            conclusion: Statement::Series(statement),
            premises: if integrate { vec![0] } else { Vec::new() },
            substitutions: Vec::new(),
            certificate: Certificate::Series(Box::new(certificate)),
        });
        let before = base.cas.canonical_snapshot(&mut default_budget()?)?;
        assert_eq!(
            base.cas
                .check_proof(&base.context, &steps, &mut default_budget()?)
                .expect_err("coordinate identity")
                .kind,
            nrz_cas::FaultKind::Invalid(nrz_cas::InvalidCode::Rule)
        );
        assert_eq!(base.cas.canonical_snapshot(&mut default_budget()?)?, before);
    }
    Ok(())
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
