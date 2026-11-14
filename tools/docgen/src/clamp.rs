// SPDX-License-Identifier: AGPL-3.0-only
//! Value-only floor-clamp golden admission; no native evaluator is exposed.

use nrz_cas::{
    Rational,
    limits::{Budget, Limits},
};
use serde::Deserialize;
use std::{cmp::Ordering, collections::BTreeSet, io::Read, path::Path};

#[cfg(test)]
mod cas_route;
#[cfg(test)]
mod tests;

/// Authored corpus path, independent of prepared datasets.
const PATH: &str = "data/clamp/floor.json";
/// Largest finite positive binary64 bit pattern.
const MAX: u64 = 0x7fef_ffff_ffff_ffff;
/// Binary64 sign mask.
const SIGN: u64 = 1 << 63;
/// Envelope byte ceiling, checked before deserialization.
const BYTES: u64 = 1_048_576;
/// Per-literal byte tariff before exact parsing.
const LITERAL: usize = 1024;

/// Private causes keep malformed evidence separate from mathematical outcomes.
#[derive(Debug, PartialEq, Eq)]
enum Cause {
    /// Input read failed.
    Io,
    /// Envelope exceeds its byte ceiling.
    Bytes,
    /// Closed JSON shape is invalid.
    Json,
    /// Namespace does not identify this format.
    Namespace,
    /// Version is not the current triple.
    Version,
    /// Envelope attribution or references differ.
    Envelope,
    /// Ordered route declarations are invalid.
    Routes,
    /// Entry count is outside the admitted range.
    Entries,
    /// Entry identity is empty or repeated.
    Identity,
    /// Rational literal exceeds its tariff.
    Literal,
    /// Rational spelling is not canonical.
    Rational,
    /// Represented spelling does not have sixteen digits.
    BitsLength,
    /// Represented spelling is not lowercase hexadecimal.
    BitsCase,
    /// Represented value is not finite.
    NonFinite,
    /// A represented total is negative.
    Radius,
    /// Wall centre has negative-zero bits.
    CenterZero,
    /// Wall total has negative-zero bits.
    TotalZero,
    /// Declared outcome differs from reconstruction.
    Expected,
    /// Evidence does not bind the same row and route.
    Association,
    /// Golden values differ from the exact image.
    Golden,
}

