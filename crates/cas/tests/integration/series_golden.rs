// SPDX-License-Identifier: AGPL-3.0-only
//! Hand-authored mixed witness; no production encoder or mathematical producer builds its bytes.

use crate::testing::common::{TestResult, default_budget};
use crate::testing::series_proof::fixture;
use std::collections::BTreeMap;

/// One literal byte segment or symbolic earlier-record reference.
enum Part {
    Bytes(Vec<u8>),
    Ref(&'static str),
}
/// Independently stated record grammar, without a graph or production record descriptor.
struct Record {
    name: &'static str,
    tag: u8,
    layer: u8,
    parts: Vec<Part>,
}

/// The wire collection width is four bytes.
fn count(value: u32) -> Vec<u8> {
    value.to_le_bytes().to_vec()
}
/// Hand literal name encoding.
fn name(value: &str) -> Vec<u8> {
    let mut out = count(u32::try_from(value.len()).expect("fixture name"));
    out.extend_from_slice(value.as_bytes());
    out
}
/// The fixture needs only one-byte magnitudes, including canonical empty zero numerator.
fn number(negative: bool, numerator: u8, denominator: u8) -> Vec<u8> {
    let mut out = vec![u8::from(negative)];
    out.extend(count(u32::from(numerator != 0)));
    if numerator != 0 {
        out.push(numerator);
    }
    out.extend(count(1));
    out.push(denominator);
    out
}
/// A literal segment keeps all semantic tags visible at its declaration.
const fn bytes(value: Vec<u8>) -> Part {
    Part::Bytes(value)
}
/// Concatenate independently specified fixed wire fields.
fn join(parts: &[Vec<u8>]) -> Vec<u8> {
    parts.concat()
}

/// A scalar leaf retains the registered Real ambient and has no binder or case payload.
fn expression(key: &'static str, operator: Vec<u8>) -> Record {
    Record {
        name: key,
        tag: 3,
        layer: 1,
        parts: vec![
            Part::Ref("header"),
            bytes(join(&[count(0), vec![1]])),
            Part::Ref("real"),
            bytes(join(&[
                vec![0, 1, 1],
                operator,
                vec![0],
                count(0),
                vec![0, 0],
                count(0),
            ])),
        ],
    }
}

/// The two canonical intersections are the explicit partition used by the checked case law.
fn region(key: &'static str, members: &[&'static str]) -> Record {
    let mut parts = vec![
        Part::Ref("header"),
        bytes(join(&[
            count(0),
            vec![1],
            name("x"),
            vec![if members.is_empty() { 0 } else { 4 }],
        ])),
    ];
    if !members.is_empty() {
        parts.push(bytes(count(
            u32::try_from(members.len()).expect("fixture members"),
        )));
        for member in members {
            parts.push(Part::Ref(member));
        }
    }
    parts.push(bytes(vec![0]));
    Record {
        name: key,
        tag: 1,
        layer: 0,
        parts,
    }
}

/// Both order-three objects retain the exact interpretation, coordinate and original context.
fn object(key: &'static str, coefficients: [&'static str; 3]) -> Record {
    let mut parts = vec![
        Part::Ref("Q"),
        Part::Ref("zero"),
        Part::Ref("x"),
        bytes(count(0)),
        Part::Ref("context"),
        bytes(join(&[vec![0], count(3), count(3)])),
    ];
    for value in coefficients {
        parts.push(Part::Ref(value));
    }
    Record {
        name: key,
        tag: 38,
        layer: 0,
        parts,
    }
}

/// Complete used-law closure is hand-stated and alphabetically ordered.
fn closure() -> Vec<u8> {
    join(&[
        count(2),
        name("case.domain"),
        count(1),
        name("series.scale"),
        count(1),
    ])
}

