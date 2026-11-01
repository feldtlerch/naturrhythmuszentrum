// SPDX-License-Identifier: AGPL-3.0-only
//! Literal child and parent ceilings preserve prior spend and atomic output on refusal.

use super::{TestResult, fixture, ledger, pi};
use nrz_bindings::{Budget, Limits, LimitsOverride, readings::algebra::Location};
use nrz_matrix::PiCoordinateContract;
use nrz_scalar::{ArithmeticContext, Scalar};

/// Both formats pin cold graph owners and independent live consumer schedules.
/// ´test:integration:pi-live-coordinate-owner-boundaries´
#[test]
fn pi_live_coordinate_owner_boundaries() -> TestResult {
    let plan = fixture::plan()?;
    boundaries::<f32>(
        &plan,
        [210_365, 31_140, 5401],
        [97_339, 79_782],
        [116_787, 368],
        [80_454, 80_422, 80_918],
        [81_070, 81_062, 81_534],
        [210_364, 30_173, 38_411],
    )?;
    boundaries::<f64>(
        &plan,
        [210_365, 31_140, 5401],
        [97_339, 79_782],
        [116_947, 496],
        [80_758, 80_606, 81_382],
        [81_534, 81_430, 82_158],
        [210_364, 30_173, 38_411],
    )
}

/// The affine origin changes exact owner payload while numerical arrays keep their original schedule.
/// ´test:integration:pi-live-coordinate-point-owner-boundaries´
#[test]
fn pi_live_coordinate_point_owner_boundaries() -> TestResult {
    let plan = super::point::point_plan()?;
    boundaries::<f32>(
        &plan,
        [299_724, 33_973, 5979],
        [101_336, 82_207],
        [122_623, 368],
        [82_879, 82_847, 83_343],
        [83_495, 83_487, 83_959],
        [299_723, 33_016, 42_117],
    )?;
    boundaries::<f64>(
        &plan,
        [299_724, 33_973, 5979],
        [101_336, 82_207],
        [122_783, 496],
        [83_183, 83_031, 83_807],
        [83_959, 83_855, 84_583],
        [299_723, 33_016, 42_117],
    )
}

/// Authentication and exact coefficients are cold owners; subsequent consumers share the same arrays.
fn boundaries<P: Scalar + 'static>(
    plan: &super::state_fixture::Plan,
    exact: [u64; 3],
    evidence: [u64; 2],
    certificate: [u64; 2],
    normalization: [u64; 3],
    restoration: [u64; 3],
    failed_work: [u64; 3],
) -> TestResult {
    let context = ArithmeticContext::new()?;
    let circle = pi::<P>(&context, &mut ledger()?)?;
    boundary(
        |policy, budget| {
            let contract = PiCoordinateContract::new(
                &plan.graph,
                &plan.contract,
                &circle,
                policy,
                &context,
                budget,
            )?;
            assert_eq!(contract.owned_bytes(), certificate[1]);
            Ok(())
        },
        exact,
        [exact[0], certificate[0], certificate[1]],
        [failed_work, [failed_work[0], 0, 0]],
    )?;
    let contract = PiCoordinateContract::new(
        &plan.graph,
        &plan.contract,
        &circle,
        &policy(exact),
        &context,
        &mut ledger()?,
    )?;
    let (heap, source) = fixture::imported::<P>(plan)?;
    let normalize = [3248, normalization[0], normalization[1]];
    boundary(
        |policy, budget| {
            heap.normalize_certified_reading(&source, &contract, policy, &context, budget)?;
            Ok(())
        },
        normalize,
        [3248, normalization[2], normalization[1]],
        [[0; 3]; 2],
    )?;
    let normalized = heap.normalize_certified_reading(
        &source,
        &contract,
        &policy(normalize),
        &context,
        &mut ledger()?,
    )?;
    boundary(
        |policy, budget| {
            heap.restore_certified_reading(&normalized, &contract, policy, &context, budget)?;
            Ok(())
        },
        [177, restoration[0], restoration[1]],
        [177, restoration[2], restoration[1]],
        [[0; 3]; 2],
    )?;
    assert_eq!(
        plan.contract.normalization().retained_payload_bytes()
            + plan.contract.restoration().retained_payload_bytes(),
        evidence[1]
    );
    let mut authentication = ledger()?;
    for image in [plan.contract.normalization(), plan.contract.restoration()] {
        plan.graph
            .validate_reuse_key(image.reuse_key(), &mut authentication)?;
    }
    assert_eq!(authentication.usage().work, evidence[0]);
    Ok(())
}

