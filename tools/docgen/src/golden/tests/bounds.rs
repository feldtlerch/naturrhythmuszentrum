// SPDX-License-Identifier: AGPL-3.0-only
//! Preallocation, exact resource cause, exponent and measured-profile controls.

use super::{Cause, Error, Limits, Value, budget, fixture, json, read, rejects};
use crate::golden::{Payload, json::Audit};
use nrz_cas::limits::BudgetKind;
use nrz_cas::{Exhaustion, FaultKind};

/// Require the exact original Work fault, including the attempted charge.
/// # Panics
/// Panics only when a control loses or changes its original resource cause.
pub(super) fn work_refusal(result: &Result<Payload, Error>, limit: u64, needed: u64) {
    let Err(Error::Arithmetic(fault)) = result else {
        panic!("expected original arithmetic fault");
    };
    assert_eq!(
        fault.kind,
        FaultKind::Exhausted {
            reason: Exhaustion::Limit,
            budget: BudgetKind::Work,
            limit,
            needed
        }
    );
}

/// Assert a typed rejection before the allocating JSON decoder is entered.
/// # Panics
/// Panics if preflight loses a resource cause or starts allocating the JSON tree.
fn before_tree(bytes: &[u8], limits: Limits, cause: Cause) {
    let mut audit = Audit::default();
    let result = crate::golden::json::parse(bytes, limits, &mut audit);
    assert!(matches!(result, Err(Error::Invalid { cause: actual, .. }) if actual == cause));
    assert_eq!(audit.tree_decodes, 0);
}

/// Oversized entries and literals never reach string or container retention.
/// # Panics
/// Panics if an adversarial fixture is not refused at the preallocation boundary.
/// ´claim:docgen:golden-preallocation-controls´
/// ´test:unit:golden-preallocation-controls´
#[test]
fn golden_preallocation_controls() {
    let bytes = serde_json::to_vec(&fixture()).expect("fixture encoding");
    before_tree(
        &bytes,
        Limits {
            entries: 0,
            ..Limits::default()
        },
        Cause::Entries,
    );
    let mut value = fixture();
    value["entries"][0]["input"]["point"] = json!("7".repeat(500_000));
    let large = serde_json::to_vec(&value).expect("long literal encoding");
    assert!(large.len() < Limits::default().bytes);
    before_tree(&large, Limits::default(), Cause::Literal);
    let small_entries = format!("{{\"entries\":[{}0]}}", "0,".repeat(200_000));
    assert!(small_entries.len() < Limits::default().bytes);
    before_tree(small_entries.as_bytes(), Limits::default(), Cause::Entries);
    before_tree("[".repeat(65).as_bytes(), Limits::default(), Cause::Depth);
    // Escaped schema keys and digits cannot bypass association or decoded-byte limits.
    let escaped = br#"{"entr\u0069es":[{"input":{"po\u0069nt":"\u0031\u0032"}}]}"#;
    before_tree(
        escaped,
        Limits {
            literal_bytes: 1,
            ..Limits::default()
        },
        Cause::Literal,
    );
    let mut audit = Audit::default();
    assert!(
        crate::golden::json::parse(
            escaped,
            Limits {
                literal_bytes: 2,
                ..Limits::default()
            },
            &mut audit
        )
        .is_ok()
    );
    assert_eq!(audit.tree_decodes, 1);
    assert_eq!(audit.inventory.largest_literal, 2);
    // Multibyte metadata and surrogate pairs are valid and have no rational ceiling.
    let unicode = br#"{"label":"\ud834\udd1e","entries":[{"exponent":"\u00e9"}]}"#;
    before_tree(
        unicode,
        Limits {
            literal_bytes: 1,
            ..Limits::default()
        },
        Cause::Literal,
    );
}

