// SPDX-License-Identifier: AGPL-3.0-only
//! Equal dimensions and plausible companion shapes cannot replace the sealed original output recipe.

use super::{TestResult, prepared};
use nrz_bindings::{Budget, Kind, Limits, readings::LcAngularFrequencyDefinition};

/// ´test:integration:lc-angular-frequency-definition-refusals´
#[test]
fn lc_angular_frequency_definition_refusals() -> TestResult {
    let prepared = prepared()?;
    let definition = &prepared.definition;
    for mutation in 0..3 {
        let mut symbols = definition.symbols().clone();
        match mutation {
            0 => symbols.companions.swap(0, 1),
            1 => symbols.companions[2] = symbols.companions[0].clone(),
            2 => symbols.covariance_error_value = symbols.value.clone(),
            _ => return Err("mutation".into()),
        }
        assert!(
            LcAngularFrequencyDefinition::new(
                &prepared.fixture.graph,
                definition.law().clone(),
                definition.radian().clone(),
                symbols,
                &registry()?,
                &mut Budget::new(Limits::default())?
            )
            .is_err()
        );
    }
    assert_eq!(definition.descriptor().kind(), &Kind::angular_frequency()?);
    assert_eq!(definition.rate_descriptor().kind(), &Kind::rate()?);
    assert_eq!(definition.radian().descriptor().kind(), &Kind::Angle);
    Ok(())
}

fn registry() -> TestResult<nrz_bindings::Registry> {
    let mut registry = nrz_bindings::Registry::new(nrz_bindings::ExponentPolicy::Rational);
    registry.register_binary(
        nrz_bindings::BinaryOp::Multiply,
        Kind::rate()?,
        Kind::Angle,
        Kind::angular_frequency()?,
    )?;
    Ok(registry)
}

/// ´test:integration:lc-angular-frequency-kind-role-radian-refusals´
#[test]
fn lc_angular_frequency_kind_role_radian_refusals() -> TestResult {
    use nrz_bindings::{Descriptor, Measure, Role};
    let mut prepared = prepared()?;
    let mut budget = super::shared::policy()?;
    for kind in [Kind::rate()?, Kind::frequency()?] {
        let graph = std::rc::Rc::get_mut(&mut prepared.fixture.graph).ok_or("unique graph")?;
        let proposed = crate::angular_testing::proposed(graph, kind, &mut budget)?;
        assert!(
            LcAngularFrequencyDefinition::new(
                graph,
                prepared.fixture.law.clone(),
                proposed.radian,
                proposed.symbols,
                &proposed.registry,
                &mut budget
            )
            .is_err()
        );
    }
    let graph = std::rc::Rc::get_mut(&mut prepared.fixture.graph).ok_or("unique graph")?;
    let point_output =
        Descriptor::new(Kind::angular_frequency()?, Measure::coherent(), Role::Point)?;
    let proposed = crate::angular_testing::proposed_descriptor(graph, point_output, &mut budget)?;
    assert!(
        LcAngularFrequencyDefinition::new(
            graph,
            prepared.fixture.law.clone(),
            proposed.radian,
            proposed.symbols,
            &proposed.registry,
            &mut budget
        )
        .is_err()
    );
    let definition = &prepared.definition;
    for radian in invalid_radians(definition, &mut budget)? {
        assert!(
            LcAngularFrequencyDefinition::new(
                &prepared.fixture.graph,
                definition.law().clone(),
                radian,
                definition.symbols().clone(),
                &registry()?,
                &mut budget
            )
            .is_err()
        );
    }
    assert!(
        LcAngularFrequencyDefinition::new(
            &prepared.fixture.graph,
            definition.law().clone(),
            definition.radian().clone(),
            definition.symbols().clone(),
            &nrz_bindings::Registry::new(nrz_bindings::ExponentPolicy::Rational),
            &mut budget
        )
        .is_err()
    );
    LcAngularFrequencyDefinition::new(
        &prepared.fixture.graph,
        definition.law().clone(),
        definition.radian().clone(),
        definition.symbols().clone(),
        &registry()?,
        &mut budget,
    )?;
    Ok(())
}

/// Construct valid angle declarations that fail this output's exact one-radian contract.
fn invalid_radians(
    definition: &LcAngularFrequencyDefinition,
    budget: &mut Budget,
) -> TestResult<[nrz_bindings::readings::EntryMeaning; 2]> {
    use nrz_bindings::{
        Descriptor, Measure, OriginDefinition, ReferenceDefinition, Role, Scale,
        readings::{
            EntryMeaning,
            algebra::{ExactConstant, Rational, syntax::Constant},
        },
    };
    let angle = definition.radian().descriptor();
    let point = Descriptor::new(Kind::Angle, Measure::coherent(), Role::Point)?;
    let point_radian = EntryMeaning::new(
        point.clone(),
        ReferenceDefinition::unit("angle reference", angle)?,
        Some(OriginDefinition::new("angle origin", &point)?),
        budget,
    )?;
    let turn = ExactConstant::named(Constant::Pi, budget)?
        .mul(&Rational::from_ratio(2, 1, budget)?.into(), budget)?;
    let turn_radian = EntryMeaning::new(
        angle.clone(),
        ReferenceDefinition::new(
            "full cycle is not one radian",
            angle,
            Scale::Expression(turn),
        )?,
        None,
        budget,
    )?;
    Ok([point_radian, turn_radian])
}