/// Every resource has exactly one successful literal boundary and one failure immediately below it.
fn boundary(
    mut attempt: impl FnMut(&LimitsOverride, &mut Budget) -> TestResult,
    child: [u64; 3],
    parent: [u64; 3],
    failed_work: [[u64; 3]; 2],
) -> TestResult {
    let mut cumulative = parent;
    cumulative[0] += 17;
    let mut budget = prior(cumulative)?;
    attempt(&policy(child), &mut budget)?;
    assert_eq!(budget.usage().work, cumulative[0]);
    for (owner, receipts) in failed_work.into_iter().enumerate() {
        for (resource, spent) in receipts.into_iter().enumerate() {
            let mut child_limit = child;
            let mut parent_limit = cumulative;
            if owner == 0 {
                child_limit[resource] -= 1;
            } else {
                parent_limit[resource] -= 1;
            }
            let mut budget = prior(parent_limit)?;
            let before = budget.usage();
            assert!(
                attempt(&policy(child_limit), &mut budget).is_err(),
                "owner {owner}, resource {resource}"
            );
            assert_eq!(budget.usage().work, before.work + spent);
        }
    }
    Ok(())
}

/// Resolve literal explicit allowances without changing any production default.
fn policy(profile: [u64; 3]) -> LimitsOverride {
    LimitsOverride {
        work: Some(profile[0]),
        staged_payload_bytes: Some(profile[1]),
        output_bytes: Some(profile[2]),
        ..LimitsOverride::default()
    }
}

/// A nonzero caller prefix exposes accidental commits on every failed child attempt.
fn prior(profile: [u64; 3]) -> TestResult<Budget> {
    let mut budget = Budget::new(Limits::default().resolve(&policy(profile))?)?;
    budget.work(17, Location::new("prior live coordinate caller"))?;
    Ok(budget)
}

/// Repeated consumers pay only their live schedule while both coefficient arrays stay shared.
/// ´test:integration:pi-live-coordinate-cumulative-reuse-boundary´
#[test]
fn pi_live_coordinate_cumulative_reuse_boundary() -> TestResult {
    cumulative::<f32>()?;
    cumulative::<f64>()
}

/// One cold admission, two normalizations and one restoration retain every earlier Work charge.
fn cumulative<P: Scalar + 'static>() -> TestResult {
    let plan = fixture::plan()?;
    let context = ArithmeticContext::new()?;
    let circle = pi::<P>(&context, &mut ledger()?)?;
    for limit in [217_055, 217_054] {
        let mut budget = prior([limit, 116_947, 81_430])?;
        let contract = PiCoordinateContract::new(
            &plan.graph,
            &plan.contract,
            &circle,
            &policy([210_365, 31_140, 5401]),
            &context,
            &mut budget,
        )?;
        let (heap, source) = fixture::imported::<P>(&plan)?;
        let normalized = heap.normalize_certified_reading(
            &source,
            &contract,
            &fixture::execution(),
            &context,
            &mut budget,
        )?;
        let warm = heap.normalize_certified_reading(
            &source,
            &contract,
            &fixture::execution(),
            &context,
            &mut budget,
        )?;
        assert!(std::ptr::eq(
            normalized.coefficients().as_ptr(),
            warm.coefficients().as_ptr()
        ));
        assert_eq!(budget.usage().work, 216_878);
        let before = budget.usage();
        let result = heap.restore_certified_reading(
            &normalized,
            &contract,
            &fixture::execution(),
            &context,
            &mut budget,
        );
        if limit == 217_055 {
            result?;
            assert_eq!(budget.usage().work, limit);
        } else {
            assert!(result.is_err());
            assert_eq!(budget.usage(), before);
        }
    }
    Ok(())
}
