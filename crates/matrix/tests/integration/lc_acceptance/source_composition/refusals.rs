// SPDX-License-Identifier: AGPL-3.0-only
//! Equal centers and repeated exact maps never substitute the selected source owners.

use super::{super::reciprocal_statistics as shared, mapped};
use crate::reading_testing::{TestResult, ledger};
use crate::spectral_testing::Fixture;
use nrz_matrix::{LcSourceBinding, ReciprocalDerivativeContract, SpectralError};
use nrz_scalar::{ArithmeticContext, Scalar};

/// Both formats reject equal-center imports, omitted cross blocks and stale source or selected-copy versions.
/// ´test:integration:lc-source-composition-identity-and-version-refusals´
#[test]
fn lc_source_composition_identity_and_version_refusals() -> TestResult {
    let mut budget = shared::policy()?;
    let mut fixture = Fixture::with_mapped_sources(budget.limits().clone())?;
    let contract = shared::derivative(&mut fixture, &mut budget)?;
    check::<f32>(&fixture, &contract)?;
    check::<f64>(&fixture, &contract)
}

/// Changed or substituted owners cannot be reused through equal centers.
fn check<P: Scalar + 'static>(
    fixture: &Fixture,
    contract: &ReciprocalDerivativeContract,
) -> TestResult {
    let unknown = mapped::prepare::<P>(fixture, false, false)?;
    assert!(mapped::binding(fixture, contract, &unknown, [1, 5], &mut ledger()?).is_err());
    for changed in [0, 4] {
        let mut case = mapped::prepare::<P>(fixture, false, true)?;
        let binding = mapped::binding(fixture, contract, &case, [1, 5], &mut ledger()?)?;
        let context = ArithmeticContext::new()?;
        let result = case
            .heap
            .evaluate_lc_source_statistics(&binding, &context, &mut ledger()?)?;
        let values = case.heap.reading_entries(&case.readings[changed])?;
        case.heap
            .replace_reading_entries(&case.readings[changed], &values, &mut ledger()?)?;
        assert!(result.covariance(&case.heap, &context).is_err());
        assert!(result.sensitivities(&case.heap, &context).is_err());
    }
    let mut case = mapped::prepare::<P>(fixture, false, true)?;
    let original = mapped::binding(fixture, contract, &case, [1, 5], &mut ledger()?)?;
    case.readings = mapped::import(fixture, &mut case.heap)?;
    assert!(mapped::binding(fixture, contract, &case, [1, 5], &mut ledger()?).is_err());
    case.heap.evaluate_lc_source_statistics(
        &original,
        &ArithmeticContext::new()?,
        &mut ledger()?,
    )?;
    Ok(())
}

/// An equal-valued copied target needs its own operating-point evaluation and exact selected map.
/// ´test:integration:lc-source-composition-refuses-substituted-selected-copy´
#[test]
fn lc_source_composition_refuses_substituted_selected_copy() -> TestResult {
    let mut budget = shared::policy()?;
    let mut fixture = Fixture::with_mapped_sources(budget.limits().clone())?;
    let contract = shared::derivative(&mut fixture, &mut budget)?;
    copied::<f32>(&fixture, &contract)?;
    copied::<f64>(&fixture, &contract)
}

/// Only the derivative evaluated on the exact chosen copy may enter its plan.
fn copied<P: Scalar + 'static>(
    fixture: &Fixture,
    contract: &ReciprocalDerivativeContract,
) -> TestResult {
    let case = mapped::prepare::<P>(fixture, true, true)?;
    let bound = mapped::binding(fixture, contract, &case, [1, 5], &mut ledger()?)?;
    for indices in [[3, 7], [8, 9]] {
        let targets = indices.map(|index| case.readings[index].clone());
        let derivative = case.heap.evaluate_reciprocal_derivative(
            contract,
            &targets,
            &ArithmeticContext::new()?,
            &mut ledger()?,
        )?;
        let result = LcSourceBinding::new(
            &case.heap,
            fixture.plan(
                [case.readings[1].clone(), case.readings[5].clone()],
                &mut ledger()?,
            )?,
            derivative,
            bound.source_map().clone(),
            vec![case.readings[4].clone(), case.readings[0].clone()],
            case.companion,
            &mut ledger()?,
        );
        assert!(matches!(result, Err(SpectralError::Identity)));
    }
    Ok(())
}

