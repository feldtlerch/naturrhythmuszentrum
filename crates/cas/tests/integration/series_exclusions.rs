// SPDX-License-Identifier: AGPL-3.0-only
//! Exact finite judgments do not import analytic bounds or foreign checked authority.

use crate::testing::common::{TestResult, default_budget};
use crate::testing::series_proof::fixture;
use nrz_cas::enclosure::{EnclosureGoal, EnclosureOutcome, EnclosureRequest, RefinementPolicy};

/// A valid rational enclosure remains a bound and cannot be replayed as finite coefficient evidence.
/// ´test:integration:series-enclosure-wire-separation´
#[test]
fn series_enclosure_wire_separation() -> TestResult {
    let mut source = fixture(2, false)?;
    let request = EnclosureRequest {
        target: source.base.one.clone(),
        context: source.base.context.clone(),
        coordinates: Vec::new(),
        goal: EnclosureGoal::Width,
    };
    let policy = RefinementPolicy {
        version: 1,
        precision: 4,
        grid_ceiling: 12,
        rounds: 1,
        reduction_ceiling: 1,
        term_ceiling: 16,
        checking_work: 1_000_000,
    };
    let report = source
        .base
        .cas
        .enclose(&request, policy, &mut default_budget()?, &mut |_| false)?;
    assert_eq!(report.outcome, EnclosureOutcome::Bound);
    let checked = report.checked.ok_or("checked enclosure")?;
    let bytes = checked.certificate().to_bytes(&mut default_budget()?)?;
    assert_eq!(&bytes[..8], b"NRZENCW1");
    for reordered in [false, true] {
        let mut receiver = fixture(2, reordered)?;
        let before = receiver
            .base
            .cas
            .canonical_snapshot(&mut default_budget()?)?;
        assert!(
            receiver
                .base
                .cas
                .replay(&bytes, &receiver.expected, &mut default_budget()?)
                .is_err()
        );
        assert_eq!(
            receiver
                .base
                .cas
                .canonical_snapshot(&mut default_budget()?)?,
            before
        );
    }
    Ok(())
}

/// A matching receiving-store theorem still cannot export another store's checked receipt.
/// ´test:integration:series-foreign-witness-refused´
#[test]
fn series_foreign_witness_refused() -> TestResult {
    let mut source = fixture(2, false)?;
    let witness =
        source
            .base
            .cas
            .check_proof(&source.base.context, &source.steps, &mut default_budget()?)?;
    let bytes = source
        .base
        .cas
        .export_witness(&witness, &mut default_budget()?)?;
    for reordered in [false, true] {
        let mut receiver = fixture(2, reordered)?;
        let before = receiver
            .base
            .cas
            .canonical_snapshot(&mut default_budget()?)?;
        assert!(
            receiver
                .base
                .cas
                .export_witness(&witness, &mut default_budget()?)
                .is_err()
        );
        assert_eq!(
            receiver
                .base
                .cas
                .canonical_snapshot(&mut default_budget()?)?,
            before
        );
        assert!(
            receiver
                .base
                .cas
                .replay(&bytes, &receiver.expected, &mut default_budget()?)?
                .is_discharged()
        );
    }
    Ok(())
}

/// A checked rational enclosure contract is not a finite arithmetic certificate.
/// ´test:integration:series-enclosure-typed-evidence-refused´
#[test]
fn series_enclosure_typed_evidence_refused() -> TestResult {
    use nrz_cas::decision::{EnclosureOrigin, RationalEnclosure};
    use nrz_cas::proof::Certificate;
    use nrz_cas::{FaultKind, InvalidCode, Rational};
    for reordered in [false, true] {
        let mut f = fixture(3, reordered)?;
        let enclosure = RationalEnclosure {
            target: f.base.one.clone(),
            context: f.base.context.clone(),
            input_family: f.base.domain.clone(),
            lower: Rational::one(),
            upper: Rational::one(),
            origin: EnclosureOrigin::ExactRational {
                expression: f.base.one.clone(),
                value: Rational::one(),
            },
        };
        let checked = f
            .base
            .cas
            .check_rational_enclosure(&enclosure, &mut default_budget()?)?;
        assert_eq!(checked.contract(), &enclosure);
        let mut changed = f.steps.clone();
        changed.last_mut().ok_or("series computation")?.certificate =
            Certificate::IntegerSelection(Box::new(enclosure));
        let before = f.base.cas.canonical_snapshot(&mut default_budget()?)?;
        assert_eq!(
            f.base
                .cas
                .check_proof(&f.base.context, &changed, &mut default_budget()?)
                .expect_err("enclosure cannot supply finite arithmetic evidence")
                .kind,
            FaultKind::Invalid(InvalidCode::Rule)
        );
        assert_eq!(
            f.base.cas.canonical_snapshot(&mut default_budget()?)?,
            before
        );
        let positive = f
            .base
            .cas
            .check_proof(&f.base.context, &f.steps, &mut default_budget()?)?;
        let bytes = f
            .base
            .cas
            .export_witness(&positive, &mut default_budget()?)?;
        f.base
            .cas
            .replay(&bytes, &f.expected, &mut default_budget()?)?;
    }
    Ok(())
}
