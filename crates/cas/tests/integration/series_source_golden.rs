// SPDX-License-Identifier: AGPL-3.0-only
//! Source records are authored from the wire grammar, independently of export and production.

use super::series_golden::{
    Part, Record, bytes, count, encode_records, expression, join, name, number, object, records,
};
use crate::testing::common::{TestResult, default_budget};
use crate::testing::series_source::{setup, statement};

/// Hand dependency DAG includes ordinary coordinate definedness and registered trig totality.
pub(super) fn golden(kind: usize) -> TestResult<Vec<u8>> {
    let (values, manifest) = source_records(kind, false)?;
    encode_records(&values, manifest)
}

/// Full order-four companion records also support the independent mixed arithmetic goldens.
#[expect(
    clippy::too_many_lines,
    reason = "The hand grammar retains source laws and their ordinary closure together"
)]
fn source_records(kind: usize, full: bool) -> TestResult<(Vec<Record>, Vec<u8>)> {
    let seed = kind == 3;
    let key = if seed {
        "series.truncate"
    } else {
        "series.source"
    };
    let (mode, form, law) = if seed {
        (0, 2, 0)
    } else {
        (10, 0, u8::try_from(kind)? + 3)
    };
    let manifest = if seed {
        join(&[count(2), name("case.domain"), count(1), name(key), count(1)])
    } else {
        join(&[
            count(3),
            name("case.domain"),
            count(1),
            name(key),
            count(1),
            name("trig.defined"),
            count(1),
        ])
    };
    let mut values = records();
    values.retain(|r| {
        matches!(
            r.name,
            "real"
                | "complex"
                | "header"
                | "ambient"
                | "singleton-meet"
                | "intersection"
                | "context"
                | "Q"
                | "x"
                | "zero"
                | "ordinary-statement"
                | "ordinary-proof"
        ) || (matches!(kind, 0 | 1 | 3) && r.name == "one")
    });
    values.push(Record {
        name: "defined",
        tag: 7,
        layer: 0,
        parts: vec![
            Part::Ref("header"),
            bytes(join(&[count(0), vec![0], count(1)])),
            Part::Ref("x"),
        ],
    });
    if !seed {
        values.push(Record {
            name: "source",
            tag: 3,
            layer: 1,
            parts: vec![
                Part::Ref("header"),
                bytes(join(&[count(0), vec![1]])),
                Part::Ref("real"),
                bytes(join(&[
                    vec![0, 1, 1, 58 + u8::try_from(kind)?, 0],
                    count(1),
                ])),
                Part::Ref("x"),
                bytes(join(&[vec![0, 0], count(0)])),
            ],
        });
        values.push(Record {
            name: "source-defined",
            tag: 7,
            layer: 0,
            parts: vec![
                Part::Ref("header"),
                bytes(join(&[count(0), vec![0], count(1)])),
                Part::Ref("source"),
            ],
        });
        values.push(Record {
            name: "trig-statement",
            tag: 10,
            layer: 0,
            parts: vec![
                Part::Ref("context"),
                bytes(vec![1]),
                Part::Ref("ambient"),
                Part::Ref("source-defined"),
            ],
        });
        values.push(Record {
            name: "trig-proof",
            tag: 11,
            layer: 0,
            parts: vec![
                bytes(count(2)),
                Part::Ref("trig-statement"),
                bytes(count(1)),
                Part::Ref("ordinary-proof"),
                bytes(join(&[count(0), vec![0]])),
            ],
        });
    }
    if kind == 0 {
        values.push(expression(
            "minus-sixth",
            join(&[vec![0], number(true, 1, 6)]),
        ));
    }
    if kind == 1 && full {
        values.push(expression("half", join(&[vec![0], number(true, 1, 2)])));
    }
    let coefficients: &[&str] = match kind {
        0 => &["zero", "one", "zero", "minus-sixth"],
        1 if full => &["one", "zero", "half", "zero"],
        1 => &["one"],
        2 => &["zero"],
        _ => &["zero", "one"],
    };
    values.push(object("output", coefficients));
    values.push(Record {
        name: "series-statement",
        tag: 39,
        layer: 0,
        parts: vec![
            bytes(join(&[count(1), name(key), count(1)])),
            Part::Ref("context"),
            bytes(vec![mode, form, law]),
            Part::Ref(if seed { "x" } else { "source" }),
            Part::Ref("ambient"),
            bytes(vec![2]),
            Part::Ref("output"),
            bytes(join(&[count(0), count(0), manifest.clone()])),
        ],
    });
    let mut rows = vec![mode, form, law];
    let primary = if kind == 0 || (kind == 1 && full) {
        vec![(false, 0, 1), (false, 1, 1), (false, 0, 1), (true, 1, 6)]
    } else if seed {
        vec![(false, 0, 1), (false, 1, 1)]
    } else {
        vec![(false, 0, 1)]
    };
    rows.extend(count(u32::try_from(primary.len())?));
    for (s, n, d) in primary {
        rows.extend(number(s, n, d));
    }
    let companion = if kind == 0 || (kind == 1 && full) {
        vec![(false, 1, 1), (false, 0, 1), (true, 1, 2), (false, 0, 1)]
    } else if kind == 1 {
        vec![(false, 1, 1)]
    } else {
        Vec::new()
    };
    rows.extend(count(u32::try_from(companion.len())?));
    for (s, n, d) in companion {
        rows.extend(number(s, n, d));
    }
    rows.extend(count(if kind == 0 || full { 3 } else { 0 }));
    if kind == 0 || full {
        for i in 0..3_u8 {
            rows.extend(count(u32::from(i)));
            rows.extend(number(false, i + 1, 1));
            rows.extend(number(false, 1, i + 1));
        }
    }
    let guard = if seed { "ordinary-proof" } else { "trig-proof" };
    values.push(Record {
        name: "series-certificate",
        tag: 40,
        layer: 0,
        parts: vec![
            Part::Ref("series-statement"),
            bytes(join(&[vec![mode], count(1)])),
            Part::Ref(guard),
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
            Part::Ref(guard),
            bytes(manifest.clone()),
        ],
    });
    Ok((values, manifest))
}

