// SPDX-License-Identifier: AGPL-3.0-only
//! Plausible shapes, reordered axes and changed cycle meanings cannot authorize the connected bundle.

use super::{Prepared, TestResult, inputs, prepared, publication, shared, symbols};
use nrz_bindings::readings::{
    EntryMeaning, LcFrequencyDefinition, PhysicalReading, ReadingDefinition, Representation,
    RepresentationDefinition,
    algebra::{
        Cas, Rational,
        readings::{AffineMap, EntryIndex, SelectionSpec},
    },
};
use nrz_bindings::{
    Budget, Descriptor, Kind, Measure, OriginDefinition, ReferenceDefinition, Role, Scale,
};
use nrz_scalar::{ArithmeticContext, Scalar};
use std::rc::Rc;

/// Full-cycle magnitude, sign and every covariance/error target are checked before publication.
/// ´test:integration:lc-frequency-full-cycle-sign-target-refusals´
#[test]
fn lc_frequency_full_cycle_sign_target_refusals() -> TestResult {
    let mut prepared = prepared()?;
    let mut budget = shared::policy()?;
    mutations(&mut prepared, &mut budget)?;
    cycles(&prepared, &mut budget)?;
    let graph = Rc::get_mut(&mut prepared.fixture.graph).ok_or("unique mutation graph")?;
    let reversed = symbols::proposed(
        graph,
        prepared.fixture.law.clone(),
        &prepared.declarations,
        true,
        &mut budget,
    )?;
    assert!(
        LcFrequencyDefinition::new(
            graph,
            reversed.angular,
            reversed.cycle,
            reversed.symbols,
            &prepared.declarations,
            &mut budget
        )
        .is_err()
    );
    positive::<f32>(&prepared)?;
    positive::<f64>(&prepared)
}

fn mutations(prepared: &mut Prepared, budget: &mut Budget) -> TestResult {
    let graph = Rc::get_mut(&mut prepared.fixture.graph).ok_or("unique mutation graph")?;
    let definition = &prepared.definition;
    for mutation in 0..11 {
        let mut symbols = definition.symbols().clone();
        match mutation {
            0 => symbols.companions.swap(0, 1),
            1 => symbols.companions[2] = symbols.cross_covariance.clone(),
            2 => symbols.cross_covariance = symbols.companions[2].clone(),
            3 => symbols.cross_covariance = definition.angular().symbols().companions[2].clone(),
            4 => symbols.cross_error = symbols.covariance_error.clone(),
            5 => symbols.cross_error_value = symbols.covariance_error_value.clone(),
            6 => symbols.cyclic = definition.angular().symbols().value.clone(),
            7 => symbols.covariance_error_value = symbols.cross_error_value.clone(),
            8 => symbols.cross_error = definition.angular().symbols().covariance_error.clone(),
            9 => symbols.cross_error_value = symbols.cyclic.clone(),
            10 => symbols.cyclic = negative(graph, &symbols.cyclic, budget)?,
            _ => return Err("connected declaration mutation".into()),
        }
        assert!(
            LcFrequencyDefinition::new(
                graph,
                definition.angular().clone(),
                definition.cycle().clone(),
                symbols,
                &prepared.declarations,
                budget
            )
            .is_err(),
            "mutation {mutation}"
        );
    }
    Ok(())
}

/// This is a valid signed representation; the closed output requires its exact positive identity map.
fn negative(
    graph: &mut Cas,
    original: &Representation,
    budget: &mut Budget,
) -> TestResult<Representation> {
    let map = AffineMap::new(
        1,
        1,
        vec![Rational::from_ratio(-1, 1, budget)?],
        vec![Rational::zero()],
        budget,
    )?;
    let original = original.reading();
    let reading = PhysicalReading::define(
        graph,
        original.family(),
        ReadingDefinition {
            symbol: original.symbol().clone(),
            selection: SelectionSpec::Entry(EntryIndex { row: 0, column: 0 }),
            map: map.clone(),
            entries: original.entries().to_vec(),
        },
        budget,
    )?;
    Ok(Representation::prove(
        graph,
        &reading,
        RepresentationDefinition::Defined {
            symbol: original.symbol().clone(),
            expansion: map,
        },
        budget,
    )?)
}

