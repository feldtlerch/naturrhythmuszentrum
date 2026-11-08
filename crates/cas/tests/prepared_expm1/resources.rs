// SPDX-License-Identifier: AGPL-3.0-only
//! Resource and namespace controls independent of the proposal arithmetic.

use super::{
    Counts, Fixture, InvalidCode, Limits, Outcome, Policy, Rational, TestResult, invalid, ledger,
    number, power, prepared, request,
};
use nrz_cas::limits::{Budget, BudgetKind};
use nrz_cas::{Exhaustion, FaultKind};

/// Permanent limits govern all routes and caller ceilings are restored without refund.
/// # Panics
/// Panics if any route bypasses the store or changes the original exhaustion.
/// ´claim:cas:prepared-expm1-permanent-limits´
/// ´test:integration:prepared-expm1-permanent-limits´
#[test]
fn prepared_expm1_permanent_limits() -> TestResult {
    let mut source = Fixture::new()?;
    let source_request = request(&mut source, number(-1, 1)?, 40)?;
    let proof = prepared(&mut source, &source_request)?.proof().clone();
    for route in [2, 0, 1] {
        let mut limits = ledger()?.limits().clone();
        limits.integer_exponent = 256;
        let mut f = Fixture::with_names_and_limits(&["x"], limits)?;
        let r = request(&mut f, number(-1, 1)?, 40)?;
        let mut b = ledger()?;
        b.work(7, nrz_cas::Location::new("caller prefix"))?;
        let result = match route {
            0 => f
                .cas
                .prepare_expm1(&r, Policy::default(), &mut b, &mut Counts::default())
                .map(drop),
            1 => f
                .cas
                .propose_expm1(&r, Policy::default(), 1, &mut b, &mut Counts::default())
                .map(drop),
            _ => f
                .cas
                .check_expm1(
                    &r,
                    Policy::default(),
                    &proof,
                    &mut b,
                    &mut Counts::default(),
                )
                .map(drop),
        };
        assert_eq!(
            result.expect_err("store exponent").kind,
            FaultKind::Exhausted {
                reason: Exhaustion::Limit,
                budget: BudgetKind::IntegerExponent,
                limit: 256,
                needed: 1022,
            }
        );
        assert_eq!(b.limits().integer_exponent, 4096);
        assert!(b.usage().work > 7);
    }
    Ok(())
}

/// A warm exact-zero theorem still checks every retained rational before copying it.
/// # Panics
/// Panics if a valid but oversized conservative bound escapes the caller ceiling.
/// ´claim:cas:prepared-expm1-warm-components´
/// ´test:integration:prepared-expm1-warm-components´
#[test]
fn prepared_expm1_warm_components() -> TestResult {
    let mut f = Fixture::new()?;
    let r = request(&mut f, Rational::zero(), 40)?;
    let mut proof = f.cas.propose_expm1(
        &r,
        Policy::default(),
        1,
        &mut ledger()?,
        &mut Counts::default(),
    )?;
    proof.endpoint_ratio = power(3000)?;
    f.cas.check_expm1(
        &r,
        Policy::default(),
        &proof,
        &mut ledger()?,
        &mut Counts::default(),
    )?;
    let mut limits = ledger()?.limits().clone();
    limits.component_bits = 2048;
    let mut b = Budget::new(limits)?;
    let error = f
        .cas
        .prepare_expm1(&r, Policy::default(), &mut b, &mut Counts::default())
        .expect_err("warm component");
    assert_eq!(
        error.kind,
        FaultKind::Exhausted {
            reason: Exhaustion::ArithmeticRange,
            budget: BudgetKind::ComponentBits,
            limit: 2048,
            needed: 3001,
        }
    );
    Ok(())
}

/// A stale first proof cannot hide a current second proof of the same statement.
/// # Panics
/// Panics if a warm lookup reconstructs coefficients or replays either proof.
/// ´claim:cas:prepared-expm1-stale-scan´
/// ´test:integration:prepared-expm1-stale-scan´
#[test]
fn prepared_expm1_stale_scan() -> TestResult {
    let mut f = Fixture::new()?;
    let r = request(&mut f, Rational::zero(), 40)?;
    let ordinary = f.cas.propose_expm1(
        &r,
        Policy::default(),
        1,
        &mut ledger()?,
        &mut Counts::default(),
    )?;
    let mut enlarged = ordinary.clone();
    enlarged.endpoint_ratio = enlarged
        .endpoint_ratio
        .add(&Rational::one(), &mut ledger()?)?;
    f.cas.check_expm1(
        &r,
        Policy::default(),
        &enlarged,
        &mut ledger()?,
        &mut Counts::default(),
    )?;
    let mut wrong = ordinary.clone();
    wrong.version = 2;
    invalid(
        f.cas
            .check_expm1(
                &r,
                Policy::default(),
                &wrong,
                &mut ledger()?,
                &mut Counts::default(),
            )
            .expect_err("rollback"),
        InvalidCode::UnsupportedVersion,
    );
    f.cas.check_expm1(
        &r,
        Policy::default(),
        &ordinary,
        &mut ledger()?,
        &mut Counts::default(),
    )?;
    assert_eq!(f.cas.retained_judgment_count(), 2);
    for _ in 0..2 {
        let mut counts = Counts::default();
        assert!(matches!(
            f.cas
                .prepare_expm1(&r, Policy::default(), &mut ledger()?, &mut counts)?,
            Outcome::Ready(_)
        ));
        assert_eq!(
            (
                counts.proposals,
                counts.coefficients,
                counts.checks,
                counts.publications
            ),
            (0, 0, 0, 0)
        );
    }
    Ok(())
}

