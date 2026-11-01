// SPDX-License-Identifier: AGPL-3.0-only
//! Mapped physical copies retain original source columns and independent correlated-model oracles.

use super::super::{reciprocal_statistics as shared, selected_joint};
use crate::reading_testing::{TestResult, ledger};
use crate::reciprocal_testing::oracle::Rational;
use crate::spectral_testing::Fixture;
use nrz_bindings::{
    Budget,
    readings::{ReadingSource, ReadingSourceMap},
};
use nrz_matrix::{
    CompanionRef, CompanionSetDraft, CovarianceBlock, DependencyPolicy, Extent, Heap, HeapLimits,
    JointLimits, JointModel, LcSourceBinding, ReadingBinding, ReadingRegion,
    ReciprocalDerivativeContract, Source, SourceFactor,
};
use nrz_scalar::{ArithmeticContext, Scalar};
use std::rc::Rc;

/// One common original source declaration and all its selected physical copies.
pub(super) struct Case<P: Scalar> {
    /// Actual numerical owner of all current source and target versions.
    pub(super) heap: Heap,
    /// Original source, signed expansion, copied and overlapping bindings in declared order.
    pub(super) readings: Vec<ReadingBinding<P>>,
    /// Complete native joint declaration over the two original scalar sources.
    pub(super) companion: CompanionRef<P>,
}

/// The model is either rank one or full rank, with an explicit missing-cross twin.
pub(super) fn prepare<P: Scalar + 'static>(
    fixture: &Fixture,
    full: bool,
    known: bool,
) -> TestResult<Case<P>> {
    let mut heap = Heap::new(HeapLimits::default())?;
    let readings = import(fixture, &mut heap)?;
    let mut selected = Vec::new();
    for index in [0, 4] {
        selected.push(heap.select_reading(&readings[index], &mut ledger()?)?);
    }
    let handles = selected_joint::capture_handles(&mut heap, &selected)?;
    let sources = selected
        .iter()
        .map(|value| Source::new(value.symbol().descriptor(), 1))
        .collect::<Result<Vec<_>, _>>()?;
    let one = P::ONE;
    let d128 = P::from_u8(128);
    let mut factors = vec![one / (d128 * P::from_u8(2)), one / (d128 * P::from_u8(4))];
    let mut weights = vec![one];
    if full {
        factors.extend([one / (d128 * P::from_u8(4)), P::ZERO]);
        weights.push(one);
    }
    let limits = JointLimits::default();
    let cross = if known {
        CovarianceBlock::Known
    } else {
        CovarianceBlock::Unknown
    };
    let model = JointModel::from_factor(
        &sources,
        SourceFactor::new(2, &factors, &weights, limits)?,
        &[
            CovarianceBlock::Known,
            cross.clone(),
            cross,
            CovarianceBlock::Known,
        ],
        limits,
    )?;
    let companion = heap.publish_joint(
        CompanionSetDraft::new(
            model,
            selected_joint::capture_targets(&heap, handles)?,
            [vec![0], vec![1]],
        ),
        limits,
    )?;
    Ok(Case {
        heap,
        readings,
        companion,
    })
}

/// Exact dyadic centers satisfy the declared signed and scaled copy equations.
pub(super) fn import<P: Scalar + 'static>(
    fixture: &Fixture,
    heap: &mut Heap,
) -> TestResult<Vec<ReadingBinding<P>>> {
    import_with(fixture, heap, &regions::<P>()?, &mut ledger()?)
}

/// Complete source and selected-copy storage, independently mutable for refusal twins.
pub(super) fn regions<P: Scalar>() -> TestResult<[ReadingRegion<P>; 4]> {
    let l = P::ONE / P::from_u8(4);
    let c = P::ONE / P::from_u8(16);
    Ok([
        ReadingRegion {
            extent: Extent::new(1, 1)?,
            values: vec![l / P::from_u8(2)],
        },
        ReadingRegion {
            extent: Extent::new(1, 1)?,
            values: vec![-c],
        },
        ReadingRegion {
            extent: Extent::new(3, 1)?,
            values: vec![l, -l, l],
        },
        ReadingRegion {
            extent: Extent::new(3, 1)?,
            values: vec![c, -c, c],
        },
    ])
}

/// Caller-ledger import makes the exact-copy checks and their limits observable.
pub(super) fn import_with<P: Scalar + 'static>(
    fixture: &Fixture,
    heap: &mut Heap,
    regions: &[ReadingRegion<P>],
    budget: &mut Budget,
) -> TestResult<Vec<ReadingBinding<P>>> {
    Ok(heap
        .import_readings(
            Rc::clone(&fixture.graph),
            fixture.coverage.clone(),
            Vec::new(),
            regions,
            DependencyPolicy::WholeBase,
            budget,
        )?
        .bindings()
        .to_vec())
}

