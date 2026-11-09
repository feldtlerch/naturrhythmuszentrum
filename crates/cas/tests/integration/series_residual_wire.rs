// SPDX-License-Identifier: AGPL-3.0-only
//! Independently stated residual wire payloads and complete mixed replay expectations.

use super::series_golden::{count, join, number};
use crate::testing::common::{TestResult, default_budget};
use crate::testing::series_residual::{Fixture, Term, binary, member, rational};
use nrz_cas::syntax::Operator;
use nrz_cas::{FaultKind, InvalidCode};

/// A real mode-eleven positive precedes the legacy malformed-discriminator controls.
#[expect(
    clippy::too_many_lines,
    reason = "the independent wire payload and mutation offsets remain together"
)]
pub(super) fn payload_control() -> TestResult {
    let mut f = Fixture::new(&["1"], "0", false, false)?;
    let subject = f.compose()?;
    let term = binary(Operator::Sub, Term::Y(0), Term::Q("1"));
    let (expression, rows) = f.term(&term, 1)?;
    let (residual, solve) = f.cas.check_evaluated_residual(
        &subject,
        &expression,
        &f.y,
        &rows,
        Some(&member()),
        &mut default_budget()?,
    )?;
    let bytes = f.cas.export_witness(&residual, &mut default_budget()?)?;
    let records = crate::testing::answer::records(&bytes)?;
    let (_, statement) = records
        .iter()
        .rfind(|(tag, _)| *tag == 39)
        .ok_or("residual statement")?;
    let (_, certificate) = records
        .iter()
        .rfind(|(tag, _)| *tag == 40)
        .ok_or("residual certificate")?;
    let statement = statement.start + 37;
    assert_eq!(&bytes[statement..statement + 6], &[11, 0, 3, 0, 5, 0]);
    assert_eq!(bytes[statement + 14], 0);
    assert_eq!(&bytes[statement + 19..statement + 24], &[1, 0, 0, 0, 1]);
    assert_eq!(
        &bytes[statement + 40..statement + 48],
        &[0, 0, 0, 0, 1, 0, 0, 0]
    );
    let certificate_start = certificate.start;
    assert_eq!(bytes[certificate_start + 10], 11);
    assert_eq!(
        &bytes[certificate_start + 11..certificate_start + 15],
        &[1, 0, 0, 0]
    );
    let certificate = certificate_start + 19;
    let golden = join(&[
        vec![11, 0, 0],
        count(3),
        count(0),
        count(1),
        number(false, 1, 1),
        count(1),
        count(1),
        number(false, 1, 1),
        count(2),
        count(1),
        number(false, 0, 1),
    ]);
    assert_eq!(&bytes[certificate..certificate + golden.len()], golden);
    let mut receiver = Fixture::new(&["1"], "0", true, false)?;
    let (expected_residual, expected_solve) = receiver.expectations(&term, &[rational("0")?], 0)?;
    receiver
        .cas
        .replay(&bytes, &expected_residual, &mut default_budget()?)?;
    let solving_bytes = f
        .cas
        .export_witness(&solve.ok_or("solve")?, &mut default_budget()?)?;
    receiver.cas.replay_evaluated_residual(
        &solving_bytes,
        &expected_residual,
        &expected_solve,
        &mut default_budget()?,
    )?;
    let mut mutations = Vec::new();
    for stage in [1, 2, 255] {
        mutations.push((statement + 3, 0, stage, InvalidCode::SeriesUnsupported));
        mutations.push((certificate + 2, 0, stage, InvalidCode::SeriesUnsupported));
    }
    for form in [1, 2, 255] {
        mutations.push((statement + 1, 0, form, InvalidCode::SeriesUnsupported));
        mutations.push((certificate + 1, 0, form, InvalidCode::SeriesUnsupported));
    }
    for result in [0, 1, 2, 255] {
        mutations.push((statement + 2, 3, result, InvalidCode::SeriesUnsupported));
    }
    mutations.extend([
        (statement + 4, 5, 11, InvalidCode::SeriesUnsupported),
        (statement + 23, 1, 2, InvalidCode::Encoding),
        (certificate + 7, 0, 1, InvalidCode::Rule),
        (certificate_start + 10, 11, 5, InvalidCode::Rule),
        (certificate_start, 1, 2, InvalidCode::UnsupportedVersion),
        (certificate + 15, 0, 2, InvalidCode::Encoding),
        (certificate + 25, 1, 0, InvalidCode::Encoding),
    ]);
    for (offset, original, replacement, code) in mutations {
        assert_eq!(bytes[offset], original);
        let mut changed = bytes.clone();
        changed[offset] = replacement;
        let before = receiver.cas.canonical_snapshot(&mut default_budget()?)?;
        let fault = receiver
            .cas
            .inspect(&changed, &mut default_budget()?)
            .expect_err("malformed residual wire");
        assert_eq!(fault.kind, FaultKind::Invalid(code), "offset {offset}");
        assert_eq!(
            before,
            receiver.cas.canonical_snapshot(&mut default_budget()?)?
        );
    }
    for (length, code) in [
        (0, InvalidCode::UnsupportedVersion),
        (7, InvalidCode::UnsupportedVersion),
        (bytes.len() - 1, InvalidCode::Encoding),
    ] {
        assert_eq!(
            receiver
                .cas
                .inspect(&bytes[..length], &mut default_budget()?)
                .expect_err("truncated residual wire")
                .kind,
            FaultKind::Invalid(code)
        );
    }
    let mut trailing = bytes;
    trailing.push(0);
    assert_eq!(
        receiver
            .cas
            .inspect(&trailing, &mut default_budget()?)
            .expect_err("trailing residual wire byte")
            .kind,
        FaultKind::Invalid(InvalidCode::Encoding)
    );
    Ok(())
}

/// The complete positive uses the existing object/Compose precedents and an independent dense trace.
/// ´test:integration:residual-wire-payload-and-mutations´
#[test]
fn residual_wire_payload_and_mutations() -> TestResult {
    payload_control()
}
