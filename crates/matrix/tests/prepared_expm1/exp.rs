// SPDX-License-Identifier: AGPL-3.0-only
//! Exp composition controls reuse exact test-side arithmetic, not native expected figures.
use super::*;
use nrz_matrix::{PreparedExp, PreparedExpError};
use nrz_scalar::{ExpFailure, ExpInput};

const fn input(x: f64) -> ExpInput {
    ExpInput {
        endpoints: [x, x],
        center: x,
        bits: 40,
    }
}

/// A degree-one exp bound can succeed although its uniform relative basis fails.
/// For h=2^-40, 1-h < exp(-h) < 1-h+h^2/2; both 1-h and
/// 1-h+2^-53 are binary64 values and their slacks are below 2^-40.
/// This is an independent analytic illustration, not a new CAS certificate.
/// # Panics
/// Panics if basis failure is conflated with exp target failure or loses its request.
/// ´claim:matrix:prepared-exp-basis-outcome´
/// ´test:integration:prepared-exp-basis-outcome´
#[test]
fn prepared_exp_basis_outcome() -> TestResult {
    let context = ArithmeticContext::new()?;
    let mut f = Fixture::new()?;
    let h = power(-40)?;
    let r = request(&mut f, h.negated(&mut ledger()?)?, 40)?;
    let failure = PreparedExp::new(
        &mut f.cas,
        &r,
        [-2.0_f64.powi(-40), -2.0_f64.powi(-40)],
        Policy {
            degree: 1,
            attempts: 1,
            ..Policy::default()
        },
        &context,
        &mut ledger()?,
        &mut Counts::default(),
    )
    .expect_err("no basis");
    let PreparedExpError::BasisPreparation { auxiliary, policy } = failure else {
        panic!("basis outcome");
    };
    assert_eq!(auxiliary, &r);
    assert_eq!(policy.magnitude_bound, [1, 1]);
    assert_eq!(policy.magnitude_exponent, 0);
    assert_eq!(policy.requested_bits, 40);
    let mut b = ledger()?;
    oracles::le(
        &h.mul(&h, &mut b)?.div(&number(2, 1)?, &mut b)?,
        &power(-53)?,
        &mut b,
    )?;
    oracles::le(&power(-53)?, &h, &mut b)?;
    assert_eq!(
        represented(1.0 - 2.0_f64.powi(-40), &mut b)?,
        Rational::one().sub(&h, &mut b)?
    );
    let zero_request = request(&mut f, Rational::zero(), 60)?;
    let zero_plan = PreparedExp::new(
        &mut f.cas,
        &zero_request,
        [0.0, 0.0],
        Policy::default(),
        &context,
        &mut b,
        &mut Counts::default(),
    )
    .map_err(|e| e.to_string())?;
    assert_eq!(
        zero_plan
            .evaluate(
                ExpInput {
                    bits: 60,
                    ..input(0.0)
                },
                &context
            )?
            .reading()
            .endpoints,
        [1.0, 1.0]
    );
    Ok(())
}

/// Exact truth, rounded nominal schedule and non-additive radius for 73 distinct points.
/// # Panics
/// Panics if the independent rational reconstruction disagrees with native evidence.
/// ´claim:matrix:prepared-exp-independent-oracles´
/// ´test:integration:prepared-exp-independent-oracles´
#[test]
fn prepared_exp_independent_oracles() -> TestResult {
    let mut f = Fixture::new()?;
    let r = broad(&mut f, 40)?;
    let context = ArithmeticContext::new()?;
    let mut counts = Counts::default();
    let plan = PreparedExp::new(
        &mut f.cas,
        &r,
        [-2.0, 0.0],
        Policy::default(),
        &context,
        &mut ledger()?,
        &mut counts,
    )
    .map_err(|e| e.to_string())?;
    let before = format!("{counts:?}");
    let mut points = vec![
        -2.0,
        0.0,
        -1.0,
        -2.0_f64.powi(-40),
        -2.0_f64.powi(-10),
        -f64::MIN_POSITIVE,
        -f64::MIN_POSITIVE.next_up(),
        -2.0_f64.powi(-60),
        (-2.0_f64).next_up(),
    ];
    for i in 0..64 {
        points.push(-f64::from((i * 37 + 11) % 64 + 1) / 128.0);
    }
    let distinct: std::collections::BTreeSet<_> = points.iter().map(|x| x.to_bits()).collect();
    assert_eq!(distinct.len(), 73);
    for x in points {
        check(&plan, input(x), &context)?;
    }
    for (endpoints, center) in [
        ([-2.0, 0.0], -1.0),
        ([-f64::MIN_POSITIVE, 0.0], -f64::MIN_POSITIVE / 2.0),
        ([-0.75, -0.25], -0.375),
    ] {
        check(
            &plan,
            ExpInput {
                endpoints,
                center,
                bits: 40,
            },
            &context,
        )?;
    }
    assert_eq!(format!("{counts:?}"), before);
    let image = plan
        .evaluate(
            ExpInput {
                endpoints: [-2.0, 0.0],
                center: -1.0,
                bits: 40,
            },
            &context,
        )?
        .reading();
    assert!((image.endpoints[0].midpoint(image.endpoints[1]) - image.center).abs() > 0.19);
    assert!(image.numerical_error < 2.0_f64.powi(-40));
    Ok(())
}