/// Hand-authored scale by minus one half, with the ordinary checked Defined fact, no assumption.
#[expect(
    clippy::too_many_lines,
    reason = "The independent golden DAG retains its complete hand-authored record grammar"
)]
fn records() -> Vec<Record> {
    let mut values = vec![
        Record {
            name: "real",
            tag: 34,
            layer: 0,
            parts: vec![bytes(join(&[name("scalar.real"), count(1)]))],
        },
        Record {
            name: "complex",
            tag: 34,
            layer: 0,
            parts: vec![bytes(join(&[name("scalar.complex"), count(1)]))],
        },
        Record {
            name: "header",
            tag: 0,
            layer: 0,
            parts: vec![
                bytes(join(&[
                    vec![0, 1, 0],
                    count(1),
                    name("x"),
                    name("x"),
                    vec![0, 1, 1],
                    count(2),
                ])),
                Part::Ref("real"),
                Part::Ref("complex"),
                bytes(count(0)),
            ],
        },
        region("ambient", &[]),
        region("singleton-meet", &["ambient"]),
        region("intersection", &["ambient", "singleton-meet"]),
        Record {
            name: "context",
            tag: 8,
            layer: 0,
            parts: vec![
                Part::Ref("header"),
                bytes(vec![0]),
                Part::Ref("ambient"),
                bytes(join(&[vec![0], count(0)])),
            ],
        },
        Record {
            name: "Q",
            tag: 17,
            layer: 0,
            parts: vec![bytes(vec![0])],
        },
        expression("x", join(&[vec![1], name("x")])),
    ];
    for (key, negative, numerator, denominator) in [
        ("zero", false, 0, 1),
        ("one", false, 1, 1),
        ("two", false, 2, 1),
        ("three", false, 3, 1),
        ("half", true, 1, 2),
        ("minus-one", true, 1, 1),
        ("minus-three-halves", true, 3, 2),
    ] {
        values.push(expression(
            key,
            join(&[vec![0], number(negative, numerator, denominator)]),
        ));
    }
    values.extend([
        Record {
            name: "defined",
            tag: 7,
            layer: 0,
            parts: vec![
                Part::Ref("header"),
                bytes(join(&[count(0), vec![0], count(1)])),
                Part::Ref("half"),
            ],
        },
        Record {
            name: "ordinary-statement",
            tag: 10,
            layer: 0,
            parts: vec![
                Part::Ref("context"),
                bytes(vec![1]),
                Part::Ref("ambient"),
                Part::Ref("defined"),
            ],
        },
        Record {
            name: "ordinary-proof",
            tag: 11,
            layer: 0,
            parts: vec![
                bytes(count(0)),
                Part::Ref("ordinary-statement"),
                bytes(join(&[count(0), count(0), vec![6], count(2)])),
                Part::Ref("ambient"),
                Part::Ref("intersection"),
                bytes(join(&[
                    count(0),
                    count(0),
                    count(1),
                    count(0),
                    count(2),
                    count(1),
                    vec![1],
                    count(1),
                    vec![1],
                ])),
            ],
        },
        object("input", ["one", "two", "three"]),
        object("output", ["half", "minus-one", "minus-three-halves"]),
        Record {
            name: "series-statement",
            tag: 39,
            layer: 0,
            parts: vec![
                bytes(join(&[count(1), name("series.scale"), count(1)])),
                Part::Ref("context"),
                bytes(vec![2, 0]),
                Part::Ref("input"),
                Part::Ref("half"),
                bytes(vec![0]),
                Part::Ref("output"),
                bytes(join(&[count(0), count(0), closure()])),
            ],
        },
    ]);
    let mut rows = join(&[vec![2, 0], count(3)]);
    for (index, b, c, d) in [(0, 1, 1, 2), (1, 2, 1, 1), (2, 3, 3, 2)] {
        rows.extend(count(index));
        rows.extend(number(true, 1, 2));
        rows.extend(number(false, b, 1));
        rows.extend(number(true, c, d));
    }
    values.push(Record {
        name: "series-certificate",
        tag: 40,
        layer: 0,
        parts: vec![
            Part::Ref("series-statement"),
            bytes(join(&[vec![2], count(1)])),
            Part::Ref("ordinary-proof"),
            bytes(rows),
        ],
    });
    values.push(Record {
        name: "conclusion",
        tag: 41,
        layer: 0,
        parts: vec![
            Part::Ref("series-statement"),
            Part::Ref("series-certificate"),
            bytes(count(1)),
            Part::Ref("ordinary-proof"),
            bytes(closure()),
        ],
    });
    values
}

/// Canonical order follows the independently stated minimum-ready-record grammar.
fn golden() -> TestResult<Vec<u8>> {
    let records = records();
    let mut assigned = BTreeMap::new();
    let mut encoded = Vec::new();
    while assigned.len() < records.len() {
        let mut candidates = Vec::new();
        for record in &records {
            if assigned.contains_key(record.name) {
                continue;
            }
            let mut payload = Vec::new();
            let mut ready = true;
            for part in &record.parts {
                match part {
                    Part::Bytes(value) => payload.extend(value),
                    Part::Ref(key) => {
                        if let Some(index) = assigned.get(key) {
                            payload.extend(count(*index));
                        } else {
                            ready = false;
                            break;
                        }
                    }
                }
            }
            if ready {
                let mut value = vec![record.tag];
                value.extend(count(u32::try_from(payload.len() + 6)?));
                value.extend(count(1));
                value.extend([record.layer, 0]);
                value.extend(payload);
                candidates.push((value, record.name));
            }
        }
        candidates.sort();
        let (value, key) = candidates.into_iter().next().ok_or("cyclic golden")?;
        assigned.insert(key, u32::try_from(encoded.len())?);
        encoded.push(value);
    }
    let mut result = b"NRZCAS05".to_vec();
    result.extend(closure());
    result.extend(count(u32::try_from(encoded.len())?));
    for record in encoded {
        result.extend(record);
    }
    result.extend(count(*assigned.get("conclusion").ok_or("golden root")?));
    Ok(result)
}

/// A checked case-domain dependency and records 38–41 replay without production-generated bytes.
/// ´test:integration:series-independent-golden´
#[test]
fn series_independent_golden() -> TestResult {
    let bytes = golden()?;
    let mut source = fixture(2, false)?;
    let checked =
        source
            .base
            .cas
            .check_proof(&source.base.context, &source.steps, &mut default_budget()?)?;
    let actual = source
        .base
        .cas
        .export_witness(&checked, &mut default_budget()?)?;
    if actual != bytes {
        let offset = actual
            .iter()
            .zip(&bytes)
            .position(|(a, b)| a != b)
            .unwrap_or_else(|| actual.len().min(bytes.len()));
        return Err(format!(
            "golden mismatch lengths {}/{} at {offset}: actual {:?}, hand {:?}",
            actual.len(),
            bytes.len(),
            actual.get(offset.saturating_sub(8)..(offset + 40).min(actual.len())),
            bytes.get(offset.saturating_sub(8)..(offset + 40).min(bytes.len()))
        )
        .into());
    }
    for reordered in [false, true] {
        let mut f = fixture(2, reordered)?;
        let witness = f
            .base
            .cas
            .replay(&bytes, &f.expected, &mut default_budget()?)?;
        assert!(witness.is_discharged());
        assert_eq!(witness.hypotheses(), []);
        assert_eq!(
            f.base
                .cas
                .export_witness(&witness, &mut default_budget()?)?,
            bytes
        );
        let rules = f
            .base
            .cas
            .proof_citations(witness.proof(), &mut default_budget()?)?;
        assert_eq!(rules.len(), 2);
    }
    Ok(())
}
