// SPDX-License-Identifier: AGPL-3.0-only
//! Isolating structural, representation and association fixtures for the clamp format.

use super::*;
use serde_json::{Value, json};
/// Fallible checker control result.
type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

/// Select one valid authored row.
/// # Panics
/// Panics only if the committed corpus or requested fixture id is absent.
fn fixture(id: &str) -> Value {
    let mut value: Value =
        serde_json::from_slice(include_bytes!("../../../../data/clamp/floor.json"))
            .expect("authored JSON");
    let row = value["entries"]
        .as_array()
        .expect("entries")
        .iter()
        .find(|row| row["id"] == id)
        .expect("fixture id")
        .clone();
    value["entries"] = json!([row]);
    value
}

/// Typed failures demonstrate which guard each one-member mutation reaches.
/// # Panics
/// Panics only if an isolating fixture reaches another cause or admits.
/// ´claim:docgen:clamp-golden-refusals´
/// ´test:unit:clamp-golden-refusals´
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "One table keeps each isolated fixture beside its typed refusal assertion."
)]
fn clamp_golden_refusals() -> TestResult {
    let cases = [
        ("/namespace", json!("other"), Cause::Namespace),
        ("/version", json!([1, 0, 0]), Cause::Version),
        ("/version", json!([2, 0, 1]), Cause::Version),
        ("/version", json!([2, 1, 0]), Cause::Version),
        ("/version", json!([3, 0, 0]), Cause::Version),
        ("/owner", json!("other"), Cause::Envelope),
        ("/license", json!("other"), Cause::Envelope),
        ("/sources", json!([]), Cause::Envelope),
        ("/labels", json!([]), Cause::Envelope),
        ("/routes/0/id", json!(""), Cause::Routes),
        ("/routes/1/id", json!("cas"), Cause::Routes),
        ("/routes/0/kind", json!("other"), Cause::Routes),
        ("/routes/1/kind", json!("other"), Cause::Routes),
        ("/routes/0/revision", json!(""), Cause::Routes),
        ("/routes/0/argument", json!(""), Cause::Routes),
        ("/entries", json!([]), Cause::Entries),
        ("/entries/0/id", json!(""), Cause::Identity),
        (
            "/entries/0/input/r",
            json!("bff0000000000000"),
            Cause::Radius,
        ),
        (
            "/entries/0/wall/t",
            json!("bff0000000000000"),
            Cause::Radius,
        ),
        (
            "/entries/0/wall/c",
            json!("7ff0000000000000"),
            Cause::NonFinite,
        ),
        (
            "/entries/0/wall/c",
            json!("3FB999999999999A"),
            Cause::BitsCase,
        ),
        (
            "/entries/0/wall/c",
            json!("3fb99999999999a"),
            Cause::BitsLength,
        ),
        (
            "/entries/0/wall/c",
            json!("zzzzzzzzzzzzzzzz"),
            Cause::BitsCase,
        ),
        ("/entries/0/wall/a", json!("01/10"), Cause::Rational),
        ("/entries/0/expected", json!("other"), Cause::Expected),
        ("/entries/0/golden/image/0", json!("0"), Cause::Golden),
        ("/entries/0/golden/image/1", json!("0"), Cause::Golden),
        ("/entries/0/golden/nominal/0", json!("0"), Cause::Golden),
        ("/entries/0/golden/nominal/1", json!("0"), Cause::Golden),
        ("/entries/0/golden/center", json!("0"), Cause::Golden),
        ("/entries/0/golden/error", json!("0"), Cause::Golden),
        (
            "/entries/0/evidence/0/route",
            json!("other"),
            Cause::Association,
        ),
        (
            "/entries/0/evidence/1/route",
            json!("other"),
            Cause::Association,
        ),
        (
            "/entries/0/evidence/0/input/x",
            json!("3ff0000000000000"),
            Cause::Association,
        ),
        (
            "/entries/0/evidence/0/input/r",
            json!("0000000000000000"),
            Cause::Association,
        ),
        (
            "/entries/0/evidence/0/wall/a",
            json!("0"),
            Cause::Association,
        ),
        (
            "/entries/0/evidence/0/wall/c",
            json!("0000000000000000"),
            Cause::Association,
        ),
        (
            "/entries/0/evidence/0/wall/t",
            json!("0000000000000000"),
            Cause::Association,
        ),
        (
            "/entries/0/evidence/0/golden",
            Value::Null,
            Cause::Association,
        ),
    ];
    for (path, replacement, cause) in cases {
        let mut value = fixture("tenth-cross");
        *value.pointer_mut(path).ok_or("fixture pointer")? = replacement;
        let result = read(serde_json::to_vec(&value)?.as_slice());
        assert!(
            matches!(&result,Err(Error::Invalid(got)) if *got==cause),
            "{path}: expected {cause:?}, got {result:?}"
        );
    }
    for (field, cause) in [("c", Cause::CenterZero), ("t", Cause::TotalZero)] {
        let mut value = fixture("zero-point");
        value["entries"][0]["wall"][field] = json!("8000000000000000");
        assert!(
            matches!(read(serde_json::to_vec(&value)?.as_slice()),Err(Error::Invalid(got)) if got==cause),
            "signed zero {field}"
        );
    }
    for path in [
        "",
        "/routes/0",
        "/entries/0",
        "/entries/0/input",
        "/entries/0/wall",
        "/entries/0/golden",
        "/entries/0/evidence/0",
    ] {
        let mut value = fixture("tenth-cross");
        value
            .pointer_mut(path)
            .ok_or("object pointer")?
            .as_object_mut()
            .ok_or("object")?
            .insert("unknown".into(), json!(0));
        assert!(
            matches!(
                read(serde_json::to_vec(&value)?.as_slice()),
                Err(Error::Invalid(Cause::Json))
            ),
            "unknown member {path}"
        );
    }
    for path in [
        "/routes",
        "/entries/0/evidence",
        "/entries/0/golden/image",
        "/entries/0/golden/nominal",
    ] {
        for length in [1, 3] {
            let mut value = fixture("tenth-cross");
            let array = value
                .pointer_mut(path)
                .ok_or("array pointer")?
                .as_array_mut()
                .ok_or("array")?;
            array.resize(length, array[0].clone());
            assert!(
                matches!(
                    read(serde_json::to_vec(&value)?.as_slice()),
                    Err(Error::Invalid(Cause::Json))
                ),
                "count {path} {length}"
            );
        }
    }
    for path in ["/entries/0", "/entries/0/evidence/0"] {
        let mut value = fixture("wrong-center");
        value
            .pointer_mut(path)
            .ok_or("missing golden object")?
            .as_object_mut()
            .ok_or("object")?
            .remove("golden");
        assert!(
            matches!(
                read(serde_json::to_vec(&value)?.as_slice()),
                Err(Error::Invalid(Cause::Json))
            ),
            "missing required null {path}"
        );
    }
    let mut repeated = fixture("zero-point");
    let row = repeated["entries"][0].clone();
    repeated["entries"] = json!([row.clone(), row]);
    assert!(
        matches!(
            read(serde_json::to_vec(&repeated)?.as_slice()),
            Err(Error::Invalid(Cause::Identity))
        ),
        "duplicate id"
    );
    let mut oversized = fixture("zero-point");
    oversized["entries"] = json!(vec![oversized["entries"][0].clone(); 257]);
    assert!(
        matches!(
            read(serde_json::to_vec(&oversized)?.as_slice()),
            Err(Error::Invalid(Cause::Entries))
        ),
        "entry ceiling"
    );
    let mut at_bound = fixture("zero-point");
    let prototype = at_bound["entries"][0].clone();
    at_bound["entries"] = json!(
        (0..256)
            .map(|index| {
                let mut row = prototype.clone();
                row["id"] = json!(index.to_string());
                row
            })
            .collect::<Vec<_>>()
    );
    assert!(
        read(serde_json::to_vec(&at_bound)?.as_slice()).is_ok(),
        "entry exact ceiling"
    );
    assert!(
        matches!(
            read(b"{\"namespace\":\"a\",\"namespace\":\"b\"}".as_slice()),
            Err(Error::Invalid(Cause::Json))
        ),
        "duplicate key"
    );
    assert!(
        matches!(
            read(vec![b' '; usize::try_from(BYTES)? + 1].as_slice()),
            Err(Error::Invalid(Cause::Bytes))
        ),
        "byte ceiling"
    );
    assert!(
        matches!(read(b"false".as_slice()), Err(Error::Invalid(Cause::Json))),
        "wrong JSON shape"
    );
    Ok(())
}