/// Independent reconstruction uses exact factorial-series truth and rational ties-to-even.
/// # Panics
/// Panics if containment, absolute slack, nominal rounding or radius composition fails.
fn check(plan: &PreparedExp<'_>, input: ExpInput, context: &ArithmeticContext) -> TestResult {
    let result = plan.evaluate(input, context)?;
    assert_eq!(result.bits(), 40);
    assert_eq!(result.input().endpoints, input.endpoints);
    assert_eq!(result.input().center.to_bits(), input.center.to_bits());
    assert_eq!(result.input().bits, input.bits);
    let out = result.reading();
    let mut b = ledger()?;
    let one = Rational::one();
    let mut truth = Vec::new();
    for x in input.endpoints {
        let pair = oracles::truth(&represented(-x, &mut b)?, &mut b)?;
        truth.push([pair[0].add(&one, &mut b)?, pair[1].add(&one, &mut b)?]);
    }
    let lo = represented(out.endpoints[0], &mut b)?;
    let hi = represented(out.endpoints[1], &mut b)?;
    oracles::le(&lo, &truth[0][0], &mut b)?;
    oracles::le(&truth[1][1], &hi, &mut b)?;
    oracles::le(&truth[0][1].sub(&lo, &mut b)?, &power(-40)?, &mut b)?;
    oracles::le(&hi.sub(&truth[1][0], &mut b)?, &power(-40)?, &mut b)?;
    let x = represented(input.center, &mut b)?;
    let mut rounded = Rational::zero();
    for c in plan.basis().proof().coefficients.iter().rev() {
        rounded = oracles::nearest(&rounded.mul(&x, &mut b)?, &mut b)?;
        rounded = oracles::nearest(&rounded.add(&c.exact(&mut b)?, &mut b)?, &mut b)?;
    }
    rounded = oracles::nearest(&rounded.mul(&x, &mut b)?, &mut b)?;
    rounded = oracles::nearest(&rounded.add(&one, &mut b)?, &mut b)?;
    assert_eq!(represented(out.center, &mut b)?, rounded);
    let nominal = oracles::truth(&represented(-input.center, &mut b)?, &mut b)?;
    for t in nominal {
        oracles::le(
            &oracles::absolute(&rounded.sub(&t.add(&one, &mut b)?, &mut b)?, &mut b)?,
            &represented(out.numerical_error, &mut b)?,
            &mut b,
        )?;
    }
    let mut expected = represented(out.numerical_error, &mut b)?;
    for endpoint in [&lo, &hi] {
        let difference = rounded.sub(endpoint, &mut b)?;
        let value = if difference.is_zero() {
            Rational::zero()
        } else {
            let nearest = oracles::nearest(&difference, &mut b)?;
            let outward = oracles::neighbour(&nearest, !difference.is_negative(), &mut b)?;
            oracles::absolute(&outward, &mut b)?
        };
        if value.compare(&expected, &mut b)? == std::cmp::Ordering::Greater {
            expected = value;
        }
    }
    assert_eq!(represented(out.total_radius, &mut b)?, expected);
    if input.center == 0.0 {
        assert_eq!(out.numerical_error.to_bits(), 0);
    }
    Ok(())
}