/// Scope bytes and a returned copy must coexist with all retained graph judgments.
/// # Panics
/// Panics if a warm request fits only by omitting simultaneous owned payload.
/// ´claim:cas:prepared-expm1-live-payload´
/// ´test:integration:prepared-expm1-live-payload´
#[test]
fn prepared_expm1_live_payload() -> TestResult {
    let name = "x".repeat(65_536);
    let limits = Limits {
        name_bytes: 131_072,
        source_bytes: 4_194_304,
        witness_bytes: 4_194_304,
        ..ledger()?.limits().clone()
    };
    let mut f = Fixture::with_names_and_limits(&[&name], limits)?;
    let r = request(&mut f, Rational::zero(), 40)?;
    let mut b = Budget::new(Limits {
        name_bytes: 131_072,
        source_bytes: 4_194_304,
        witness_bytes: 4_194_304,
        ..ledger()?.limits().clone()
    })?;
    let proof = f
        .cas
        .propose_expm1(&r, Policy::default(), 1, &mut b, &mut Counts::default())?;
    assert!(proof.scope.len() >= 65_536);
    assert_eq!(proof.coefficients.len(), 1);
    assert_eq!(proof.stages.len(), 1);
    f.cas.check_expm1(
        &r,
        Policy::default(),
        &proof,
        &mut b,
        &mut Counts::default(),
    )?;
    // Three distinct claims make H exceed the individual witness-DAG scratch bound.
    // Otherwise that unrelated bound could mask the omitted simultaneous reservation.
    for bits in [41, 39] {
        let other = request(&mut f, Rational::zero(), bits)?;
        let proof =
            f.cas
                .propose_expm1(&other, Policy::default(), 1, &mut b, &mut Counts::default())?;
        f.cas.check_expm1(
            &other,
            Policy::default(),
            &proof,
            &mut b,
            &mut Counts::default(),
        )?;
    }
    assert_eq!(f.cas.retained_judgment_count(), 3);
    let h = f.cas.retained_judgment_bytes();
    // Six vertex-kind counters and one ledger registration need 6*64+64 bytes.
    let cap = h + 384 + 64;
    let mut limits = b.limits().clone();
    limits.staged_payload_bytes = cap;
    let mut warm = Budget::new(limits)?;
    let error = f
        .cas
        .prepare_expm1(&r, Policy::default(), &mut warm, &mut Counts::default())
        .map(drop)
        .expect_err("live payload");
    assert_eq!(error.location.operation, "prepared expm1");
    assert!(matches!(
        error.kind,
        FaultKind::Exhausted {
            reason: Exhaustion::Limit,
            budget: BudgetKind::StagedPayloadBytes,
            ..
        }
    ));
    Ok(())
}

/// Supplied record counts refuse before any inventory walk, even with only checkpoint Work.
/// # Panics
/// Panics if malformed direct input is traversed or loses its typed count refusal.
/// ´claim:cas:prepared-expm1-inventory-order´
/// ´test:integration:prepared-expm1-inventory-order´
#[test]
fn prepared_expm1_inventory_order() -> TestResult {
    let mut f = Fixture::new()?;
    let r = request(&mut f, Rational::zero(), 40)?;
    let mut proof = f.cas.propose_expm1(
        &r,
        Policy::default(),
        1,
        &mut ledger()?,
        &mut Counts::default(),
    )?;
    proof.stages = vec![proof.stages[0].clone(); 1_000_000];
    let mut counts = Counts::default();
    let mut b = Budget::new(Limits {
        work: 256,
        ..ledger()?.limits().clone()
    })?;
    let error = f
        .cas
        .check_expm1(&r, Policy::default(), &proof, &mut b, &mut counts)
        .map(drop)
        .expect_err("stage count");
    invalid(error, InvalidCode::Expm1Rounding);
    assert_eq!(counts.proof_records, 0);
    assert_eq!(counts.coordinates, 0);
    proof.stages.truncate(1);
    let mut counts = Counts::default();
    let mut b = Budget::new(Limits {
        work: 256,
        ..ledger()?.limits().clone()
    })?;
    let error = f
        .cas
        .check_expm1(&r, Policy::default(), &proof, &mut b, &mut counts)
        .map(drop)
        .expect_err("prepaid traversal");
    assert_eq!(
        error.kind,
        FaultKind::Exhausted {
            reason: Exhaustion::Limit,
            budget: BudgetKind::Work,
            limit: 256,
            needed: 263
        }
    );
    assert_eq!(counts.proof_records, 0);
    let mut many = r.clone();
    many.original.coordinates = vec![
        super::CoordinateBound {
            coordinate: r.original.target.clone(),
            interval: super::Interval::singleton(&Rational::zero(), &mut ledger()?)?
        };
        1_000_000
    ];
    for route in [2, 0, 1] {
        let mut counts = Counts::default();
        let mut b = Budget::new(Limits {
            work: 256,
            matrix_entries: 16,
            ..ledger()?.limits().clone()
        })?;
        let result = match route {
            0 => f
                .cas
                .prepare_expm1(&many, Policy::default(), &mut b, &mut counts)
                .map(drop),
            1 => f
                .cas
                .propose_expm1(&many, Policy::default(), 1, &mut b, &mut counts)
                .map(drop),
            _ => f
                .cas
                .check_expm1(&many, Policy::default(), &proof, &mut b, &mut counts)
                .map(drop),
        };
        assert_eq!(
            result.expect_err("coordinate ceiling").kind,
            FaultKind::Exhausted {
                reason: Exhaustion::Limit,
                budget: BudgetKind::MatrixEntries,
                limit: 16,
                needed: 1_000_000
            }
        );
        assert_eq!(counts.coordinates, 0);
        assert_eq!(counts.proof_records, 0);
    }
    Ok(())
}

