// SPDX-License-Identifier: AGPL-3.0-only
//! Original metadata and ordered proof slots cannot be replaced by plausible equal-shaped data.

use crate::testing::common::{TestResult, default_budget};
use crate::testing::series_proof::{defined, fixture, object, rational, step};
use nrz_cas::proof::{Certificate, Statement};
use nrz_cas::series::{SeriesCertificate as C, SeriesObject, SeriesStatement as S};
use nrz_cas::syntax::Draft;
use nrz_cas::{FaultKind, InvalidCode, Reference};

/// Equal coefficient vectors do not erase centre, context or exact order identity.
/// ´test:integration:series-metadata-joins´
#[test]
fn series_metadata_joins() -> TestResult {
    use nrz_cas::domain::{AmbientDomain, ContextDraft, RegionDraft, RegionExpr};
    for change in 0..3 {
        let mut f = fixture(0, false)?;
        let context = if change == 1 {
            let nodes = f.base.cas.admit(
                &[
                    Draft::Region(RegionDraft {
                        header: Reference::Existing(f.base.header.clone()),
                        scope: Vec::new(),
                        ambient: AmbientDomain::Real("x".into()),
                        expression: RegionExpr::Meet(vec![Reference::Existing(
                            f.base.domain.clone(),
                        )]),
                    }),
                    Draft::Context(ContextDraft::Root {
                        header: Reference::Existing(f.base.header.clone()),
                        admissible: Reference::Local(0),
                        assumptions: Vec::new(),
                    }),
                ],
                &mut default_budget()?,
            )?;
            nodes[1].clone()
        } else {
            f.base.context.clone()
        };
        let coefficients = if change == 2 {
            vec![
                Reference::Existing(f.base.one.clone()),
                Reference::Existing(f.base.zero.clone()),
            ]
        } else {
            vec![Reference::Existing(f.base.one.clone())]
        };
        let output = f
            .base
            .cas
            .admit(
                &[Draft::SeriesObject(SeriesObject {
                    field: Reference::Existing(f.field.clone()),
                    centre: Reference::Existing(if change == 0 {
                        f.base.one.clone()
                    } else {
                        f.base.zero.clone()
                    }),
                    variable: Reference::Existing(f.base.x.clone()),
                    scope: Vec::new(),
                    context: Reference::Existing(context),
                    order: u32::try_from(coefficients.len())?,
                    coefficients,
                })],
                &mut default_budget()?,
            )?
            .pop()
            .ok_or("twin")?;
        let Statement::Series(value) = &mut f.steps[0].conclusion else {
            return Err("series".into());
        };
        let S::Truncate { output: slot, .. } = value.as_mut() else {
            return Err("truncate".into());
        };
        *slot = output;
        let before = f.base.cas.canonical_snapshot(&mut default_budget()?)?;
        assert!(matches!(
            f.base
                .cas
                .check_proof(&f.base.context, &f.steps, &mut default_budget()?)
                .map_err(|e| e.kind),
            Err(FaultKind::Invalid(_))
        ));
        assert_eq!(
            f.base.cas.canonical_snapshot(&mut default_budget()?)?,
            before
        );
    }
    Ok(())
}

/// A valid ordinary proof for another scalar and a valid truncation of another input both refuse.
/// ´test:integration:series-valid-premise-joins´
#[test]
fn series_valid_premise_joins() -> TestResult {
    for kind in [2, 4, 9, 10] {
        for missing in [false, true] {
            let mut f = fixture(kind, false)?;
            if missing {
                f.steps.last_mut().ok_or("root")?.premises.clear();
            } else if kind != 4 {
                let scalar = f.base.number("11")?;
                f.steps[0] = defined(&mut f.base, &scalar)?;
            } else {
                let input = object(&mut f.base, &f.field, &["9", "2", "3"])?;
                let output = object(&mut f.base, &f.field, &["9", "2"])?;
                f.steps[0] = step(
                    &f.base,
                    S::Truncate { input, output },
                    C::Truncate {
                        form: 0,
                        coefficients: vec![rational("9")?, rational("2")?],
                    },
                    Vec::new(),
                );
            }
            if !missing {
                assert!(
                    f.base
                        .cas
                        .check_proof(&f.base.context, &f.steps[..1], &mut default_budget()?)?
                        .is_discharged()
                );
            }
            let before = f.base.cas.canonical_snapshot(&mut default_budget()?)?;
            assert_eq!(
                f.base
                    .cas
                    .check_proof(&f.base.context, &f.steps, &mut default_budget()?)
                    .expect_err("missing or mismatched premise")
                    .kind,
                FaultKind::Invalid(InvalidCode::Rule)
            );
            assert_eq!(
                f.base.cas.canonical_snapshot(&mut default_budget()?)?,
                before
            );
        }
    }
    Ok(())
}