/// Closed operation compatibility and exact exponent classes are separate from runtime truth.
/// # Panics
/// Panics if a declared operation/exponent control has the wrong outcome.
/// ´claim:docgen:golden-exponent-controls´
/// ´test:unit:golden-exponent-controls´
#[test]
fn golden_exponent_controls() {
    let mut value = fixture();
    value["entries"][6]["expected"]["refusal"] = json!("non-dyadic-exponent");
    rejects(&value, Cause::Expected);
    let mut wide = super::singleton("power", "1", "1");
    wide["entries"][0]["exponent"] = json!("1/18446744073709551616");
    let bytes = serde_json::to_vec(&wide).expect("wide dyadic encoding");
    assert!(
        read(
            bytes.as_slice(),
            Limits::default(),
            &mut budget(100_000_000)
        )
        .is_ok()
    );
    for exponent in ["1/3", "0", "-1/4", "5/4"] {
        let mut value = fixture();
        value["entries"][5]["exponent"] = json!(exponent);
        rejects(&value, Cause::Exponent);
    }
    for (exponent, reason, admitted) in [
        ("1/3", "non-dyadic-exponent", true),
        ("1/3", "outside-domain", false),
        ("1/3", "below-normal-operand", false),
        ("1/3", "target-not-certifiable", false),
        ("1/4", "non-dyadic-exponent", false),
        ("0", "outside-domain", true),
        ("-1/3", "outside-domain", true),
        ("4/3", "outside-domain", true),
        ("4/3", "non-dyadic-exponent", false),
        ("0", "below-normal-operand", false),
    ] {
        let mut value = fixture();
        let mut row = value["entries"][6].clone();
        row["operation"] = json!("power");
        row["target"] = json!({"kind":"outward"});
        row["exponent"] = json!(exponent);
        row["expected"]["refusal"] = json!(reason);
        value["entries"] = json!([row]);
        if admitted {
            let bytes = serde_json::to_vec(&value).expect("encoding");
            assert!(
                read(
                    bytes.as_slice(),
                    Limits::default(),
                    &mut budget(100_000_000)
                )
                .is_ok()
            );
        } else {
            rejects(&value, Cause::Exponent);
        }
    }
}

/// Measure fresh-ledger Work and rational bytes for each fixture and their complete envelope.
/// # Errors
/// Returns encoding, admission or output errors.
/// # Panics
/// Panics only if the committed sample ceases to have an entry array.
/// ´claim:docgen:golden-profile-measurement´
/// ´test:unit:golden-profile-measurement´
#[test]
fn golden_profile_measurement() -> Result<(), Box<dyn std::error::Error>> {
    use std::io::Write;
    let value = fixture();
    let mut output = std::io::stdout().lock();
    for row in value["entries"].as_array().expect("entries") {
        let mut one = value.clone();
        one["entries"] = json!([row]);
        measure(&one, row["id"].as_str().expect("row id"), &mut output)?;
    }
    measure(&value, "whole", &mut output)?;
    output.flush()?;
    Ok(())
}

/// One complete envelope measurement, including repeated endpoint evidence.
/// # Panics
/// Panics if measured usage drifts or the chosen profile loses its stated margin.
fn measure(
    value: &Value,
    name: &str,
    out: &mut impl std::io::Write,
) -> Result<(), Box<dyn std::error::Error>> {
    let bytes = serde_json::to_vec(value)?;
    let mut ledger = budget(100_000_000);
    drop(read(bytes.as_slice(), Limits::default(), &mut ledger)?);
    let mut audit = Audit::default();
    drop(crate::golden::json::parse(
        &bytes,
        Limits::default(),
        &mut audit,
    )?);
    let expected = match name {
        "E1" => (2_182_192, 304, 1418),
        "M1" => (1_396_079, 309, 1438),
        "M0" | "T0" => (10_255, 1, 8),
        "sigma-ln-two-box" => (3_944_721, 304, 2956),
        "P1" => (2_000_046, 305, 2943),
        "R4" => (290, 2, 2),
        "whole" => (9_543_838, 309, 8773),
        _ => panic!("unmeasured fixture row"),
    };
    assert_eq!(
        (
            ledger.usage().work,
            audit.inventory.largest_literal,
            audit.inventory.literal_bytes
        ),
        expected
    );
    if name == "whole" {
        let limits = crate::golden::checker_limits();
        assert_eq!(limits.work, 2 * 3_944_721);
        assert_eq!(limits.literal_bytes, 2 * expected.1 as u64);
        for cap in [expected.1 as u64, expected.1 as u64 - 1] {
            let mut exact = nrz_cas::limits::Budget::new(nrz_cas::limits::Limits {
                work: 100_000_000,
                literal_bytes: cap,
                ..limits.clone()
            })?;
            let result = read(bytes.as_slice(), Limits::default(), &mut exact);
            if cap == expected.1 as u64 {
                drop(result?);
            } else {
                let Err(Error::Arithmetic(fault)) = result else {
                    panic!("literal resource cause");
                };
                assert_eq!(
                    fault.kind,
                    FaultKind::Exhausted {
                        reason: Exhaustion::Limit,
                        budget: BudgetKind::LiteralBytes,
                        limit: cap,
                        needed: expected.1 as u64
                    }
                );
            }
        }
    }
    writeln!(
        out,
        "PROFILE {name} work={} max_literal={} total_literals={}",
        ledger.usage().work,
        audit.inventory.largest_literal,
        audit.inventory.literal_bytes
    )?;
    Ok(())
}