/// A fixed physical derivative direction retains the existing named root-adapter refusal in both formats.
/// ´test:integration:lc-source-composition-retains-fixed-direction-refusal´
#[test]
fn lc_source_composition_retains_fixed_direction_refusal() -> TestResult {
    use nrz_bindings::readings::{
        ReciprocalJacobianCache, ReciprocalJacobianKey, SpectralLawError,
    };
    let mut budget = shared::policy()?;
    let mut fixture = Fixture::with_mapped_sources(budget.limits().clone())?;
    let graph = std::rc::Rc::get_mut(&mut fixture.graph).ok_or("original graph")?;
    let key =
        ReciprocalJacobianKey::new(graph, fixture.law.operator(), &["C".into()], &mut budget)?;
    let exact = ReciprocalJacobianCache::new().get_or_derive(graph, &key, &mut budget)?;
    let contract = ReciprocalDerivativeContract::new(
        graph,
        exact,
        nrz_scalar::LiteralLimits::default(),
        &mut budget,
    )?;
    let single = mapped::prepare::<f32>(&fixture, false, true)?;
    let double = mapped::prepare::<f64>(&fixture, false, true)?;
    for error in [
        mapped::binding(&fixture, &contract, &single, [1, 5], &mut ledger()?)
            .err()
            .ok_or("fixed f32 refusal")?,
        mapped::binding(&fixture, &contract, &double, [1, 5], &mut ledger()?)
            .err()
            .ok_or("fixed f64 refusal")?,
    ] {
        assert!(matches!(
            error.downcast_ref::<SpectralError>(),
            Some(SpectralError::Law(SpectralLawError::Transport))
        ));
    }
    Ok(())
}

/// Every stored affine copy must satisfy its original equation; partial expanded writes still refuse.
/// ´test:integration:lc-source-composition-exact-copy-admission´
#[test]
fn lc_source_composition_exact_copy_admission() -> TestResult {
    let mut budget = shared::policy()?;
    let mut fixture = Fixture::with_mapped_sources(budget.limits().clone())?;
    let contract = shared::derivative(&mut fixture, &mut budget)?;
    copies::<f32>(&fixture, &contract)?;
    copies::<f64>(&fixture, &contract)
}

/// Refusal is atomic and a complete retry uses the same heap without a changed limit.
fn copies<P: Scalar + 'static>(
    fixture: &Fixture,
    contract: &ReciprocalDerivativeContract,
) -> TestResult {
    for (region, entry) in [
        (0, 0),
        (1, 0),
        (2, 0),
        (2, 1),
        (2, 2),
        (3, 0),
        (3, 1),
        (3, 2),
    ] {
        let mut heap = nrz_matrix::Heap::new(nrz_matrix::HeapLimits::default())?;
        let mut regions = mapped::regions::<P>()?;
        regions[region].values[entry] = -regions[region].values[entry];
        let error = mapped::import_with(fixture, &mut heap, &regions, &mut ledger()?)
            .err()
            .ok_or("inconsistent copy must refuse")?;
        assert!(matches!(
            error.downcast_ref::<nrz_matrix::LiveBindingError>(),
            Some(nrz_matrix::LiveBindingError::UncoveredCopy)
        ));
        mapped::import::<P>(fixture, &mut heap)?;
    }
    let mut case = mapped::prepare::<P>(fixture, false, true)?;
    let binding = mapped::binding(fixture, contract, &case, [1, 5], &mut ledger()?)?;
    let context = ArithmeticContext::new()?;
    let result = case
        .heap
        .evaluate_lc_source_statistics(&binding, &context, &mut ledger()?)?;
    for index in [1, 3, 5, 7] {
        let values = case.heap.reading_entries(&case.readings[index])?;
        assert!(matches!(
            case.heap
                .replace_reading_entries(&case.readings[index], &values, &mut ledger()?),
            Err(nrz_matrix::LiveBindingError::Heap(
                nrz_matrix::HeapError::MissingCoverage
            ))
        ));
        result.covariance(&case.heap, &context)?;
    }
    Ok(())
}