/// Independent origin and seed bytes replay in both allocation orders; export is only a control.
/// ´test:integration:series-source-independent-goldens´
#[test]
fn series_source_independent_goldens() -> TestResult {
    for kind in 0..4 {
        let bytes = golden(kind)?;
        for reordered in [false, true] {
            let mut f = if kind == 3 {
                crate::testing::series_source::linear_seed_receiver(reordered)?
            } else {
                crate::testing::series_source::receiver(
                    kind,
                    if kind == 0 { 4 } else { 1 },
                    reordered,
                )?
            };
            let expected = f.expected.as_ref().ok_or("expected")?;
            f.base
                .cas
                .replay(&bytes, expected, &mut default_budget()?)?;
            let mut sender = if kind == 3 {
                crate::testing::series_source::linear_seed(reordered)?
            } else {
                setup(kind, if kind == 0 { 4 } else { 1 }, reordered, true)?
            };
            let expected = sender.expected.as_ref().ok_or("expected")?;
            let supplied = sender.base.cas.check_series_source(
                statement(expected)?,
                &sender.certificate,
                &sender.proofs,
                &mut default_budget()?,
            )?;
            assert_eq!(
                sender
                    .base
                    .cas
                    .export_witness(&supplied, &mut default_budget()?)?,
                bytes
            );
        }
    }
    Ok(())
}