/// Every admitted entry has its own finite allowance, independent of fixture length.
/// # Errors
/// Returns encoding or validation errors.
/// # Panics
/// Panics if the full entry ceiling fails or the one-below fault changes.
/// ´claim:docgen:golden-entry-profile´
/// ´test:unit:golden-entry-profile´
#[test]
fn golden_entry_profile() -> Result<(), Box<dyn std::error::Error>> {
    let mut value = fixture();
    let row = value["entries"][3].clone();
    let record = crate::golden::Record::new(&row, "$.entries[0]")?;
    let ids: Vec<_> = value["routes"]
        .as_array()
        .expect("routes")
        .iter()
        .map(|route| route["id"].as_str().expect("route id"))
        .collect();
    for prefix in [3_944_721, 3_944_722] {
        let mut exact = nrz_cas::limits::Budget::new(crate::golden::checker_limits())?;
        exact.work(prefix, nrz_cas::Location::new("profile boundary prefix"))?;
        let result = crate::golden::entry(&record, &ids, Limits::default(), &mut exact);
        if prefix == 3_944_721 {
            result?;
            assert_eq!(exact.usage().work, 7_889_442);
        } else {
            let Err(Error::Arithmetic(fault)) = result else {
                panic!("entry ceiling");
            };
            assert_eq!(
                fault.kind,
                FaultKind::Exhausted {
                    reason: Exhaustion::Limit,
                    budget: BudgetKind::Work,
                    limit: 7_889_442,
                    needed: 7_889_443,
                }
            );
        }
    }
    assert_eq!(crate::golden::payload_work(256, 7_889_442)?, 2_019_697_152);
    assert!(
        matches!(crate::golden::payload_work(2, u64::MAX), Err(fault)
        if matches!(fault.kind, FaultKind::Exhausted { reason: Exhaustion::Capacity, budget: BudgetKind::Work, .. }))
    );
    let entries: Vec<_> = (0..Limits::default().entries)
        .map(|index| {
            let mut row = row.clone();
            row["id"] = json!(format!("sigma-{index}"));
            row
        })
        .collect();
    value["entries"] = json!(entries);
    let bytes = serde_json::to_vec(&value)?;
    let limits = Limits::default();
    let value = crate::golden::decode(bytes.as_slice(), limits)?;
    crate::golden::validate_profiled(&value, limits, |_| crate::golden::checker_limits())?;
    let result = crate::golden::validate_profiled(&value, limits, |index| {
        let mut profile = crate::golden::checker_limits();
        if index == limits.entries - 1 {
            profile.work = 3_944_720;
        }
        profile
    });
    let Err(Error::Arithmetic(fault)) = result else {
        panic!("expected per-entry exhaustion");
    };
    assert_eq!(
        fault.kind,
        FaultKind::Exhausted {
            reason: Exhaustion::Limit,
            budget: BudgetKind::Work,
            limit: 3_944_720,
            needed: 3_944_721,
        }
    );
    Ok(())
}
