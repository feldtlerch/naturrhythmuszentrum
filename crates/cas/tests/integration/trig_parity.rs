// SPDX-License-Identifier: AGPL-3.0-only
//! Independent parity expectations and complete portable evidence retain exact original joins.

use crate::testing::common::{TestResult, default_budget};
use crate::testing::half_pi::term;
use crate::testing::trig_parity::{fixture, operator};
use nrz_cas::proof::Statement;
use nrz_cas::syntax::Operator;

mod conditions;

/// Each receiver independently constructs both endpoints in either allocation order.
/// ´test:integration:trig-parity-fresh-replay´
#[test]
fn trig_parity_fresh_replay() -> TestResult {
    for kind in 0..3 {
        for nested in [false, true] {
            let (mut source, _, _) = fixture(kind, true, nested, false)?;
            let witness =
                source
                    .cas
                    .check_proof(&source.context, &source.steps, &mut default_budget()?)?;
            let bytes = source
                .cas
                .export_witness(&witness, &mut default_budget()?)?;
            for reordered in [false, true] {
                let (mut f, _, _) = fixture(kind, true, nested, reordered)?;
                let y = term(&mut f.cas, &f.header, Operator::Free("y".into()), &[])?;
                let function = term(&mut f.cas, &f.header, operator(kind), &[y])?;
                let other = if kind == 1 {
                    function
                } else {
                    term(&mut f.cas, &f.header, Operator::Neg, &[function])?
                };
                let mut changed = f.expected.clone();
                let Statement::Equality { right, .. } = &mut changed.statement else {
                    return Err("equality".into());
                };
                *right = other;
                let snapshot = f.cas.canonical_snapshot(&mut default_budget()?)?;
                assert!(
                    f.cas
                        .replay(&bytes, &changed, &mut default_budget()?)
                        .is_err()
                );
                assert_eq!(f.cas.canonical_snapshot(&mut default_budget()?)?, snapshot);
                let checked = f.cas.replay(&bytes, &f.expected, &mut default_budget()?)?;
                assert_eq!(checked.judgment(), &f.expected);
                assert_eq!(checked.hypotheses().len(), 1);
                assert_eq!(
                    f.cas.export_witness(&checked, &mut default_budget()?)?,
                    bytes
                );
            }
        }
    }
    Ok(())
}

/// Every replacement targets a complete witness with independently asserted record offsets.
/// ´test:integration:trig-parity-wire-mutations´
#[test]
fn trig_parity_wire_mutations() -> TestResult {
    use nrz_cas::{FaultKind, InvalidCode};
    for kind in 0..3 {
        let (mut source, _, _) = fixture(kind, true, true, false)?;
        let witness =
            source
                .cas
                .check_proof(&source.context, &source.steps, &mut default_budget()?)?;
        let bytes = source
            .cas
            .export_witness(&witness, &mut default_budget()?)?;
        let mutations = wire_changes(&bytes, kind)?;
        for reordered in [false, true] {
            for (changed, valid_join) in &mutations {
                let (mut receiver, _, _) = fixture(kind, true, true, reordered)?;
                let snapshot = receiver.cas.canonical_snapshot(&mut default_budget()?)?;
                let fault = receiver
                    .cas
                    .replay(changed, &receiver.expected, &mut default_budget()?)
                    .err()
                    .ok_or("wire mutation")?;
                if *valid_join {
                    assert_eq!(
                        fault.kind,
                        FaultKind::Invalid(InvalidCode::EndpointMismatch)
                    );
                }
                assert_eq!(
                    receiver.cas.canonical_snapshot(&mut default_budget()?)?,
                    snapshot
                );
                receiver
                    .cas
                    .replay(&bytes, &receiver.expected, &mut default_budget()?)?;
            }
        }
    }
    Ok(())
}

/// Pin complete record shapes before mutating each portable obligation.
fn wire_changes(bytes: &[u8], kind: usize) -> TestResult<Vec<(Vec<u8>, bool)>> {
    use crate::testing::answer::{index, records};
    let inventory = records(bytes)?;
    let (_, application) = inventory
        .iter()
        .rev()
        .find(|(tag, _)| *tag == 11)
        .ok_or("application")?;
    assert_eq!(application.len(), 27);
    assert_eq!(
        bytes.get(application.start + 14..application.start + 18),
        Some(1_u32.to_le_bytes().as_slice())
    );
    let conclusion = usize::try_from(index(bytes, &mut (application.start + 10))?)?;
    let (tag, statement) = inventory.get(conclusion).ok_or("statement")?;
    assert_eq!(*tag, 10);
    assert_eq!(statement.len(), 24);
    assert_eq!(bytes.get(statement.start + 10), Some(&0));
    assert_eq!(bytes.get(statement.end - 1), Some(&0));
    assert_eq!(bytes.get(application.end - 1), Some(&0));
    let mut mutations = Vec::new();
    let key = crate::testing::trig_parity::rule(kind)
        .specification()
        .key
        .as_bytes();
    let versions = bytes
        .windows(key.len())
        .enumerate()
        .filter_map(|(offset, word)| (word == key).then_some(offset + key.len()))
        .collect::<Vec<_>>();
    assert_eq!(versions.len(), 1);
    let version = *versions.first().ok_or("schema version")?;
    assert_eq!(
        bytes.get(version..version + 4),
        Some(1_u32.to_le_bytes().as_slice())
    );
    let mut changed = bytes.to_vec();
    changed
        .get_mut(version..version + 4)
        .ok_or("version")?
        .copy_from_slice(&2_u32.to_le_bytes());
    mutations.push((changed, false));
    for (offset, value) in [
        (statement.start + 10, 32),
        (application.end - 1, 20),
        (statement.end - 1, 1),
    ] {
        let mut changed = bytes.to_vec();
        changed[offset] = value;
        mutations.push((changed, false));
    }
    let mut changed = bytes.to_vec();
    changed
        .get_mut(statement.start + 15..statement.start + 19)
        .ok_or("output")?
        .copy_from_slice(
            bytes
                .get(statement.start + 11..statement.start + 15)
                .ok_or("input")?,
        );
    mutations.push((changed, false));
    // The earlier original guard and the totality result are both valid DomainGuard proofs.
    // Replacing Defined(sine(u)) by Defined(u) isolates the consumer's operand join.
    let earlier = inventory
        .iter()
        .position(|(tag, _)| *tag == 11)
        .ok_or("earlier guard")?;
    let original = usize::try_from(index(bytes, &mut (application.start + 18))?)?;
    assert_eq!(inventory.get(original).ok_or("original guard")?.0, 11);
    assert_ne!(earlier, original);
    let mut changed = bytes.to_vec();
    changed
        .get_mut(application.start + 18..application.start + 22)
        .ok_or("proof slot")?
        .copy_from_slice(&u32::try_from(earlier)?.to_le_bytes());
    mutations.push((changed, true));
    Ok(mutations)
}