/// Stored exp rows are selected structurally; rational inputs use containing binary64 boxes.
/// The input box's full image width additionally bounds slack at the original rational point.
/// # Panics
/// Panics if any row, association, golden interval or refusal cause is lost.
/// ´claim:matrix:prepared-exp-golden-values´
/// ´test:integration:prepared-exp-golden-values´
#[test]
fn prepared_exp_golden_values() -> TestResult {
    let payload = nrz_docgen::golden::read(
        include_bytes!("../../../../data/golden/prepared-binary64.json").as_slice(),
        nrz_docgen::golden::Limits::default(),
        &mut ledger()?,
    )?;
    let context = ArithmeticContext::new()?;
    let mut ids = Vec::new();
    for row in payload.value()["entries"].as_array().ok_or("entries")? {
        if row["operation"] != "exp" {
            continue;
        }
        let id = row["id"].as_str().ok_or("id")?;
        match id {
            "R2" => assert_eq!(row["expected"]["refusal"], "outside-domain"),
            "R4" => assert_eq!(row["expected"]["refusal"], "target-not-certifiable"),
            _ => assert_eq!(row["expected"], "enclosure"),
        }
        ids.push(id);
        let text = row["input"]["point"].as_str().ok_or("input")?;
        let bits = u32::try_from(row["target"]["bits"].as_u64().ok_or("bits")?)?;
        assert_eq!(row["target"]["kind"], "absolute");
        let mut b = ledger()?;
        let exact = Rational::parse(text, &mut b)?;
        let (numerator, denominator) = text.split_once('/').unwrap_or((text, "1"));
        let candidate = numerator.parse::<f64>()? / denominator.parse::<f64>()?;
        let endpoints = [candidate.next_down(), candidate.next_up()];
        oracles::le(&represented(endpoints[0], &mut b)?, &exact, &mut b)?;
        oracles::le(&exact, &represented(endpoints[1], &mut b)?, &mut b)?;
        let mut f = Fixture::new()?;
        let mut r = broad(&mut f, bits)?;
        r.original.coordinates[0].interval = Interval::new(
            represented(endpoints[0], &mut b)?,
            represented(endpoints[1], &mut b)?,
            &mut b,
        )?;
        let plan = PreparedExp::new(
            &mut f.cas,
            &r,
            endpoints,
            Policy::default(),
            &context,
            &mut ledger()?,
            &mut Counts::default(),
        );
        if id == "R2" {
            assert!(matches!(
                plan,
                Err(PreparedExpError::Admission(PreparedExpm1Error::Domain))
            ));
            continue;
        }
        // R4's target has no auxiliary basis under this conservative law. It
        // must not masquerade as a final-target refusal; scalar controls below
        // exercise final spacing refusal using an independently admitted basis.
        if id == "R4" {
            // The dataset row is the fact; the public route currently stops one step earlier with no claim.
            assert!(plan.is_err(), "R4 must never return success");
            assert!(matches!(
                plan,
                Err(PreparedExpError::BasisPreparation { .. })
            ));
            continue;
        }
        let plan = plan.map_err(|e| e.to_string())?;
        let out = plan
            .evaluate(
                ExpInput {
                    endpoints,
                    center: candidate,
                    bits,
                },
                &context,
            )?
            .reading();
        let golden = row["golden"]["point"].as_array().ok_or("golden")?;
        let low = Rational::parse(golden[0].as_str().ok_or("low")?, &mut b)?;
        let high = Rational::parse(golden[1].as_str().ok_or("high")?, &mut b)?;
        let lo = represented(out.endpoints[0], &mut b)?;
        let hi = represented(out.endpoints[1], &mut b)?;
        oracles::le(&lo, &low, &mut b)?;
        oracles::le(&high, &hi, &mut b)?;
        oracles::le(&hi.sub(&lo, &mut b)?, &power(-i64::from(bits))?, &mut b)?;
    }
    ids.sort_unstable();
    assert_eq!(ids, ["E1", "E2", "E3", "R2", "R4"]);
    Ok(())
}