/// Rename only semantic roles in the independently authored source DAG.
fn rename(key: &'static str, cosine: bool) -> &'static str {
    match (key, cosine) {
        ("source", false) => "sine",
        ("source", true) => "cosine",
        ("source-defined", false) => "sine-defined",
        ("source-defined", true) => "cosine-defined",
        ("trig-statement", false) => "sine-totality-statement",
        ("trig-statement", true) => "cosine-totality-statement",
        ("trig-proof", false) => "sine-totality",
        ("trig-proof", true) => "cosine-totality",
        ("output", false) => "sine-object",
        ("output", true) => "cosine-object",
        ("series-statement", false) => "sine-statement",
        ("series-statement", true) => "cosine-statement",
        ("series-certificate", false) => "sine-certificate",
        ("series-certificate", true) => "cosine-certificate",
        ("conclusion", false) => "sine-proof",
        ("conclusion", true) => "cosine-proof",
        _ => key,
    }
}

/// One independent ordered arithmetic DAG; the product deliberately repeats the sine proof.
#[expect(
    clippy::too_many_lines,
    reason = "Every mixed record and repeated ordered reference is hand-authored"
)]
pub(super) fn lifted_golden(product: bool) -> TestResult<Vec<u8>> {
    let key = if product { "series.mul" } else { "series.add" };
    let mode = if product { 3 } else { 1 };
    let manifest = join(&[
        count(5),
        name("case.domain"),
        count(1),
        name("ctx.hypothesis"),
        count(1),
        name(key),
        count(1),
        name("series.source"),
        count(1),
        name("trig.defined"),
        count(1),
    ]);
    let object_closure = join(&[count(1), name(key), count(1)]);
    let mut values: Vec<Record> = Vec::new();
    for cosine in [false, true] {
        if cosine && product {
            continue;
        }
        let (origin, _) = source_records(usize::from(cosine), true)?;
        for mut record in origin {
            if record.name == "series-statement" {
                record.parts[0] = bytes(join(&[count(3), name("series.source"), count(1)]));
            }
            if record.name == "trig-proof" {
                record.parts[0] = bytes(count(4));
            }
            record.name = rename(record.name, cosine);
            for part in &mut record.parts {
                if let Part::Ref(key) = part {
                    *key = rename(key, cosine);
                }
            }
            if !values.iter().any(|prior| prior.name == record.name) {
                values.push(record);
            }
        }
    }
    let context = values
        .iter_mut()
        .find(|r| r.name == "context")
        .ok_or("context")?;
    context.parts = vec![
        Part::Ref("header"),
        bytes(vec![0]),
        Part::Ref("ambient"),
        bytes(join(&[
            vec![0],
            count(1),
            name("original-composite-defined"),
        ])),
        Part::Ref("composite-defined"),
    ];
    values.push(Record {
        name: "composite",
        tag: 3,
        layer: 1,
        parts: vec![
            Part::Ref("header"),
            bytes(join(&[count(0), vec![1]])),
            Part::Ref("real"),
            bytes(join(&[
                vec![0, 1, 1, if product { 7 } else { 5 }, 0],
                count(2),
            ])),
            Part::Ref("sine"),
            Part::Ref(if product { "sine" } else { "cosine" }),
            bytes(join(&[vec![0, 0], count(0)])),
        ],
    });
    values.push(Record {
        name: "composite-defined",
        tag: 7,
        layer: 0,
        parts: vec![
            Part::Ref("header"),
            bytes(join(&[count(0), vec![0], count(1)])),
            Part::Ref("composite"),
        ],
    });
    values.push(Record {
        name: "domain-statement",
        tag: 10,
        layer: 0,
        parts: vec![
            Part::Ref("context"),
            bytes(vec![1]),
            Part::Ref("ambient"),
            Part::Ref("composite-defined"),
        ],
    });
    values.push(Record {
        name: "domain-proof",
        tag: 11,
        layer: 0,
        parts: vec![
            bytes(count(1)),
            Part::Ref("domain-statement"),
            bytes(join(&[count(0), count(0), vec![2]])),
            Part::Ref("context"),
            bytes(0_u64.to_le_bytes().to_vec()),
        ],
    });
    values.push(object(
        "object-output",
        if product {
            &["zero", "zero", "one", "zero"]
        } else {
            &["one", "one", "half", "minus-sixth"]
        },
    ));
    let right_object = if product {
        "sine-object"
    } else {
        "cosine-object"
    };
    values.push(Record {
        name: "object-statement",
        tag: 39,
        layer: 0,
        parts: vec![
            bytes(join(&[count(2), name(key), count(1)])),
            Part::Ref("context"),
            bytes(vec![mode, 0]),
            Part::Ref("sine-object"),
            Part::Ref(right_object),
            bytes(vec![0]),
            Part::Ref("object-output"),
            bytes(join(&[
                count(4),
                vec![0],
                count(0),
                count(0),
                object_closure.clone(),
            ])),
        ],
    });
    let mut rows = join(&[
        vec![mode, 0],
        count(4),
        vec![0],
        count(if product { 10 } else { 4 }),
    ]);
    let sine = [(false, 0, 1), (false, 1, 1), (false, 0, 1), (true, 1, 6)];
    if product {
        for degree in 0..4_u32 {
            for i in 0..=degree {
                let j = degree - i;
                rows.extend(count(degree));
                rows.extend(count(i));
                rows.extend(count(j));
                for (s, n, d) in [
                    sine[usize::try_from(i)?],
                    sine[usize::try_from(j)?],
                    (false, u8::from(i == 1 && j == 1), 1),
                    (false, u8::from(degree == 2 && i >= 1), 1),
                ] {
                    rows.extend(number(s, n, d));
                }
            }
        }
    } else {
        let cosine = [(false, 1, 1), (false, 0, 1), (true, 1, 2), (false, 0, 1)];
        let sum = [(false, 1, 1), (false, 1, 1), (true, 1, 2), (true, 1, 6)];
        for (index, ((a, b), c)) in sine.into_iter().zip(cosine).zip(sum).enumerate() {
            rows.extend(count(u32::try_from(index)?));
            for (s, n, d) in [a, b, c] {
                rows.extend(number(s, n, d));
            }
        }
    }
    values.push(Record {
        name: "object-certificate",
        tag: 40,
        layer: 0,
        parts: vec![
            Part::Ref("object-statement"),
            bytes(join(&[vec![mode], count(0), rows])),
        ],
    });
    values.push(Record {
        name: "object-proof",
        tag: 41,
        layer: 0,
        parts: vec![
            Part::Ref("object-statement"),
            Part::Ref("object-certificate"),
            bytes(join(&[count(0), object_closure])),
        ],
    });
    values.push(Record {
        name: "lifted-statement",
        tag: 39,
        layer: 0,
        parts: vec![
            bytes(join(&[count(2), name(key), count(1)])),
            Part::Ref("context"),
            bytes(vec![mode, 1, 0]),
            Part::Ref("composite"),
            Part::Ref("ambient"),
            bytes(vec![2]),
            Part::Ref("object-output"),
            bytes(count(1)),
            Part::Ref("context"),
            bytes(join(&[count(0), count(0), manifest.clone()])),
        ],
    });
    let right_proof = if product {
        "sine-proof"
    } else {
        "cosine-proof"
    };
    values.push(Record {
        name: "lifted-certificate",
        tag: 40,
        layer: 0,
        parts: vec![
            Part::Ref("lifted-statement"),
            bytes(join(&[vec![mode], count(4)])),
            Part::Ref("sine-proof"),
            Part::Ref(right_proof),
            Part::Ref("object-proof"),
            Part::Ref("domain-proof"),
            bytes(join(&[vec![mode, 1, 0], count(0), count(0), count(0)])),
        ],
    });
    values.push(Record {
        name: "conclusion",
        tag: 41,
        layer: 0,
        parts: vec![
            Part::Ref("lifted-statement"),
            Part::Ref("lifted-certificate"),
            bytes(count(4)),
            Part::Ref("sine-proof"),
            Part::Ref(right_proof),
            Part::Ref("object-proof"),
            Part::Ref("domain-proof"),
            bytes(manifest.clone()),
        ],
    });
    encode_records(&values, manifest)
}

/// The sum and repeated-source product preserve all ordinary and series references in hand bytes.
/// ´test:integration:series-source-lifted-independent-goldens´
#[test]
fn series_source_lifted_independent_goldens() -> TestResult {
    for (product, kind) in [(false, 1), (true, 7)] {
        let bytes = lifted_golden(product)?;
        for reordered in [false, true] {
            let mut f = crate::testing::series_source::receiver(kind + 4, 4, reordered)?;
            let expected = f.expected.as_ref().ok_or("expected")?;
            let witness = f
                .base
                .cas
                .replay(&bytes, expected, &mut default_budget()?)?;
            assert_eq!(witness.hypotheses().len(), 1);
            let mut sender = crate::testing::series_source::lifted(kind, reordered)?;
            let expected = sender.expected.as_ref().ok_or("expected")?;
            let checked = sender.base.cas.check_series_source(
                statement(expected)?,
                &sender.certificate,
                &sender.proofs,
                &mut default_budget()?,
            )?;
            assert_eq!(
                sender
                    .base
                    .cas
                    .export_witness(&checked, &mut default_budget()?)?,
                bytes
            );
        }
    }
    Ok(())
}
