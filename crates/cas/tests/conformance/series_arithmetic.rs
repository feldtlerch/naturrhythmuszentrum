// SPDX-License-Identifier: AGPL-3.0-only
//! Independent finite arithmetic and precise capability boundaries.

use crate::testing::common::{TestResult, default_budget};
use crate::testing::series_proof::{fixture, rational};
use nrz_cas::proof::{Certificate, Statement};
use nrz_cas::series::SeriesCertificate as C;
use nrz_cas::{FaultKind, InvalidCode};

/// Every coefficient is chosen by hand, including all six convolution pairs and zero terms.
/// ´test:integration:series-arithmetic-hand-fixtures´
#[test]
fn series_arithmetic_hand_fixtures() -> TestResult {
    for kind in 0..12 {
        let mut f = fixture(kind, false)?;
        assert_eq!(
            f.base
                .cas
                .series_object(match &f.expected.statement {
                    Statement::Series(s) => match s.as_ref() {
                        nrz_cas::series::SeriesStatement::Truncate { input, .. }
                        | nrz_cas::series::SeriesStatement::Scale { input, .. }
                        | nrz_cas::series::SeriesStatement::Derive { input, .. }
                        | nrz_cas::series::SeriesStatement::Integrate { input, .. }
                        | nrz_cas::series::SeriesStatement::Coefficient { input, .. } => input,
                        nrz_cas::series::SeriesStatement::Add { left, .. }
                        | nrz_cas::series::SeriesStatement::Mul { left, .. } => left,
                    },
                    _ => return Err("series".into()),
                })?
                .field,
            f.field
        );
        let witness = f
            .base
            .cas
            .check_proof(&f.base.context, &f.steps, &mut default_budget()?)?;
        assert_eq!(witness.judgment(), &f.expected);
        assert!(witness.is_discharged());
        assert_eq!(witness.hypotheses(), []);
        assert_eq!(
            f.base.cas.retained_judgment_count(),
            if kind == 4 { 2 } else { 1 }
        );
        assert!(f.base.cas.retained_judgment_bytes() > 0);
        let citations = f
            .base
            .cas
            .proof_citations(witness.proof(), &mut default_budget()?)?;
        assert_eq!(citations.len(), f.steps.len());
    }
    Ok(())
}

/// A false result, missing convolution pair or altered index cannot supply finite authority.
/// ´test:integration:series-arithmetic-row-mutations´
#[test]
fn series_arithmetic_row_mutations() -> TestResult {
    for kind in 0..12 {
        let mut f = fixture(kind, false)?;
        let root = f.steps.last_mut().ok_or("root")?;
        let Certificate::Series(value) = &mut root.certificate else {
            return Err("series certificate".into());
        };
        match value.as_mut() {
            C::Truncate { coefficients, .. } => coefficients[0] = rational("9")?,
            C::Add { rows, .. } | C::Scale { rows, .. } => rows[0].result = rational("9")?,
            C::Mul { rows, .. } => {
                rows.pop();
            }
            C::Derive { rows, .. } => rows[0].result = rational("9")?,
            C::Integrate { rows, .. } => rows[0].result = rational("9")?,
            C::Coefficient { value, .. } => *value = rational("9")?,
        }
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

/// Source forms, polynomial seeds and composition are unavailable even for exact small inputs.
/// ´test:integration:series-arithmetic-deferred-forms´
#[test]
fn series_arithmetic_deferred_forms() -> TestResult {
    for kind in 0..12 {
        for form in [1, 2] {
            let mut f = fixture(kind, false)?;
            let root = f.steps.last_mut().ok_or("root")?;
            let Certificate::Series(value) = &mut root.certificate else {
                return Err("series certificate".into());
            };
            match value.as_mut() {
                C::Truncate { form: slot, .. }
                | C::Add { form: slot, .. }
                | C::Scale { form: slot, .. }
                | C::Derive { form: slot, .. }
                | C::Integrate { form: slot, .. }
                | C::Mul { form: slot, .. }
                | C::Coefficient { form: slot, .. } => *slot = form,
            }
            let before = f.base.cas.canonical_snapshot(&mut default_budget()?)?;
            let fault = f
                .base
                .cas
                .check_proof(&f.base.context, &f.steps, &mut default_budget()?)
                .expect_err("deferred form");
            assert_eq!(
                fault.kind,
                FaultKind::Invalid(InvalidCode::SeriesUnsupported)
            );
            assert_eq!(
                f.base.cas.canonical_snapshot(&mut default_budget()?)?,
                before
            );
        }
    }
    for key in [
        "series.compose",
        "series.source",
        "series.reciprocal",
        "series.divide",
        "series.invert",
        "series.residual",
    ] {
        assert!(
            !nrz_cas::proof::PRIMITIVES
                .iter()
                .any(|rule| rule.specification().key == key)
        );
    }
    let f = fixture(0, false)?;
    assert!(matches!(f.expected.statement, Statement::Series(_)));
    Ok(())
}