/// Exact literal capacity and its charged Work prefix have independent boundaries.
/// # Panics
/// Panics only if a literal boundary or charged prefix changes.
/// ´claim:docgen:clamp-golden-literal´
/// ´test:unit:clamp-golden-literal´
#[test]
fn clamp_golden_literal() -> TestResult {
    let exact = "9".repeat(LITERAL);
    assert!(
        rational(&exact, &mut ledger()?).is_ok(),
        "literal exact bound"
    );
    assert!(
        matches!(
            rational(&(exact + "9"), &mut ledger()?),
            Err(Error::Invalid(Cause::Literal))
        ),
        "literal one past"
    );
    let mut b = Budget::new(Limits {
        work: 0,
        ..ledger()?.limits().clone()
    })?;
    assert!(
        matches!(rational("1", &mut b), Err(Error::Arithmetic(_))),
        "work refusal"
    );
    assert_eq!(b.usage().work, 0, "uncharged failed reservation");
    let mut charged = ledger()?;
    charged.work(17, nrz_cas::Location::new("test prefix"))?;
    rational("1/10", &mut charged)?;
    let mut baseline = ledger()?;
    Rational::parse("1/10", &mut baseline)?.exact_string(&mut baseline)?;
    assert_eq!(
        charged.usage().work,
        baseline.usage().work + 21,
        "literal tariff and prefix"
    );
    let mut value = fixture("tenth-cross");
    value["entries"][0]["wall"]["a"] = json!("9".repeat(LITERAL + 1));
    assert!(
        matches!(
            read(serde_json::to_vec(&value)?.as_slice()),
            Err(Error::Invalid(Cause::Literal))
        ),
        "wall literal before arithmetic"
    );
    Ok(())
}

