// SPDX-License-Identifier: AGPL-3.0-only
//! Pinned closed-record layouts locate single-field mutations inside complete witnesses.

use super::{InvalidCode, TestResult};
use crate::testing::answer::{index, records};

type Mutation = (&'static str, Vec<u8>, Option<InvalidCode>);

pub(super) fn changed(bytes: &[u8], kind: usize) -> TestResult<Vec<Mutation>> {
    let records = records(bytes)?;
    let (_, application) = records
        .iter()
        .rev()
        .find(|(tag, _)| *tag == 11)
        .ok_or("application")?;
    let arity = if kind < 2 { 1_u32 } else { 2_u32 };
    assert_eq!(application.len(), if kind < 2 { 27 } else { 31 });
    assert_eq!(
        bytes.get(application.start..application.start + 6),
        Some([1, 0, 0, 0, 1, 0].as_slice())
    );
    assert_eq!(
        bytes.get(application.start + 14..application.start + 18),
        Some(arity.to_le_bytes().as_slice())
    );
    let conclusion = usize::try_from(index(bytes, &mut (application.start + 10))?)?;
    let (tag, statement) = records.get(conclusion).ok_or("conclusion record")?;
    assert_eq!(*tag, 10);
    assert_eq!(statement.len(), 24);
    let subtype = statement.start + 10;
    assert_eq!(bytes.get(subtype), Some(&0));
    assert_eq!(bytes.get(subtype + 13), Some(&0));
    let mode = application.end - 1;
    assert_eq!(
        bytes.get(mode - 4..mode),
        Some(0_u32.to_le_bytes().as_slice())
    );
    assert_eq!(bytes.get(mode), Some(&0));
    let mut changes = Vec::new();
    byte(
        &mut changes,
        bytes,
        "unknown equality subtype",
        subtype,
        0,
        32,
        Some(InvalidCode::Encoding),
    )?;
    byte(
        &mut changes,
        bytes,
        "unknown certificate",
        mode,
        0,
        20,
        Some(InvalidCode::Encoding),
    )?;
    byte(
        &mut changes,
        bytes,
        "changed equality contract",
        subtype + 13,
        0,
        1,
        None,
    )?;
    let mut swapped = bytes.to_vec();
    let left = bytes.get(subtype + 1..subtype + 5).ok_or("left")?;
    let right = bytes.get(subtype + 5..subtype + 9).ok_or("right")?;
    assert_ne!(left, right);
    swapped
        .get_mut(subtype + 1..subtype + 5)
        .ok_or("left")?
        .copy_from_slice(right);
    swapped
        .get_mut(subtype + 5..subtype + 9)
        .ok_or("right")?
        .copy_from_slice(left);
    changes.push(("swapped original output", swapped, None));
    // The conclusion is a statement, so it cannot stand in an ordered proof slot.
    let mut premise = bytes.to_vec();
    assert_ne!(
        bytes.get(application.start + 18..application.start + 22),
        Some(u32::try_from(conclusion)?.to_le_bytes().as_slice())
    );
    premise
        .get_mut(application.start + 18..application.start + 22)
        .ok_or("premise")?
        .copy_from_slice(&u32::try_from(conclusion)?.to_le_bytes());
    changes.push(("statement replacing proof premise", premise, None));
    if kind == 0 {
        wrong_output(bytes, &records, subtype + 5, &mut changes)?;
    }
    manifest(bytes, kind, &mut changes)?;
    if kind < 2 {
        half_pi(bytes, &records, &mut changes)?;
    }
    if kind == 3 {
        named_pi(bytes, &records, &mut changes)?;
    }
    Ok(changes)
}

fn wrong_output(
    bytes: &[u8],
    records: &[(u8, std::ops::Range<usize>)],
    mut output_at: usize,
    changes: &mut Vec<Mutation>,
) -> TestResult {
    let output = usize::try_from(index(bytes, &mut output_at)?)?;
    let (tag, rational) = records.get(output).ok_or("rational output")?;
    assert_eq!(*tag, 3);
    assert_eq!(
        bytes.get(rational.start + 22..rational.start + 34),
        Some([0, 1, 1, 0, 0, 0, 1, 1, 0, 0, 0, 1].as_slice())
    );
    byte(
        changes,
        bytes,
        "same-shape wrong rational output",
        rational.start + 28,
        1,
        2,
        None,
    )
}

/// Change only the root's claimed condition inventory, retaining a complete decodable record.
pub(super) fn dropped_conditions(bytes: &[u8]) -> TestResult<Vec<(&'static str, Vec<u8>)>> {
    let records = records(bytes)?;
    let (_, root) = records.iter().find(|(tag, _)| *tag == 12).ok_or("root")?;
    assert_eq!(root.len(), 39);
    assert_eq!(
        bytes.get(root.start..root.start + 7),
        Some([1, 0, 0, 0, 1, 0, 1].as_slice())
    );
    assert_eq!(
        bytes.get(root.start - 4..root.start),
        Some(39_u32.to_le_bytes().as_slice())
    );
    for offset in [root.start + 15, root.start + 23] {
        assert_eq!(
            bytes.get(offset..offset + 4),
            Some(1_u32.to_le_bytes().as_slice())
        );
    }
    let guard = usize::try_from(index(bytes, &mut (root.start + 19))?)?;
    assert_eq!(records.get(guard).ok_or("guard")?.0, 9);
    assert_eq!(
        bytes.get(root.start + 31..root.end),
        Some(0_u64.to_le_bytes().as_slice())
    );
    let mut result = Vec::new();
    for (name, count, start, end) in [
        ("dropped residual", 15, 19, 23),
        ("dropped hypothesis", 23, 27, 39),
    ] {
        let mut changed = bytes.to_vec();
        changed
            .get_mut(root.start + count..root.start + count + 4)
            .ok_or("count")?
            .copy_from_slice(&0_u32.to_le_bytes());
        changed.drain(root.start + start..root.start + end);
        let length = u32::try_from(39 - (end - start))?;
        changed
            .get_mut(root.start - 4..root.start)
            .ok_or("length")?
            .copy_from_slice(&length.to_le_bytes());
        // The independent record walker still reaches the same record inventory and root.
        assert_eq!(
            crate::testing::answer::records(&changed)?.len(),
            records.len()
        );
        result.push((name, changed));
    }
    Ok(result)
}

fn manifest(bytes: &[u8], kind: usize, changes: &mut Vec<Mutation>) -> TestResult {
    let key = [
        "trig.sin-half-pi",
        "trig.cos-half-pi",
        "trig.atan-zero",
        "trig.atan-one",
    ]
    .get(kind)
    .ok_or("kind")?;
    let matches: Vec<_> = bytes
        .windows(key.len())
        .enumerate()
        .filter_map(|(offset, value)| (value == key.as_bytes()).then_some(offset))
        .collect();
    let [key_at] = matches.as_slice() else {
        return Err("one manifest key".into());
    };
    assert_eq!(
        bytes.get(key_at - 4..*key_at),
        Some(u32::try_from(key.len())?.to_le_bytes().as_slice())
    );
    let version = key_at + key.len();
    assert_eq!(
        bytes.get(version..version + 4),
        Some(1_u32.to_le_bytes().as_slice())
    );
    byte(
        changes,
        bytes,
        "unavailable schema version",
        version,
        1,
        2,
        Some(InvalidCode::UnsupportedVersion),
    )?;
    Ok(())
}

fn named_pi(
    bytes: &[u8],
    records: &[(u8, std::ops::Range<usize>)],
    changes: &mut Vec<Mutation>,
) -> TestResult {
    let (_, pi) = records
        .iter()
        .find(|(tag, range)| {
            *tag == 3 && bytes.get(range.start + 22..range.start + 24) == Some([3, 0].as_slice())
        })
        .ok_or("named Pi")?;
    assert_eq!(
        bytes.get(pi.start + 19..pi.start + 22),
        Some([0, 1, 1].as_slice())
    );
    byte(
        changes,
        bytes,
        "named Pi replaced with E",
        pi.start + 23,
        0,
        1,
        None,
    )?;
    Ok(())
}

fn half_pi(
    bytes: &[u8],
    records: &[(u8, std::ops::Range<usize>)],
    changes: &mut Vec<Mutation>,
) -> TestResult {
    let (_, statement) = records
        .iter()
        .find(|(tag, range)| *tag == 10 && bytes.get(range.start + 10) == Some(&31))
        .ok_or("half-Pi statement")?;
    let (_, application) = records
        .iter()
        .find(|(tag, range)| *tag == 11 && bytes.get(range.start + 30) == Some(&19))
        .ok_or("half-Pi application")?;
    assert_eq!(statement.len(), 34);
    assert_eq!(application.len(), 70);
    let subtype = statement.start + 10;
    let mode = application.start + 30;
    let integer = [0, 1, 0, 0, 0, 3, 1, 0, 0, 0, 1];
    assert_eq!(
        bytes.get(subtype + 5..subtype + 16),
        Some(integer.as_slice())
    );
    assert_eq!(bytes.get(mode + 5..mode + 16), Some(integer.as_slice()));
    byte(
        changes,
        bytes,
        "forged half-Pi integer",
        mode + 10,
        3,
        4,
        None,
    )?;
    let mut noninteger = bytes.to_vec();
    for offset in [subtype + 15, mode + 15] {
        assert_eq!(bytes.get(offset), Some(&1));
        *noninteger.get_mut(offset).ok_or("denominator")? = 2;
    }
    changes.push(("nonintegral half-Pi evidence", noninteger, None));
    Ok(())
}

fn byte(
    changes: &mut Vec<Mutation>,
    bytes: &[u8],
    name: &'static str,
    offset: usize,
    old: u8,
    new: u8,
    code: Option<InvalidCode>,
) -> TestResult {
    assert_eq!(bytes.get(offset), Some(&old), "{name}");
    let mut mutated = bytes.to_vec();
    *mutated.get_mut(offset).ok_or("byte offset")? = new;
    changes.push((name, mutated, code));
    Ok(())
}