/// Exact-zero replay keeps all owners alive through publication at its derived logical peak.
/// With `S=proof.scope.len()`, P=S+32+512+the eight rational inventory sizes;
/// T=S+54 is the degree-one statement and W is its exact portable witness length.
/// F=P+256+T+256 and K=T+4+W. At Vec-to-Arc publication, two K buffers
/// coexist with two F payloads, caller P, claim S and request 256: peak=320+256+P+S+2F+2K.
/// The 320 is five journal counters; an existing judgment adds H+128 for its payload,
/// the sixth counter and owner-registration slot. This is about 10*S,
/// below 10*S+65,536; the same derivation applies with a pre-existing owner registration.
/// # Panics
/// Panics if a live owner is omitted, double counted, or the exact boundary changes.
/// ´claim:cas:prepared-expm1-publication-peak´
/// ´test:integration:prepared-expm1-publication-peak´
#[test]
fn prepared_expm1_publication_peak() -> TestResult {
    let name = "x".repeat(65_536);
    let limits = ledger()?.limits().clone();
    let mut source = Fixture::with_names_and_limits(&[&name], limits.clone())?;
    let original = request(&mut source, Rational::zero(), 40)?;
    let proof = source.cas.propose_expm1(
        &original,
        Policy::default(),
        1,
        &mut ledger()?,
        &mut Counts::default(),
    )?;
    let scope = proof.scope.len() as u64;
    let mut proof_bytes = scope + 32 + 512;
    for value in [
        &proof.absolute_tail,
        &proof.relative_tail,
        &proof.import_ratio,
        &proof.central_ratio,
        &proof.endpoint_ratio,
        &proof.restoration_error,
        &proof.stages[0].magnitude,
        &proof.stages[0].error,
    ] {
        let (n, d) = value.component_bits();
        proof_bytes += 4 * (n.div_ceil(8) + d.div_ceil(8)) + 128;
    }
    let statement = scope + 54;
    let witness = proof.to_bytes(&mut ledger()?)?.len() as u64;
    let prepared_bytes = proof_bytes + 256 + statement + 256;
    let key = statement + 4 + witness;
    let peak = 320 + 256 + proof_bytes + scope + 2 * prepared_bytes + 2 * key;
    assert!(peak < 10 * scope + 65_536);
    for extra in [0, 1] {
        for allowance in [peak - 1, peak] {
            let mut dest = Fixture::with_names_and_limits(&[&name], limits.clone())?;
            let claim = request(&mut dest, Rational::zero(), 40)?;
            let held = if extra == 1 {
                let other = request(&mut dest, Rational::zero(), 41)?;
                let candidate = dest.cas.propose_expm1(
                    &other,
                    Policy::default(),
                    1,
                    &mut ledger()?,
                    &mut Counts::default(),
                )?;
                dest.cas.check_expm1(
                    &other,
                    Policy::default(),
                    &candidate,
                    &mut ledger()?,
                    &mut Counts::default(),
                )?;
                dest.cas.retained_judgment_bytes() + 128
            } else {
                0
            };
            let mut budget = Budget::new(Limits {
                staged_payload_bytes: allowance + held,
                ..limits.clone()
            })?;
            let result = dest.cas.check_expm1(
                &claim,
                Policy::default(),
                &proof,
                &mut budget,
                &mut Counts::default(),
            );
            if allowance == peak {
                let ready = result?;
                assert_eq!(ready.statement_key().len() as u64, statement);
            } else {
                let error = result.map(drop).expect_err("one below live peak");
                assert_eq!(error.location.operation, "prepared expm1");
                assert_eq!(
                    error.kind,
                    FaultKind::Exhausted {
                        reason: Exhaustion::Limit,
                        budget: BudgetKind::StagedPayloadBytes,
                        limit: key - 1 + if extra == 1 { held - 64 } else { 0 },
                        needed: key + if extra == 1 { held - 64 } else { 0 }
                    }
                );
            }
        }
    }
    Ok(())
}