/// Exact zero, both domain neighbours, signed below-normal operands and target mismatch.
/// # Panics
/// Panics if a validation shortcut hides a distinct native refusal.
/// ´claim:matrix:prepared-exp-native-refusals´
/// ´test:integration:prepared-exp-native-refusals´
#[test]
fn prepared_exp_native_refusals() -> TestResult {
    let mut f = Fixture::new()?;
    let r = broad(&mut f, 40)?;
    let context = ArithmeticContext::new()?;
    let plan = PreparedExp::new(
        &mut f.cas,
        &r,
        [-2.0, 0.0],
        Policy::default(),
        &context,
        &mut ledger()?,
        &mut Counts::default(),
    )
    .map_err(|e| e.to_string())?;
    let zero = plan.evaluate(input(-0.0), &context)?.reading();
    assert_eq!(zero.endpoints, [1.0, 1.0]);
    assert_eq!(
        [zero.center, zero.total_radius, zero.numerical_error],
        [1.0, 0.0, 0.0]
    );
    for x in [(-2.0_f64).next_down(), 0.0_f64.next_up(), f64::NAN] {
        assert_eq!(
            plan.evaluate(input(x), &context).expect_err("domain"),
            ExpFailure::Basis(Expm1Failure::OutsideDomain)
        );
    }
    assert_eq!(
        plan.evaluate(input(-f64::MIN_POSITIVE.next_down()), &context)
            .expect_err("signed magnitude"),
        ExpFailure::Basis(Expm1Failure::BelowNormalOperand)
    );
    plan.evaluate(input(-f64::MIN_POSITIVE), &context)?;
    assert_eq!(
        plan.evaluate(
            ExpInput {
                bits: 39,
                ..input(0.0)
            },
            &context
        )
        .expect_err("target before zero"),
        ExpFailure::Target
    );
    mixed_cells(&plan, &context)?;
    // Reconstitute the public conditional native program from this checked basis
    // only for testing an exp target different from the preparation target.
    let proof = plan.basis().proof();
    let mut coefficients = Vec::new();
    for c in &proof.coefficients {
        let text = c.exact(&mut ledger()?)?.exact_string(&mut ledger()?)?;
        let (n, d) = text.split_once('/').unwrap_or((&text, "1"));
        coefficients.push(n.parse::<f64>()? / d.parse::<f64>()?);
    }
    let native = nrz_scalar::Expm1Program::new(
        &coefficients,
        nrz_scalar::Expm1Parameters {
            domain: [-2.0, 0.0],
            bits: 40,
            radius_bits: proof.radius_bits,
            absolute_error: 1.0,
            environment: 1,
        },
        &context,
    )?;
    for (x, bits) in [(-1.0, 60), (-2.0_f64.powi(-60), 58)] {
        let error = nrz_scalar::ExpProgram::new(&native, bits)
            .evaluate(ExpInput { bits, ..input(x) }, &context)
            .expect_err("exp spacing");
        let ExpFailure::TargetNotCertifiable {
            endpoint,
            image,
            width,
            bits: target,
        } = error
        else {
            panic!("final target cause");
        };
        assert_eq!((endpoint, target), (0, bits));
        assert!(f64::from_bits(image[0]) < f64::from_bits(image[1]));
        assert!(f64::from_bits(width) > 2.0_f64.powi(-i32::try_from(bits)?));
    }
    // On (1/4,1/2) the lattice spacing 2^-54 exceeds twice 2^-60.
    let mut b = ledger()?;
    let pair = oracles::truth(&Rational::one(), &mut b)?;
    oracles::le(
        &number(1, 4)?,
        &pair[0].add(&Rational::one(), &mut b)?,
        &mut b,
    )?;
    oracles::le(
        &pair[1].add(&Rational::one(), &mut b)?,
        &number(1, 2)?,
        &mut b,
    )?;
    assert_eq!(
        power(-54)?.compare(&power(-59)?, &mut b)?,
        std::cmp::Ordering::Greater
    );
    Ok(())
}

/// A failed cell retains its position beside successful neighbours.
/// # Panics
/// Panics if batch evaluation suppresses a cell refusal or reorders its neighbours.
fn mixed_cells(plan: &PreparedExp<'_>, context: &ArithmeticContext) -> TestResult {
    let values = plan.evaluate_batch(
        &[input(-0.25), input(0.25), input(-1.0)],
        context,
        &mut ledger()?,
    )?;
    assert_eq!(values.len(), 3);
    assert_eq!(
        values[0].as_ref().map(|v| v.reading().endpoints),
        Ok(plan.evaluate(input(-0.25), context)?.reading().endpoints)
    );
    assert_eq!(
        values[1].as_ref().expect_err("cell refusal"),
        &ExpFailure::Basis(Expm1Failure::OutsideDomain)
    );
    assert_eq!(
        values[2].as_ref().map(|v| v.reading().endpoints),
        Ok(plan.evaluate(input(-1.0), context)?.reading().endpoints)
    );
    Ok(())
}