fn cycles(prepared: &Prepared, budget: &mut Budget) -> TestResult {
    let definition = &prepared.definition;
    let angle = definition.cycle().descriptor();
    let two = Rational::from_ratio(2, 1, budget)?;
    for magnitude in [
        definition.angular().radian().magnitude().clone(),
        definition
            .cycle()
            .magnitude()
            .div(&two.clone().into(), budget)?,
        definition.cycle().magnitude().mul(&two.into(), budget)?,
    ] {
        let cycle = EntryMeaning::new(
            angle.clone(),
            ReferenceDefinition::new("changed cycle", angle, Scale::Expression(magnitude))?,
            None,
            budget,
        )?;
        assert!(
            LcFrequencyDefinition::new(
                &prepared.fixture.graph,
                definition.angular().clone(),
                cycle,
                definition.symbols().clone(),
                &prepared.declarations,
                budget
            )
            .is_err()
        );
    }
    let point = Descriptor::new(Kind::Angle, Measure::coherent(), Role::Point)?;
    let cycle = EntryMeaning::new(
        point.clone(),
        definition.cycle().reference().clone(),
        Some(OriginDefinition::new("cycle point origin", &point)?),
        budget,
    )?;
    assert!(
        LcFrequencyDefinition::new(
            &prepared.fixture.graph,
            definition.angular().clone(),
            cycle,
            definition.symbols().clone(),
            &prepared.declarations,
            budget
        )
        .is_err()
    );
    Ok(())
}

fn positive<P: Scalar + 'static>(prepared: &Prepared) -> TestResult {
    let mut budget = shared::policy()?;
    let mut case = inputs::nominal::<P>(&prepared.fixture, &prepared.derivative, &mut budget)?;
    let plan = publication(prepared, &case, &mut budget)?;
    case.heap
        .publish_lc_frequency(&plan, &ArithmeticContext::new()?, &mut budget)?;
    Ok(())
}

/// Foreign numerical evidence and a substituted same-kind map publish no connected output prefix.
/// ´test:integration:lc-frequency-map-and-evaluation-refusals´
#[test]
fn lc_frequency_map_and_evaluation_refusals() -> TestResult {
    let prepared = prepared()?;
    substituted::<f32>(&prepared)?;
    substituted::<f64>(&prepared)
}

fn substituted<P: Scalar + 'static>(prepared: &Prepared) -> TestResult {
    let mut setup = shared::policy()?;
    let mut case = inputs::nominal::<P>(&prepared.fixture, &prepared.derivative, &mut setup)?;
    let foreign = inputs::nominal::<P>(&prepared.fixture, &prepared.derivative, &mut setup)?;
    let wrong = publication(prepared, &foreign, &mut setup)?;
    let context = ArithmeticContext::new()?;
    let before = case.heap.provenance_usage();
    let stamps = case.heap.statistical(case.companion)?.targets();
    assert!(
        case.heap
            .publish_lc_frequency(&wrong, &context, &mut shared::policy()?)
            .is_err()
    );
    assert_eq!(case.heap.provenance_usage(), before);
    assert_eq!(case.heap.statistical(case.companion)?.targets(), stamps);
    let plan = publication(prepared, &case, &mut setup)?;
    let descriptor = prepared.definition.angular().descriptor();
    let shape = crate::conversion_testing::shape(&prepared.declarations, descriptor, 1)?;
    let changed = nrz_matrix::ConversionContract::new(
        shape.clone(),
        shape,
        prepared
            .declarations
            .conversion_map(descriptor, descriptor)?,
        nrz_scalar::LiteralLimits::default(),
        nrz_matrix::ConversionMode::Deterministic,
    )?;
    let pi = crate::integration::pi_conversion::pi::<P>(&context, &mut setup)?;
    assert!(
        nrz_matrix::LcFrequencyPublication::new(
            plan.evaluation().clone(),
            prepared.definition.clone(),
            &changed,
            &pi,
            &crate::integration::pi_conversion::exact_policy(),
            &context,
            &mut setup
        )
        .is_err()
    );
    assert_eq!(case.heap.provenance_usage(), before);
    assert_eq!(case.heap.statistical(case.companion)?.targets(), stamps);
    // The identical output inventory remains available after both failures.
    let output = case
        .heap
        .publish_lc_frequency(&plan, &context, &mut setup)?;
    output.covariance(&case.heap, &context)?;
    Ok(())
}