/// Admission failure with the exact arithmetic receipt preserved.
#[derive(Debug, thiserror::Error)]
enum Error {
    /// Malformed corpus evidence.
    #[error("clamp golden {0:?}")]
    Invalid(Cause),
    /// Bounded exact arithmetic refused.
    #[error("clamp golden arithmetic: {0}")]
    Arithmetic(#[from] nrz_cas::Fault),
}
/// Result of private corpus admission.
type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
/// Closed envelope for the separate clamp format.
struct Payload {
    /// Format namespace.
    namespace: String,
    /// Exact current version triple.
    version: [u32; 3],
    /// Corpus owner.
    owner: String,
    /// Data license identifier.
    license: String,
    /// Design source paths.
    sources: Vec<String>,
    /// Schema labels.
    labels: Vec<String>,
    /// Exactly two ordered route declarations.
    routes: [Route; 2],
    /// Unsorted named cases.
    entries: Vec<Row>,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
/// Runner-declared provenance for one independent route.
struct Route {
    /// Nonempty identity, unique in its registry.
    id: String,
    /// Route algorithm kind.
    kind: String,
    /// Runner-declared implementation revision.
    revision: String,
    /// Reproduction argument and provenance limit.
    argument: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
/// Represented varying operand, retained as written.
struct Input {
    /// Varying centre bits.
    x: String,
    /// Varying total bits.
    r: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
/// Declared rational wall and its represented bundle.
struct Wall {
    /// Exact rational wall.
    a: String,
    /// Nearest-even wall centre bits.
    c: String,
    /// Least covering wall total bits.
    t: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
/// Exact image and nominal numerical readings.
struct Values {
    /// Lower and upper Cartesian image endpoints.
    image: [String; 2],
    /// Lower and upper wall-only image endpoints.
    nominal: [String; 2],
    /// Candidate centre as an exact rational.
    center: String,
    /// Numerical error as an exact rational.
    error: String,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
/// One route result associated with the complete row operands.
struct Evidence {
    /// Reference to the ordered route identity.
    route: String,
    /// Complete represented varying operand.
    input: Input,
    /// Complete declared and represented wall.
    wall: Wall,
    #[serde(deserialize_with = "optional_values")]
    /// Exact values, or explicit null for a refusal.
    golden: Option<Values>,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
/// One named value-only clamp case.
struct Row {
    /// Nonempty identity, unique in its registry.
    id: String,
    /// Complete represented varying operand.
    input: Input,
    /// Complete declared and represented wall.
    wall: Wall,
    /// Mathematical outcome name.
    expected: String,
    #[serde(deserialize_with = "optional_values")]
    /// Exact values, or explicit null for a refusal.
    golden: Option<Values>,
    /// Exactly two ordered associated route results.
    evidence: [Evidence; 2],
}

/// Null is meaningful, but omission is not another spelling of null.
fn optional_values<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> std::result::Result<Option<Values>, D::Error> {
    Option::<Values>::deserialize(d)
}

/// Attach a typed cause to one Boolean invariant.
const fn require(value: bool, cause: Cause) -> Result<()> {
    if value {
        Ok(())
    } else {
        Err(Error::Invalid(cause))
    }
}

/// Fresh bounded arithmetic allowance for one entry.
fn ledger() -> Result<Budget> {
    Ok(Budget::new(Limits {
        work: 200_000_000,
        literal_bytes: LITERAL as u64,
        integer_exponent: 4096,
        component_bits: 4096,
        arithmetic_steps: 20_000_000,
        ..Limits::default()
    })?)
}

/// Charge, parse and require the canonical exact spelling.
fn rational(text: &str, b: &mut Budget) -> Result<Rational> {
    require(text.len() <= LITERAL, Cause::Literal)?;
    b.work(
        text.len() as u64,
        nrz_cas::Location::new("clamp golden literal"),
    )?;
    let value = Rational::parse(text, b)?;
    require(value.exact_string(b)? == text, Cause::Rational)?;
    Ok(value)
}

/// Validate spelling and finiteness without canonicalizing zero.
fn bits(text: &str) -> Result<u64> {
    require(text.len() == 16, Cause::BitsLength)?;
    require(
        text.bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c)),
        Cause::BitsCase,
    )?;
    let value = text.bytes().fold(0_u64, |acc, digit| {
        let nibble = if digit.is_ascii_digit() {
            digit - b'0'
        } else {
            digit.to_ascii_lowercase() - b'a' + 10
        };
        (acc << 4) | u64::from(nibble)
    });
    require(value & !SIGN <= MAX, Cause::NonFinite)?;
    Ok(value)
}

/// Decode represented finite bits using integers, without floating arithmetic.
fn dyadic(bits: u64, b: &mut Budget) -> Result<Rational> {
    let magnitude = bits & !SIGN;
    let field = magnitude >> 52;
    let significand = (magnitude & ((1 << 52) - 1)) + if field == 0 { 0 } else { 1 << 52 };
    let exponent = if field == 0 {
        -1074
    } else {
        field.cast_signed() - 1075
    };
    let sign = if bits & SIGN == 0 { 1 } else { -1 };
    Ok(
        Rational::from_ratio(sign * significand.cast_signed(), 1, b)?
            .mul(&Rational::from_ratio(2, 1, b)?.pow(exponent, b)?, b)?,
    )
}

/// Exact magnitude under the entry allowance.
fn abs(value: Rational, b: &mut Budget) -> Result<Rational> {
    if value.is_negative() {
        Ok(Rational::zero().sub(&value, b)?)
    } else {
        Ok(value)
    }
}

/// Integer search followed by an exact midpoint comparison, ties to even.
fn nearest(value: &Rational, b: &mut Budget) -> Result<Option<u64>> {
    let negative = value.is_negative();
    let value = abs(value.clone(), b)?;
    let mut lo = 0;
    let mut hi = MAX;
    while lo < hi {
        let mid = lo + (hi - lo).div_ceil(2);
        if dyadic(mid, b)?.compare(&value, b)? == Ordering::Greater {
            hi = mid - 1;
        } else {
            lo = mid;
        }
    }
    let lower = dyadic(lo, b)?;
    let upper = if lo == MAX {
        lower.add(&lower.sub(&dyadic(lo - 1, b)?, b)?, b)?
    } else {
        dyadic(lo + 1, b)?
    };
    let twice = value.add(&value, b)?;
    let comparison = twice.compare(&lower.add(&upper, b)?, b)?;
    let up = comparison == Ordering::Greater || (comparison == Ordering::Equal && lo & 1 != 0);
    let selected = lo + u64::from(up);
    if selected > MAX {
        return Ok(None);
    }
    Ok(Some(
        selected | if negative && selected != 0 { SIGN } else { 0 },
    ))
}

/// Adjacent finite value in the requested direction.
fn neighbor(value: u64, up: bool) -> Option<u64> {
    let result = if value & !SIGN == 0 {
        if up { 1 } else { SIGN | 1 }
    } else if (value & SIGN == 0) == up {
        value + 1
    } else {
        value - 1
    };
    (result & !SIGN <= MAX).then_some(result)
}

/// A scalar primitive forms both neighbours before either endpoint can be read.
fn rounded_neighbor(value: u64, up: bool) -> Option<u64> {
    neighbor(value, !up)?;
    neighbor(value, up)
}

/// Exact ordered selection under the entry allowance.
fn maximum(a: Rational, c: &Rational, b: &mut Budget) -> Result<Rational> {
    Ok(if a.compare(c, b)? == Ordering::Less {
        c.clone()
    } else {
        a
    })
}

/// Reconstruct the represented input box and the contract's exact Cartesian image.
#[expect(
    clippy::many_single_char_names,
    reason = "Names match the represented roles in the numerical contract."
)]
fn outcome(row: &Row, b: &mut Budget) -> Result<(&'static str, Option<Values>)> {
    let x_bits = bits(&row.input.x)?;
    let r_bits = bits(&row.input.r)?;
    let c_bits = bits(&row.wall.c)?;
    let t_bits = bits(&row.wall.t)?;
    require(c_bits != SIGN, Cause::CenterZero)?;
    require(t_bits != SIGN, Cause::TotalZero)?;
    let x = dyadic(x_bits, b)?;
    let r = dyadic(r_bits, b)?;
    let c = dyadic(c_bits, b)?;
    let t = dyadic(t_bits, b)?;
    require(!r.is_negative() && !t.is_negative(), Cause::Radius)?;
    let a = rational(&row.wall.a, b)?;
    if nearest(&a, b)? != Some(c_bits) {
        return Ok(("ClampWallCenterMismatch", None));
    }
    let distance = abs(c.sub(&a, b)?, b)?;
    if t.compare(&distance, b)? == Ordering::Less {
        return Ok(("ClampWallNotEnclosed", None));
    }
    if t_bits != 0 && dyadic(t_bits - 1, b)?.compare(&distance, b)? != Ordering::Less {
        return Ok(("ClampWallTotalMismatch", None));
    }
    let lower = if x == r {
        Some(0)
    } else {
        nearest(&x.sub(&r, b)?, b)?.and_then(|n| rounded_neighbor(n, false))
    };
    let upper = if x.is_zero() {
        Some(r_bits & !SIGN)
    } else if r.is_zero() {
        Some(x_bits)
    } else {
        nearest(&x.add(&r, b)?, b)?.and_then(|n| rounded_neighbor(n, true))
    };
    let (Some(lower), Some(upper)) = (lower, upper) else {
        return Ok(("FiniteRange", None));
    };
    let wall_lo = c.sub(&t, b)?;
    let wall_hi = c.add(&t, b)?;
    let image = [
        maximum(dyadic(lower, b)?, &wall_lo, b)?.exact_string(b)?,
        maximum(dyadic(upper, b)?, &wall_hi, b)?.exact_string(b)?,
    ];
    let nominal = [
        maximum(x.clone(), &wall_lo, b)?.exact_string(b)?,
        maximum(x.clone(), &wall_hi, b)?.exact_string(b)?,
    ];
    let values = Values {
        image,
        nominal,
        center: maximum(x, &c, b)?.exact_string(b)?,
        error: t.exact_string(b)?,
    };
    Ok(("Enclosure", Some(values)))
}

/// Admit the bounded envelope and reconstruct every associated result.
#[expect(
    clippy::suspicious_operation_groupings,
    reason = "An evidence route names a registry id; operands name the row operands."
)]
fn read(mut input: impl Read) -> Result<Payload> {
    let mut bytes = Vec::new();
    input
        .by_ref()
        .take(BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| Error::Invalid(Cause::Io))?;
    require(bytes.len() as u64 <= BYTES, Cause::Bytes)?;
    let payload: Payload =
        serde_json::from_slice(&bytes).map_err(|_| Error::Invalid(Cause::Json))?;
    require(
        payload.namespace == "org.naturrhythmuszentrum.test.golden.clamp-floor",
        Cause::Namespace,
    )?;
    require(payload.version == [1, 0, 0], Cause::Version)?;
    require(
        payload.owner == "NRZ" && payload.license == "CC0-1.0",
        Cause::Envelope,
    )?;
    require(
        payload.sources == ["docs/design/matrix-loans-design.md"]
            && payload.labels == ["spec:nrz:clamp-floor-golden-values-schema"],
        Cause::Envelope,
    )?;
    let mut ids = BTreeSet::new();
    for (route, kind) in payload
        .routes
        .iter()
        .zip(["nrz-cas-clamp", "integer-clamp"])
    {
        require(
            !route.id.is_empty()
                && ids.insert(&route.id)
                && route.kind == kind
                && !route.revision.is_empty()
                && !route.argument.is_empty(),
            Cause::Routes,
        )?;
    }
    require(
        !payload.entries.is_empty() && payload.entries.len() <= 256,
        Cause::Entries,
    )?;
    let mut seen = BTreeSet::new();
    for row in &payload.entries {
        require(!row.id.is_empty() && seen.insert(&row.id), Cause::Identity)?;
        let mut b = ledger()?;
        let (expected, values) = outcome(row, &mut b)?;
        require(row.expected == expected, Cause::Expected)?;
        if let Some(golden) = &row.golden {
            for text in golden
                .image
                .iter()
                .chain(&golden.nominal)
                .chain([&golden.center, &golden.error])
            {
                rational(text, &mut b)?;
            }
        }
        require(row.golden == values, Cause::Golden)?;
        for (evidence, route) in row.evidence.iter().zip(&payload.routes) {
            require(
                evidence.route == route.id
                    && evidence.input == row.input
                    && evidence.wall == row.wall,
                Cause::Association,
            )?;
            require(evidence.golden == row.golden, Cause::Association)?;
        }
    }
    Ok(payload)
}

/// Admit the separate authored file before the tracked-text scan and derivation.
#[expect(
    clippy::redundant_pub_crate,
    reason = "Only the parent documentation pipeline admits this private format."
)]
pub(super) fn check_input(root: &Path) -> std::result::Result<(), crate::Error> {
    let file = match std::fs::File::open(root.join(PATH)) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(source) => return Err(crate::error::io(PATH, source)),
    };
    read(file)
        .map(|_| ())
        .map_err(|error| crate::Error::Payload {
            path: PATH.into(),
            detail: error.to_string(),
        })
}