/// Exact centers cannot validate a copied radius; refusals publish nothing in either format.
/// ´test:integration:lc-source-composition-refuses-copy-companions´
#[test]
fn lc_source_composition_refuses_copy_companions() -> TestResult {
    let budget = shared::policy()?;
    let fixture = Fixture::with_mapped_copy_companions(budget.limits().clone())?;
    radii::<f32>(&fixture)?;
    radii::<f64>(&fixture)
}

/// Understated and ample copy radii both refuse the deliberately companion-free copy route.
fn radii<P: Scalar + 'static>(fixture: &Fixture) -> TestResult {
    use nrz_matrix::{
        DependencyPolicy, Extent, Heap, HeapError, HeapLimits, LiveBindingError, ReadingRegion,
    };
    use std::rc::Rc;
    let original = P::ONE / P::from_u8(64);
    for copied in [P::ZERO, P::ONE / P::from_u8(128), P::ONE / P::from_u8(16)] {
        let mut regions = mapped::regions::<P>()?.to_vec();
        regions.push(ReadingRegion {
            extent: Extent::new(4, 1)?,
            values: vec![original, original, copied, copied],
        });
        // Exactly the four companion-free regions and their eight values fit on retry.
        let mut heap = Heap::new(HeapLimits::new(4, 8, 4_096))?;
        let refused = heap.import_readings(
            Rc::clone(&fixture.graph),
            fixture.coverage.clone(),
            fixture.companions.clone(),
            &regions,
            DependencyPolicy::WholeBase,
            &mut ledger()?,
        );
        assert!(
            matches!(
                refused,
                Err(LiveBindingError::Heap(HeapError::MissingCoverage))
            ),
            "{refused:?}"
        );
        let readings = mapped::import::<P>(fixture, &mut heap)?;
        assert_eq!(
            heap.reading_entries(&readings[1])?,
            vec![P::ONE / P::from_u8(4)]
        );
    }
    original_radii::<P>(fixture)
}

/// Original radius declarations never silently promote a companion-free copied input's coverage.
fn original_radii<P: Scalar + 'static>(fixture: &Fixture) -> TestResult {
    use nrz_matrix::{Coverage, DependencyPolicy, Extent, Heap, HeapLimits, ReadingRegion};
    use std::rc::Rc;
    let original = P::ONE / P::from_u8(64);
    let mut regions = mapped::regions::<P>()?.to_vec();
    regions.push(ReadingRegion {
        extent: Extent::new(2, 1)?,
        values: vec![original; 2],
    });
    let mut heap = Heap::new(HeapLimits::new(5, 10, 4_096))?;
    let readings = heap.import_readings(
        Rc::clone(&fixture.graph),
        fixture.coverage.clone(),
        fixture.companions[..2].to_vec(),
        &regions,
        DependencyPolicy::WholeBase,
        &mut ledger()?,
    )?;
    let plan = fixture.plan(
        [
            readings.bindings()[1].clone(),
            readings.bindings()[5].clone(),
        ],
        &mut ledger()?,
    )?;
    let spectrum = heap.evaluate_lc_spectrum(&plan, &ArithmeticContext::new()?, &mut ledger()?)?;
    assert_eq!(spectrum.coverage(), [Coverage::RepresentedInputs; 2]);
    Ok(())
}