/// An unavailable coefficient is not invented zero, and a larger truncation is not an extension.
/// ´test:integration:series-precision-refusals´
#[test]
fn series_precision_refusals() -> TestResult {
    for kind in [0, 5] {
        let mut f = fixture(kind, false)?;
        if kind == 0 {
            let output = object(&mut f.base, &f.field, &["1", "2", "3", "0"])?;
            let Statement::Series(s) = &mut f.steps[0].conclusion else {
                return Err("series".into());
            };
            let S::Truncate { output: slot, .. } = s.as_mut() else {
                return Err("truncation".into());
            };
            *slot = output;
        } else {
            let Statement::Series(s) = &mut f.steps[0].conclusion else {
                return Err("series".into());
            };
            let S::Coefficient { index, .. } = s.as_mut() else {
                return Err("coefficient".into());
            };
            *index = 3;
            let Certificate::Series(c) = &mut f.steps[0].certificate else {
                return Err("certificate".into());
            };
            let C::Coefficient { index, .. } = c.as_mut() else {
                return Err("coefficient".into());
            };
            *index = 3;
        }
        let before = f.base.cas.canonical_snapshot(&mut default_budget()?)?;
        assert_eq!(
            f.base
                .cas
                .check_proof(&f.base.context, &f.steps, &mut default_budget()?)
                .expect_err("unavailable precision")
                .kind,
            FaultKind::Invalid(InvalidCode::SeriesUnsupported)
        );
        assert_eq!(
            f.base.cas.canonical_snapshot(&mut default_budget()?)?,
            before
        );
    }
    Ok(())
}

/// A later refused transaction does not erase an earlier graph-owned checked judgment.
/// ´test:integration:series-retained-rollback-authority´
#[test]
fn series_retained_rollback_authority() -> TestResult {
    let mut f = fixture(2, false)?;
    let witness = f
        .base
        .cas
        .check_proof(&f.base.context, &f.steps, &mut default_budget()?)?;
    let original = f
        .base
        .cas
        .export_witness(&witness, &mut default_budget()?)?;
    let before = f.base.cas.canonical_snapshot(&mut default_budget()?)?;
    f.steps.last_mut().ok_or("root")?.premises.clear();
    assert!(
        f.base
            .cas
            .check_proof(&f.base.context, &f.steps, &mut default_budget()?)
            .is_err()
    );
    assert_eq!(
        f.base.cas.canonical_snapshot(&mut default_budget()?)?,
        before
    );
    assert_eq!(
        f.base
            .cas
            .export_witness(&witness, &mut default_budget()?)?,
        original
    );
    Ok(())
}

/// An admitted wide coefficient vector is borrowed before an unsupported precision exit.
/// ´test:integration:series-producer-prepaid-early-exit´
#[test]
fn series_producer_prepaid_early_exit() -> TestResult {
    use nrz_cas::limits::{Budget, BudgetKind, Limits};
    use nrz_cas::series::SeriesOperation;
    let mut f = fixture(0, false)?;
    let wide = "9".repeat(200);
    let input = object(&mut f.base, &f.field, &[wide.as_str(); 8])?;
    let before = f.base.cas.canonical_snapshot(&mut default_budget()?)?;
    for work in [256, 257] {
        let mut budget = Budget::new(Limits {
            work,
            staged_payload_bytes: 1024,
            ..Limits::default()
        })?;
        let fault = f
            .base
            .cas
            .compute_series(
                &SeriesOperation::Truncate {
                    input: input.clone(),
                    order: 9,
                },
                &[],
                &mut budget,
            )
            .expect_err("unavailable precision");
        if work == 257 {
            assert_eq!(
                fault.kind,
                FaultKind::Invalid(InvalidCode::SeriesUnsupported)
            );
        } else {
            assert!(matches!(
                fault.kind,
                FaultKind::Exhausted {
                    budget: BudgetKind::Work,
                    ..
                }
            ));
        }
        assert_eq!(budget.usage().work, work);
        assert_eq!(
            f.base.cas.canonical_snapshot(&mut default_budget()?)?,
            before
        );
    }
    Ok(())
}