/// Keep the exact source order opposite to the native model order.
pub(super) fn binding<P: Scalar + 'static>(
    fixture: &Fixture,
    contract: &ReciprocalDerivativeContract,
    case: &Case<P>,
    indices: [usize; 2],
    budget: &mut Budget,
) -> TestResult<LcSourceBinding<P>> {
    let targets = indices.map(|index| case.readings[index].clone());
    let sources = vec![case.readings[4].clone(), case.readings[0].clone()];
    let mut declared = Vec::new();
    for source in &sources {
        declared.push(ReadingSource::new(
            source.reading().clone(),
            &mut ledger()?,
        )?);
    }
    let map = ReadingSourceMap::new(
        &fixture.graph,
        declared,
        targets.clone().map(|target| target.reading().clone()),
        &mut ledger()?,
    )?;
    let derivative = case.heap.evaluate_reciprocal_derivative(
        contract,
        &targets,
        &ArithmeticContext::new()?,
        &mut ledger()?,
    )?;
    Ok(LcSourceBinding::new(
        &case.heap,
        fixture.plan(targets, &mut ledger()?)?,
        derivative,
        map,
        sources,
        case.companion,
        budget,
    )?)
}

/// Both formats cover negative ancestry, selected copies, overlapping aliases and singular/full-rank models.
/// ´test:integration:lc-source-composition-mapped-copied-overlapping-models´
#[test]
fn lc_source_composition_mapped_copied_overlapping_models() -> TestResult {
    let mut budget = shared::policy()?;
    let mut fixture = Fixture::with_mapped_sources(budget.limits().clone())?;
    let contract = shared::derivative(&mut fixture, &mut budget)?;
    for full in [false, true] {
        check::<f32>(&fixture, &contract, full)?;
        check::<f64>(&fixture, &contract, full)?;
    }
    Ok(())
}

/// Exercise distinct stored copies and overlapping aliases without changing original source identity.
fn check<P: Scalar + 'static>(
    fixture: &Fixture,
    contract: &ReciprocalDerivativeContract,
    full: bool,
) -> TestResult {
    let case = prepare::<P>(fixture, full, true)?;
    let context = ArithmeticContext::new()?;
    for indices in [[1, 5], [3, 7], [8, 9]] {
        let binding = binding(fixture, contract, &case, indices, &mut ledger()?)?;
        assert_eq!(binding.source_columns(), [1, 0]);
        let maps = binding.source_map().maps();
        assert_eq!(
            maps[0].coefficients(),
            crate::reading_testing::map(2, 1, &[0, 2], &[0])?.coefficients()
        );
        assert_eq!(
            maps[1].coefficients(),
            crate::reading_testing::map(2, 1, &[-1, 0], &[0])?.coefficients()
        );
        let result = case
            .heap
            .evaluate_lc_source_statistics(&binding, &context, &mut ledger()?)?;
        oracle(result.covariance(&case.heap, &context)?, full)?;
        assert!(std::ptr::eq(
            binding.root_jacobian().operator().native(),
            contract.exact().native()
        ));
    }
    Ok(())
}

/// Independently contract the two rational factor directions in original source order.
pub(super) fn oracle<P: Scalar>(
    values: &[[nrz_scalar::Enclosure<P>; 8]; 8],
    full: bool,
) -> TestResult {
    let first = [
        (0, 1),
        (1, 320),
        (1, 200_000),
        (0, 1),
        (0, 1),
        (0, 1),
        (0, 1),
        (0, 1),
    ];
    let second = [
        (0, 1),
        (1, 640),
        (0, 1),
        (0, 1),
        (0, 1),
        (-1, 16),
        (0, 1),
        (1, 16),
    ];
    for row in 0..8 {
        for column in 0..8 {
            let (first_row, first_row_den) = first[row];
            let (first_column, first_column_den) = first[column];
            let (second_row, second_row_den) = second[row];
            let (second_column, second_column_den) = second[column];
            let numerator = first_row * first_column * second_row_den * second_column_den
                + if full {
                    second_row * second_column * first_row_den * first_column_den
                } else {
                    0
                };
            let denominator = u64::try_from(
                first_row_den * first_column_den * second_row_den * second_column_den,
            )?;
            let expected = Rational::new(numerator, denominator)?;
            assert!(
                expected.enclosed(
                    values[row][column].lower().get().to_f64(),
                    values[row][column].upper().get().to_f64()
                )?,
                "row {row}, column {column}"
            );
        }
    }
    Ok(())
}
