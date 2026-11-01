// SPDX-License-Identifier: AGPL-3.0-only
//! Both fresh frequency channels and their cross block are declared in one unrestricted context.

use super::TestResult;
use crate::reading_testing::{matrix, real};
use nrz_bindings::readings::algebra::{
    Cas, Handle, Reference,
    domain::{AmbientDomain, ContextDraft, HeaderDraft, RegionDraft, RegionExpr},
    syntax::{Declaration, Draft, Environment, ExpressionDraft, Operator},
};
use nrz_bindings::readings::{
    CompanionParameters, CompanionReading, CompanionReadingKind, EntryMeaning,
    LcAngularFrequencyDefinition, LcFrequencyDefinition, LcFrequencySymbols, LcSpectralLaw,
    ParameterErrorReading,
};
use nrz_bindings::{
    Budget, CheckedDeclarations, Descriptor, Kind, Measure, ReferenceDefinition, Role, Scale,
};

/// Exact identity channel and cycle proposal remain accessible to mutation tests before sealing.
pub(super) struct Proposed {
    pub(super) angular: LcAngularFrequencyDefinition,
    pub(super) cycle: EntryMeaning,
    pub(super) symbols: LcFrequencySymbols,
}

pub(super) fn prepare(
    graph: &mut Cas,
    law: LcSpectralLaw,
    declarations: &CheckedDeclarations,
    budget: &mut Budget,
) -> TestResult<LcFrequencyDefinition> {
    let proposed = proposed(graph, law, declarations, false, budget)?;
    Ok(LcFrequencyDefinition::new(
        graph,
        proposed.angular,
        proposed.cycle,
        proposed.symbols,
        declarations,
        budget,
    )?)
}

/// Reversed cross targets are mathematically valid declarations that the sealed bundle must reject.
pub(super) fn proposed(
    graph: &mut Cas,
    law: LcSpectralLaw,
    declarations: &CheckedDeclarations,
    reverse: bool,
    budget: &mut Budget,
) -> TestResult<Proposed> {
    let (context, symbols) = declarations_for(graph, budget)?;
    let angular = crate::angular_testing::channel(
        graph,
        &context,
        &symbols[..11],
        descriptor(true)?,
        budget,
    )?;
    let cyclic = crate::angular_testing::channel(
        graph,
        &context,
        &symbols[11..22],
        descriptor(false)?,
        budget,
    )?;
    let mut targets = [
        angular.value.reading().clone(),
        cyclic.value.reading().clone(),
    ];
    if reverse {
        targets.reverse();
    }
    let parameters = CompanionParameters::new(
        graph,
        &context,
        &symbols[22],
        CompanionReadingKind::Covariance,
        &targets,
        budget,
    )?;
    let cross_covariance =
        CompanionReading::define(graph, &symbols[23], &parameters, &targets, budget)?;
    let cross_error = ParameterErrorReading::define(
        graph,
        &symbols[25],
        &symbols[24],
        &cross_covariance,
        budget,
    )?;
    let error_descriptor = cross_error.units()[0]
        .uncertainty_descriptor()
        .ok_or("cross error units")?
        .clone();
    let cross_error_value = crate::angular_testing::representation(
        graph,
        &context,
        [&symbols[24], &symbols[26]],
        error_descriptor,
        budget,
    )?;
    let angle = Descriptor::new(Kind::Angle, Measure::coherent(), Role::Difference)?;
    let radian = EntryMeaning::new(
        angle.clone(),
        ReferenceDefinition::unit("native frequency radian", &angle)?,
        None,
        budget,
    )?;
    let angular = LcAngularFrequencyDefinition::new(
        graph,
        law,
        radian,
        angular,
        declarations.registry(),
        budget,
    )?;
    let turn = declarations
        .registry()
        .measure("turn")
        .ok_or("registered turn")?;
    let cycle = EntryMeaning::new(
        angle.clone(),
        ReferenceDefinition::new(
            "native full cycle",
            &angle,
            Scale::Expression(turn.measure().scale().clone()),
        )?,
        None,
        budget,
    )?;
    Ok(Proposed {
        angular,
        cycle,
        symbols: LcFrequencySymbols {
            cyclic: cyclic.value,
            companions: cyclic.companions,
            covariance_error: cyclic.covariance_error,
            covariance_error_value: cyclic.covariance_error_value,
            cross_covariance,
            cross_error,
            cross_error_value,
        },
    })
}

pub(super) fn descriptor(angular: bool) -> TestResult<Descriptor> {
    Ok(Descriptor::new(
        if angular {
            Kind::angular_frequency()?
        } else {
            Kind::frequency()?
        },
        Measure::coherent(),
        Role::Difference,
    )?)
}

/// One output context declares all twenty-seven distinct base, view and companion symbols.
fn declarations_for(graph: &mut Cas, budget: &mut Budget) -> TestResult<(Handle, Vec<Handle>)> {
    let keys = [
        "omega_base",
        "omega",
        "omega_total_source",
        "omega_total_output",
        "omega_numeric_source",
        "omega_numeric_output",
        "omega_covariance_source",
        "omega_covariance_output",
        "omega_covariance_error_source",
        "omega_covariance_error_output",
        "omega_covariance_error_view",
        "frequency_base",
        "frequency",
        "frequency_total_source",
        "frequency_total_output",
        "frequency_numeric_source",
        "frequency_numeric_output",
        "frequency_covariance_source",
        "frequency_covariance_output",
        "frequency_covariance_error_source",
        "frequency_covariance_error_output",
        "frequency_covariance_error_view",
        "cross_source",
        "cross_output",
        "cross_error_source",
        "cross_error_output",
        "cross_error_view",
    ];
    let sort = |index| {
        if [1, 10, 12, 21, 26].contains(&index) {
            real()
        } else {
            matrix(1, 1)
        }
    };
    let declarations = keys
        .iter()
        .enumerate()
        .map(|(index, key)| Declaration {
            key: (*key).into(),
            display_name: (*key).into(),
            sort: sort(index),
        })
        .collect();
    let nodes = graph.admit(
        &[
            Draft::Header(HeaderDraft::Core(Environment {
                declarations,
                ..Environment::default()
            })),
            Draft::Region(RegionDraft {
                header: Reference::Local(0),
                scope: Vec::new(),
                ambient: AmbientDomain::Assignments,
                expression: RegionExpr::Ambient,
            }),
            Draft::Context(ContextDraft::Root {
                header: Reference::Local(0),
                admissible: Reference::Local(1),
                assumptions: Vec::new(),
            }),
        ],
        budget,
    )?;
    let mut symbols = Vec::new();
    for (index, key) in keys.into_iter().enumerate() {
        symbols.push(
            graph
                .admit(
                    &[Draft::Expression(ExpressionDraft {
                        header: Reference::Existing(nodes[0].clone()),
                        scope: Vec::new(),
                        operator: Operator::Free(key.into()),
                        sort: sort(index),
                        operands: Vec::new(),
                        space: None,
                        binding: None,
                    })],
                    budget,
                )?
                .pop()
                .ok_or("frequency symbol")?,
        );
    }
    Ok((nodes[2].clone(), symbols))
}