/// Independent byte boundary: G plus returned scope/key, fixed native owner,
/// coefficients (32 each), six bound rationals, two per stage, and coordinate
/// inventories (256 plus two rationals each). Rational byte lengths come from
/// decimal long division, not the production estimator. Registered ledgers add
/// 64; a retained caller prefix leaves exactly S+64 or S+63 on a warm graph.
/// # Panics
/// Panics if any inventory is omitted, counted twice, or traversed before admission.
/// ´claim:matrix:prepared-exp-resources´
/// ´test:integration:prepared-exp-resources´
#[test]
fn prepared_exp_resources() -> TestResult {
    let mut f = Fixture::new()?;
    let r = broad(&mut f, 40)?;
    let context = ArithmeticContext::new()?;
    drop(
        PreparedExp::new(
            &mut f.cas,
            &r,
            [-2.0, 0.0],
            Policy::default(),
            &context,
            &mut ledger()?,
            &mut Counts::default(),
        )
        .map_err(|e| e.to_string())?,
    );
    let graph = f.cas.counts().payload_bytes;
    let judgments = f.cas.retained_judgment_bytes();
    let inputs = [input(-0.25), input(0.0), input(-1.0)];
    let output = 3 * size_of::<Result<nrz_matrix::ExpEvaluation<'_>, ExpFailure>>() as u64;
    let cells = size_of_val(&inputs) as u64 + output;
    for reused in [false, true] {
        for short in [0, 1] {
            let mut preparation = ledger()?;
            let plan = PreparedExp::new(
                &mut f.cas,
                &r,
                [-2.0, 0.0],
                Policy::default(),
                &context,
                &mut preparation,
                &mut Counts::default(),
            )
            .map_err(|e| e.to_string())?;
            let owned = exp_owned(&plan)?;
            let peak = graph + owned + cells + u64::from(reused) * 64;
            let mut fresh = ledger()?;
            let b = if reused { &mut preparation } else { &mut fresh };
            b.work(7, nrz_bindings::readings::algebra::Location::new("caller"))?;
            let before = b.usage().work;
            let prefix = b.limits().staged_payload_bytes - peak + short;
            let result = b.with_retained_payload(
                prefix,
                nrz_bindings::readings::algebra::Location::new("caller"),
                |b| plan.evaluate_batch(&inputs, &context, b),
            );
            if short == 0 {
                let values = result?;
                assert_eq!(values.len(), 3);
                for (value, input) in values.into_iter().zip(inputs) {
                    assert_eq!(
                        value?.reading().endpoints,
                        plan.evaluate(input, &context)?.reading().endpoints
                    );
                }
                assert_eq!(b.usage().work, before + 3 + u64::from(reused));
            } else {
                let needed = peak - if reused { graph - judgments } else { graph };
                assert_eq!(
                    result.expect_err("payload").kind,
                    FaultKind::Exhausted {
                        reason: nrz_bindings::readings::algebra::Exhaustion::Limit,
                        budget: BudgetKind::StagedPayloadBytes,
                        limit: needed - 1,
                        needed,
                    }
                );
                assert_eq!(b.usage().work, before + u64::from(reused));
            }
            ceilings(&plan, &inputs, &context, output)?;
        }
    }
    Ok(())
}