/// Input failures and the documentation entry point preserve their scopes.
/// # Panics
/// Panics only if byte admission or pipeline failure scope changes.
/// ´claim:docgen:clamp-golden-input´
/// ´test:unit:clamp-golden-input´
#[test]
fn clamp_golden_input() -> TestResult {
    /// Reader that fails before supplying any bytes.
    struct Broken;
    impl Read for Broken {
        fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
            Err(std::io::Error::other("fixture"))
        }
    }
    assert!(
        matches!(read(Broken), Err(Error::Invalid(Cause::Io))),
        "I/O cause"
    );
    let mut exact = serde_json::to_vec(&fixture("zero-point"))?;
    exact.resize(usize::try_from(BYTES)?, b' ');
    assert!(read(exact.as_slice()).is_ok(), "exact byte ceiling");
    let root = tempfile::tempdir()?;
    assert!(check_input(root.path()).is_ok(), "reduced fixture root");
    std::fs::create_dir_all(root.path().join("data/clamp"))?;
    std::fs::write(root.path().join(PATH), b"false")?;
    assert!(
        matches!(check_input(root.path()),Err(crate::Error::Payload{path,..}) if path==PATH),
        "pipeline rejects bad clamp input"
    );
    Ok(())
}

/// Isolate the predecessor equality in the least-total admission check.
/// # Panics
/// Panics only if a positive total at an exact zero wall is not refused as nonminimal.
fn least_total_equality() -> TestResult {
    let mut value = fixture("zero-point");
    value["entries"][0]["wall"]["t"] = json!("0000000000000001");
    let row: Row = serde_json::from_value(value["entries"][0].clone())?;
    assert!(
        matches!(
            outcome(&row, &mut ledger()?),
            Ok(("ClampWallTotalMismatch", None))
        ),
        "over-wide total with predecessor equal to distance"
    );
    Ok(())
}

