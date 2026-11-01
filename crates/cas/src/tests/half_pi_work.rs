// SPDX-License-Identifier: AGPL-3.0-only
//! Bounded statement and certificate codecs preserve canonical value payloads.

use crate::encoding::Encoder;
use crate::limits::{Budget, BudgetKind};
use crate::testing::common::{TestResult, default_budget};
use crate::testing::half_pi::{Fixture, prepared};
use crate::testing::half_pi_limits::{KINDS, ceiling, verify};
use crate::wire_decode::Decoded;
use crate::wire_read::Reader;
use crate::{Fault, FaultKind, Location};

#[derive(Clone, Copy, Debug)]
enum Codec {
    StatementEncode,
    CertificateEncode,
    StatementDecode,
    CertificateDecode,
}

fn encode(f: &Fixture, certificate: bool, budget: &mut Budget) -> Result<Vec<u8>, Fault> {
    let objects = crate::statement::objects(&f.expected.statement);
    let mut out = Encoder::new(budget, Location::new("half-Pi codec Work"));
    let write = |handle: &crate::Handle, out: &mut Encoder<'_>| {
        let index = objects
            .iter()
            .position(|value| *value == handle)
            .ok_or_else(crate::wire_read::invalid)?;
        crate::wire::index(
            u32::try_from(index).map_err(|_| crate::wire_read::invalid())?,
            out,
        )
    };
    if certificate {
        let value = &f
            .steps
            .last()
            .ok_or_else(crate::wire_read::invalid)?
            .certificate;
        crate::statement::certificate(value, &mut out, write)?;
    } else {
        crate::statement::encode(&f.expected.statement, &mut out, write)?;
    }
    Ok(out.finish())
}

fn run(f: &Fixture, codec: Codec, bytes: &[u8], budget: &mut Budget) -> Result<(), Fault> {
    match codec {
        Codec::StatementEncode | Codec::CertificateEncode => {
            assert_eq!(
                encode(f, matches!(codec, Codec::CertificateEncode), budget)?,
                bytes
            );
        }
        Codec::StatementDecode | Codec::CertificateDecode => {
            let records: Vec<_> = crate::statement::objects(&f.expected.statement)
                .into_iter()
                .cloned()
                .map(Decoded::Object)
                .collect();
            let mut reader = Reader::new(bytes, budget)?;
            if matches!(codec, Codec::StatementDecode) {
                assert_eq!(
                    crate::wire_statement::statement(&mut reader, &records)?,
                    f.expected.statement
                );
            } else {
                assert_eq!(
                    crate::wire_statement::certificate(&mut reader, &records)?,
                    f.steps
                        .last()
                        .ok_or_else(crate::wire_read::invalid)?
                        .certificate
                );
            }
            reader.finished()?;
        }
    }
    Ok(())
}

fn attempt(codec: Codec, warm: bool, kind: BudgetKind, amount: u64) -> TestResult<(bool, u64)> {
    let f = prepared(
        "18446744073709551617",
        "18446744073709551617/2",
        false,
        false,
    )?;
    let bytes = encode(
        &f,
        matches!(codec, Codec::CertificateEncode | Codec::CertificateDecode),
        &mut default_budget()?,
    )?;
    if warm {
        run(&f, codec, &bytes, &mut default_budget()?)?;
    }
    let counts = f.cas.counts();
    let before = f.cas.canonical_snapshot(&mut default_budget()?)?;
    let limits = ceiling(kind, amount);
    let mut budget = Budget::new(limits.clone())?;
    budget.work(37, Location::new("prior half-Pi codec Work"))?;
    let result =
        budget.with_retained_payload(19, Location::new("prior half-Pi codec payload"), |budget| {
            run(&f, codec, &bytes, budget)
        });
    assert_eq!(budget.limits(), &limits);
    // The policy requires intermediate >= component. At 64 both are 64;
    // canonical magnitude admission checks ComponentBits first in either codec.
    let expected = if kind == BudgetKind::IntermediateBits {
        BudgetKind::ComponentBits
    } else {
        kind
    };
    if let Err(fault) = &result {
        assert!(
            matches!(fault.kind, FaultKind::Exhausted { budget, .. } if budget == expected),
            "{codec:?}, {kind:?}: {fault:?}"
        );
    }
    assert_eq!(f.cas.counts(), counts);
    assert_eq!(f.cas.canonical_snapshot(&mut default_budget()?)?, before);
    Ok((result.is_ok(), budget.usage().work))
}