/// Exact independent cell tariffs with a spent prefix survive each typed refusal.
/// # Panics
/// Panics if limits or consumption differ from the declared inventory costs.
fn ceilings(
    plan: &PreparedExp<'_>,
    inputs: &[ExpInput],
    context: &ArithmeticContext,
    output: u64,
) -> TestResult {
    for (kind, ceiling) in [
        (BudgetKind::Work, 10),
        (BudgetKind::MatrixEntries, 3),
        (BudgetKind::OutputBytes, output),
    ] {
        for limit in [ceiling, ceiling - 1] {
            let mut limits = ledger()?.limits().clone();
            match kind {
                BudgetKind::Work => limits.work = limit,
                BudgetKind::MatrixEntries => limits.matrix_entries = limit,
                BudgetKind::OutputBytes => limits.output_bytes = limit,
                _ => unreachable!(),
            }
            let mut b = Budget::new(limits)?;
            b.work(7, nrz_bindings::readings::algebra::Location::new("caller"))?;
            let result = plan.evaluate_batch(inputs, context, &mut b);
            if limit == ceiling {
                assert_eq!(result?.len(), 3);
                assert_eq!(b.usage().work, 10);
            } else {
                assert_eq!(
                    result.expect_err("ceiling").kind,
                    FaultKind::Exhausted {
                        reason: nrz_bindings::readings::algebra::Exhaustion::Limit,
                        budget: kind,
                        limit,
                        needed: ceiling
                    }
                );
                assert_eq!(b.usage().work, 7);
            }
        }
    }
    Ok(())
}

/// Inventory derivation stated beside the resource control, independent of production estimators.
fn exp_owned(plan: &PreparedExp<'_>) -> TestResult<u64> {
    let authority = plan.basis();
    let proof = authority.proof();
    let mut owned = size_of::<PreparedExp<'_>>() as u64
        + proof.scope.len() as u64
        + authority.statement_key().len() as u64
        + 1024
        + 32 * proof.coefficients.len() as u64
        + 256 * authority.request().original.coordinates.len() as u64;
    for q in [
        &proof.absolute_tail,
        &proof.relative_tail,
        &proof.import_ratio,
        &proof.central_ratio,
        &proof.endpoint_ratio,
        &proof.restoration_error,
    ]
    .into_iter()
    .chain(proof.stages.iter().flat_map(|s| [&s.magnitude, &s.error]))
    .chain(
        authority
            .request()
            .original
            .coordinates
            .iter()
            .flat_map(|c| [c.interval.lower(), c.interval.upper()]),
    ) {
        let text = q.exact_string(&mut ledger()?)?;
        let (numerator, denominator) = text.split_once('/').unwrap_or((&text, "1"));
        owned += 128 + 4 * (decimal_bytes(numerator) + decimal_bytes(denominator));
    }
    Ok(owned)
}

/// The complete constructor output boundary is the maximum of its independent
/// returned inventory and the graph's existing judgment registration. The latter
/// can dominate; the separate unit control isolates the final output guard.
/// # Panics
/// Panics if the complete output ceiling differs from the independent inventory.
/// ´claim:matrix:prepared-exp-constructor-output´
/// ´test:integration:prepared-exp-constructor-output´
#[test]
fn prepared_exp_constructor_output() -> TestResult {
    let mut f = Fixture::new()?;
    let r = request(&mut f, Rational::zero(), 40)?;
    let context = ArithmeticContext::new()?;
    let plan = PreparedExp::new(
        &mut f.cas,
        &r,
        [0.0, 0.0],
        Policy::default(),
        &context,
        &mut ledger()?,
        &mut Counts::default(),
    )
    .map_err(|e| e.to_string())?;
    let owned = exp_owned(&plan)?;
    drop(plan);
    let needed = owned.max(f.cas.retained_judgment_bytes());
    for limit in [needed, needed - 1] {
        let mut limits = ledger()?.limits().clone();
        limits.output_bytes = limit;
        let mut budget = Budget::new(limits)?;
        budget.work(7, nrz_bindings::readings::algebra::Location::new("caller"))?;
        let result = PreparedExp::new(
            &mut f.cas,
            &r,
            [0.0, 0.0],
            Policy::default(),
            &context,
            &mut budget,
            &mut Counts::default(),
        );
        if limit == needed {
            drop(result.map_err(|e| e.to_string())?);
        } else {
            let PreparedExpError::Admission(PreparedExpm1Error::Exact(fault)) =
                result.expect_err("complete returned ownership")
            else {
                panic!("typed output cause");
            };
            assert_eq!(
                fault.kind,
                FaultKind::Exhausted {
                    reason: nrz_bindings::readings::algebra::Exhaustion::Limit,
                    budget: BudgetKind::OutputBytes,
                    limit,
                    needed
                }
            );
        }
        assert!(budget.usage().work > 7);
    }
    Ok(())
}