/// All authored unsorted rows are admitted by the Rust checker in the gate.
/// # Panics
/// Panics only if the corpus or row ordering checks fail.
/// ´claim:docgen:clamp-golden-values´
/// ´test:unit:clamp-golden-values´
#[test]
fn clamp_golden_values() -> TestResult {
    least_total_equality()?;
    assert_eq!(
        rounded_neighbor(MAX, false),
        None,
        "opposite upper neighbour"
    );
    assert_eq!(
        rounded_neighbor(SIGN | MAX, true),
        None,
        "opposite lower neighbour"
    );
    let mut allowance = ledger()?;
    let largest = dyadic(MAX, &mut allowance)?;
    assert_eq!(
        nearest(&largest, &mut allowance)?,
        Some(MAX),
        "nearest largest finite value"
    );
    let negative_largest = Rational::zero().sub(&largest, &mut allowance)?;
    assert_eq!(
        nearest(&negative_largest, &mut allowance)?,
        Some(SIGN | MAX),
        "nearest negative largest value"
    );
    let beyond = Rational::from_ratio(2, 1, &mut allowance)?.pow(1024, &mut allowance)?;
    assert_eq!(
        nearest(&beyond, &mut allowance)?,
        None,
        "nearest finite range"
    );
    let admitted = read(include_bytes!("../../../../data/clamp/floor.json").as_slice());
    assert!(admitted.is_ok(), "authored golden admission: {admitted:?}");
    let payload = admitted?;
    let isolated = payload
        .entries
        .iter()
        .find(|row| row.id == "wrong-center")
        .ok_or("wall fixture")?;
    let mut witness = ledger()?;
    let center = dyadic(bits(&isolated.wall.c)?, &mut witness)?;
    let total_bits = bits(&isolated.wall.t)?;
    let distance = abs(
        center.sub(&rational(&isolated.wall.a, &mut witness)?, &mut witness)?,
        &mut witness,
    )?;
    assert!(
        dyadic(total_bits, &mut witness)?.compare(&distance, &mut witness)? != Ordering::Less,
        "non-nearest wall still enclosed"
    );
    assert!(
        dyadic(total_bits - 1, &mut witness)?.compare(&distance, &mut witness)? == Ordering::Less,
        "non-nearest fixture has least covering total"
    );
    assert_eq!(payload.entries.len(), 34, "row count");
    assert!(
        payload
            .entries
            .windows(2)
            .any(|rows| rows[0].id > rows[1].id),
        "unsorted rows"
    );
    Ok(())
}

/// Isolate the bits-long refusal or precedence.
/// # Panics
/// Panics only if this fixture reaches another outcome.
/// ´claim:docgen:clamp-golden-bits-long´
/// ´test:unit:clamp-golden-bits-long´
#[test]
fn clamp_golden_bits_long() -> TestResult {
    let mut value = fixture("tenth-cross");
    value["entries"][0]["wall"]["c"] = json!("03fb999999999999a");
    assert!(
        matches!(
            read(serde_json::to_vec(&value)?.as_slice()),
            Err(Error::Invalid(Cause::BitsLength))
        ),
        "bits-long typed refusal"
    );
    Ok(())
}

/// Isolate the nan refusal or precedence.
/// # Panics
/// Panics only if this fixture reaches another outcome.
/// ´claim:docgen:clamp-golden-nan´
/// ´test:unit:clamp-golden-nan´
#[test]
fn clamp_golden_nan() -> TestResult {
    let mut value = fixture("tenth-cross");
    value["entries"][0]["wall"]["c"] = json!("7ff8000000000000");
    assert!(
        matches!(
            read(serde_json::to_vec(&value)?.as_slice()),
            Err(Error::Invalid(Cause::NonFinite))
        ),
        "nan typed refusal"
    );
    Ok(())
}

/// Isolate the negative-infinity refusal or precedence.
/// # Panics
/// Panics only if this fixture reaches another outcome.
/// ´claim:docgen:clamp-golden-negative-infinity´
/// ´test:unit:clamp-golden-negative-infinity´
#[test]
fn clamp_golden_negative_infinity() -> TestResult {
    let mut value = fixture("tenth-cross");
    value["entries"][0]["wall"]["c"] = json!("fff0000000000000");
    assert!(
        matches!(
            read(serde_json::to_vec(&value)?.as_slice()),
            Err(Error::Invalid(Cause::NonFinite))
        ),
        "negative-infinity typed refusal"
    );
    Ok(())
}

/// Isolate the golden-rational refusal or precedence.
/// # Panics
/// Panics only if this fixture reaches another outcome.
/// ´claim:docgen:clamp-golden-rational´
/// ´test:unit:clamp-golden-rational´
#[test]
fn clamp_golden_rational() -> TestResult {
    let mut value = fixture("tenth-cross");
    value["entries"][0]["golden"]["image"][0] = json!("00");
    assert!(
        matches!(
            read(serde_json::to_vec(&value)?.as_slice()),
            Err(Error::Invalid(Cause::Rational))
        ),
        "golden-rational typed refusal"
    );
    Ok(())
}