/// Costs and failed prefixes exclude 37 prior Work; staging includes 19 caller bytes.
/// Each cold and warm row is a separate complete-operation server measurement.
const COSTS: [(Codec, bool, [u64; 18], [u64; 18]); 8] = [
    // StatementEncode, cold: positive prefixes retain prior successful charges; zero prefixes are initial preflights.
    (
        Codec::StatementEncode,
        false,
        [104, 1, 0, 0, 51, 32, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        [100, 5, 0, 0, 100, 100, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    ),
    // StatementEncode, warm: positive prefixes retain prior successful charges; zero prefixes are initial preflights.
    (
        Codec::StatementEncode,
        true,
        [104, 1, 0, 0, 51, 32, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        [100, 5, 0, 0, 100, 100, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    ),
    // CertificateEncode, cold: positive prefixes retain prior successful charges; zero prefixes are initial preflights.
    (
        Codec::CertificateEncode,
        false,
        [120, 1, 0, 0, 67, 48, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        [112, 5, 0, 0, 112, 112, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    ),
    // CertificateEncode, warm: positive prefixes retain prior successful charges; zero prefixes are initial preflights.
    (
        Codec::CertificateEncode,
        true,
        [120, 1, 0, 0, 67, 48, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        [112, 5, 0, 0, 112, 112, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    ),
    // StatementDecode, cold: positive prefixes retain prior successful charges; zero prefixes are initial preflights.
    (
        Codec::StatementDecode,
        false,
        [372, 5, 0, 0, 39, 19, 32, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        [368, 292, 0, 0, 41, 364, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    ),
    // StatementDecode, warm: positive prefixes retain prior successful charges; zero prefixes are initial preflights.
    (
        Codec::StatementDecode,
        true,
        [372, 5, 0, 0, 39, 19, 32, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        [368, 292, 0, 0, 41, 364, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    ),
    // CertificateDecode, cold: positive prefixes retain prior successful charges; zero prefixes are initial preflights.
    (
        Codec::CertificateDecode,
        false,
        [388, 5, 0, 0, 39, 19, 48, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        [380, 292, 0, 0, 41, 364, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    ),
    // CertificateDecode, warm: positive prefixes retain prior successful charges; zero prefixes are initial preflights.
    (
        Codec::CertificateDecode,
        true,
        [388, 5, 0, 0, 39, 19, 48, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        [380, 292, 0, 0, 41, 364, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    ),
];

/// Every positive ceiling has a typed one-below refusal; zero ceilings execute successfully.
/// ´test:crate:half-pi-codec-boundaries´
#[test]
fn half_pi_codec_boundaries() -> TestResult {
    for (operation, warm, costs, prefixes) in COSTS {
        verify(
            &KINDS,
            |_| costs[0],
            &costs,
            &prefixes,
            |kind, amount| attempt(operation, warm, kind, amount),
        )?;
    }
    Ok(())
}

/// Operation, warmth, complete Work, bit ceilings and failed Work prefixes.
type BitRow = (Codec, bool, u64, [u64; 2], [u64; 2]);

/// Bit refusals retain the literal successful prefix for each independently measured row.
const BITS: [BitRow; 8] = [
    (Codec::StatementEncode, false, 104, [65, 65], [5, 5]),
    (Codec::StatementEncode, true, 104, [65, 65], [5, 5]),
    (Codec::CertificateEncode, false, 120, [65, 65], [5, 5]),
    (Codec::CertificateEncode, true, 120, [65, 65], [5, 5]),
    (Codec::StatementDecode, false, 372, [65, 65], [37, 37]),
    (Codec::StatementDecode, true, 372, [65, 65], [37, 37]),
    (Codec::CertificateDecode, false, 388, [65, 65], [37, 37]),
    (Codec::CertificateDecode, true, 388, [65, 65], [37, 37]),
];

/// Concrete integer components and exact arithmetic refuse before oversized allocation.
/// ´test:crate:half-pi-codec-bits´
#[test]
fn half_pi_codec_bits() -> TestResult {
    let kinds = [BudgetKind::ComponentBits, BudgetKind::IntermediateBits];
    for (operation, warm, work, costs, prefixes) in BITS {
        verify(
            &kinds,
            |_| work,
            &costs,
            &prefixes,
            |kind, amount| attempt(operation, warm, kind, amount),
        )?;
    }
    Ok(())
}
