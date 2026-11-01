// SPDX-License-Identifier: AGPL-3.0-only
//! Complete half-Pi witnesses replay against independently constructed receiving claims.

use crate::testing::common::{TestResult, default_budget};
use crate::testing::half_pi::{Fixture, prepared};
use nrz_cas::{FaultKind, InvalidCode};

fn encoded(integer: &str, coefficient: &str, zero: bool) -> TestResult<Vec<u8>> {
    let mut source = prepared(integer, coefficient, zero, false)?;
    let proof = source
        .cas
        .check_proof(&source.context, &source.steps, &mut default_budget()?)?;
    Ok(source.cas.export_witness(&proof, &mut default_budget()?)?)
}

/// Receiving stores prepare syntax and expected judgments without running a proof checker.
/// ´test:integration:half-pi-fresh-replay´
#[test]
fn half_pi_fresh_replay() -> TestResult {
    for (integer, coefficient, zero) in [
        ("3", "3/2", false),
        ("0", "0", true),
        ("-18446744073709551617", "-18446744073709551617/2", false),
    ] {
        let bytes = encoded(integer, coefficient, zero)?;
        for reordered in [false, true] {
            let mut destination = prepared(integer, coefficient, zero, reordered)?;
            let witness =
                destination
                    .cas
                    .replay(&bytes, &destination.expected, &mut default_budget()?)?;
            assert_eq!(witness.judgment(), &destination.expected);
            assert_eq!(witness.hypotheses().len(), 1);
            assert_eq!(
                destination
                    .cas
                    .export_witness(&witness, &mut default_budget()?)?,
                bytes
            );
        }
    }
    Ok(())
}

/// Offsets identify the new closed forms inside complete accepted portable witnesses.
/// ´test:integration:half-pi-replay-mutations´
#[test]
fn half_pi_replay_mutations() -> TestResult {
    let bytes = encoded("3", "3/2", false)?;
    let records = crate::testing::answer::records(&bytes)?;
    let statement = records
        .iter()
        .find(|(tag, range)| *tag == 10 && bytes.get(range.start + 10) == Some(&31))
        .map(|(_, range)| range.clone())
        .ok_or("half-Pi statement record")?;
    let application = records
        .iter()
        .find(|(tag, range)| *tag == 11 && bytes.get(range.start + 30) == Some(&19))
        .map(|(_, range)| range.clone())
        .ok_or("half-Pi certificate record")?;
    // Version, layer and absent space occupy six bytes; statement context adds four.
    assert_eq!(statement.len(), 34);
    assert_eq!(
        bytes.get(statement.start..statement.start + 6),
        Some([1, 0, 0, 0, 1, 0].as_slice())
    );
    let subtype = statement.start + 10;
    let mode = application.start + 30;
    assert_eq!(application.len(), 70);
    // Rule, conclusion, two ordinary premise references and no substitutions precede the mode.
    assert_eq!(
        bytes.get(application.start + 14..application.start + 18),
        Some(2_u32.to_le_bytes().as_slice())
    );
    assert_eq!(
        bytes.get(application.start + 26..mode),
        Some(0_u32.to_le_bytes().as_slice())
    );
    assert_eq!(
        bytes.get(mode + 1..mode + 24),
        bytes.get(subtype + 1..subtype + 24)
    );
    assert_eq!(
        bytes.get(subtype + 5..subtype + 16),
        Some([0, 1, 0, 0, 0, 3, 1, 0, 0, 0, 1].as_slice())
    );
    assert_eq!(
        bytes.get(mode + 24..mode + 32),
        Some(0_u64.to_le_bytes().as_slice())
    );
    assert_eq!(
        bytes.get(mode + 32..mode + 40),
        Some(1_u64.to_le_bytes().as_slice())
    );
    let mut mutations = Vec::new();
    for (offset, old, new, code) in [
        (subtype, 31, 32, InvalidCode::Encoding),
        (mode, 19, 20, InvalidCode::Encoding),
        (mode + 24, 0, 1, InvalidCode::EndpointMismatch),
        (mode + 10, 3, 4, InvalidCode::EndpointMismatch),
        (subtype + 5, 0, 2, InvalidCode::Encoding),
    ] {
        assert_eq!(bytes.get(offset), Some(&old));
        let mut changed = bytes.clone();
        *changed.get_mut(offset).ok_or("mutation offset")? = new;
        mutations.push((changed, code));
    }
    // The alternative is independently valid; only its mismatch with the expected claim refuses.
    let alternative = encoded("4", "2", false)?;
    let mut control = prepared("4", "2", false, true)?;
    control
        .cas
        .replay(&alternative, &control.expected, &mut default_budget()?)?;
    for reordered in [false, true] {
        let mut destination = prepared("3", "3/2", false, reordered)?;
        for (changed, code) in &mutations {
            refused(&mut destination, changed, *code)?;
        }
        refused(
            &mut destination,
            &alternative,
            InvalidCode::EndpointMismatch,
        )?;
        let witness =
            destination
                .cas
                .replay(&bytes, &destination.expected, &mut default_budget()?)?;
        assert_eq!(witness.judgment(), &destination.expected);
        assert_eq!(
            destination
                .cas
                .export_witness(&witness, &mut default_budget()?)?,
            bytes
        );
    }
    Ok(())
}

fn refused(destination: &mut Fixture, bytes: &[u8], code: InvalidCode) -> TestResult {
    let counts = destination.cas.counts();
    let before = destination.cas.canonical_snapshot(&mut default_budget()?)?;
    let fault = destination
        .cas
        .replay(bytes, &destination.expected, &mut default_budget()?)
        .err()
        .ok_or("mutated half-Pi witness accepted")?;
    assert_eq!(fault.kind, FaultKind::Invalid(code));
    assert_eq!(destination.cas.counts(), counts);
    assert_eq!(
        destination.cas.canonical_snapshot(&mut default_budget()?)?,
        before
    );
    Ok(())
}