/// Isolate the golden-tariff refusal or precedence.
/// # Panics
/// Panics only if this fixture reaches another outcome.
/// ´claim:docgen:clamp-golden-tariff´
/// ´test:unit:clamp-golden-tariff´
#[test]
fn clamp_golden_tariff() -> TestResult {
    let mut value = fixture("tenth-cross");
    value["entries"][0]["golden"]["image"][0] = json!("9".repeat(LITERAL + 1));
    assert!(
        matches!(
            read(serde_json::to_vec(&value)?.as_slice()),
            Err(Error::Invalid(Cause::Literal))
        ),
        "golden-tariff typed refusal"
    );
    Ok(())
}

/// Isolate the center-precedence refusal or precedence.
/// # Panics
/// Panics only if this fixture reaches another outcome.
/// ´claim:docgen:clamp-golden-center-precedence´
/// ´test:unit:clamp-golden-center-precedence´
#[test]
fn clamp_golden_center_precedence() -> TestResult {
    let mut value = fixture("wrong-center");
    value["entries"][0]["wall"]["t"] = json!("0000000000000000");
    let row: Row = serde_json::from_value(value["entries"][0].clone())?;
    assert!(
        matches!(
            outcome(&row, &mut ledger()?),
            Ok(("ClampWallCenterMismatch", None))
        ),
        "center precedes failed enclosure"
    );
    Ok(())
}

/// Isolate the wall-precedence refusal or precedence.
/// # Panics
/// Panics only if this fixture reaches another outcome.
/// ´claim:docgen:clamp-golden-wall-precedence´
/// ´test:unit:clamp-golden-wall-precedence´
#[test]
fn clamp_golden_wall_precedence() -> TestResult {
    let mut value = fixture("wrong-center");
    value["entries"][0]["input"]["x"] = json!("7fefffffffffffff");
    value["entries"][0]["input"]["r"] = json!("0000000000000000");
    let row: Row = serde_json::from_value(value["entries"][0].clone())?;
    assert!(
        matches!(
            outcome(&row, &mut ledger()?),
            Ok(("ClampWallCenterMismatch", None))
        ),
        "wall precedes unbounded-image outcome"
    );
    Ok(())
}

/// Measure complete row admission using the same arithmetic path as the reader.
/// # Panics
/// Panics only if the retained complete-row cost profile changes.
/// ´claim:docgen:clamp-golden-costs´
/// ´test:unit:clamp-golden-costs´
#[test]
fn clamp_golden_costs() -> TestResult {
    use std::io::Write as _;
    let payload = read(include_bytes!("../../../../data/clamp/floor.json").as_slice())?;
    let mut maxima = [0; 6];
    for row in &payload.entries {
        let mut b = ledger()?;
        check_row(row, &payload.routes, &mut b)?;
        let mut costs = [b.usage().work, b.usage().arithmetic_steps, 0, 0, 0, 0];
        for index in 2..6 {
            let mut lower = u64::from(index == 3);
            let mut upper = [0, 0, 1024, 4096, 8192, 4096][index];
            while lower < upper {
                let middle = lower + (upper - lower) / 2;
                let mut limits = ledger()?.limits().clone();
                match index {
                    2 => limits.literal_bytes = middle,
                    3 => limits.component_bits = middle,
                    4 => {
                        limits.component_bits = costs[3];
                        limits.intermediate_bits = middle.max(costs[3]);
                    }
                    _ => limits.integer_exponent = middle,
                }
                if check_row(row, &payload.routes, &mut Budget::new(limits)?).is_ok() {
                    upper = middle;
                } else {
                    lower = middle + 1;
                }
            }
            costs[index] = if index == 4 {
                lower.max(costs[3])
            } else {
                lower
            };
        }
        for (peak, cost) in maxima.iter_mut().zip(costs) {
            *peak = (*peak).max(cost);
        }
        writeln!(std::io::stdout(), "CLAMP COST {} {costs:?}", row.id)?;
    }
    writeln!(std::io::stdout(), "CLAMP MAXIMA {maxima:?}")?;
    assert_eq!(
        maxima,
        [22_561_972, 61_629, 341, 1076, 1106, 1074],
        "complete row profile"
    );
    Ok(())
}
