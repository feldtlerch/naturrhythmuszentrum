// SPDX-License-Identifier: AGPL-3.0-only
//! Fresh angular-frequency symbols and exact companion targets precede every numerical fixture.

use crate::reading_testing::{TestResult, matrix, real};
use nrz_bindings::readings::algebra::{
    Cas, Handle, Reference,
    domain::{AmbientDomain, ContextDraft, HeaderDraft, RegionDraft, RegionExpr},
    readings::{AffineMap, EntryIndex, SelectionSpec},
    syntax::{Declaration, Draft, Environment, ExpressionDraft, Operator},
};
use nrz_bindings::readings::{
    CompanionParameters, CompanionReading, CompanionReadingKind, CoordinateFamily, EntryMeaning,
    LcAngularFrequencyDefinition, LcAngularFrequencySymbols, LcSpectralLaw, ParameterErrorReading,
    PhysicalReading, ReadingDefinition, Representation, RepresentationDefinition,
};
use nrz_bindings::{
    BinaryOp, Budget, CompanionKind, Descriptor, ExponentPolicy, Kind, Measure,
    ReferenceDefinition, Registry, Role,
};

/// Unsealed test input supports independent kind, symbol and companion mutations.
pub(crate) struct Proposed {
    /// Checked mathematical representations and original companion targets.
    pub(crate) symbols: LcAngularFrequencySymbols,
    /// Explicit exact source-free radian definition.
    pub(crate) radian: EntryMeaning,
    /// The sole ordered rate-times-angle physical kind relation.
    pub(crate) registry: Registry,
}

/// Build and seal a coherent angular-frequency output in the same graph as the original LC law.
pub(crate) fn prepare(
    graph: &mut Cas,
    law: LcSpectralLaw,
    budget: &mut Budget,
) -> TestResult<LcAngularFrequencyDefinition> {
    let proposed = proposed(graph, Kind::angular_frequency()?, budget)?;
    Ok(LcAngularFrequencyDefinition::new(
        graph,
        law,
        proposed.radian,
        proposed.symbols,
        &proposed.registry,
        budget,
    )?)
}

/// All value and parameter declarations are in a fresh unrestricted output context.
pub(crate) fn proposed(graph: &mut Cas, kind: Kind, budget: &mut Budget) -> TestResult<Proposed> {
    proposed_descriptor(
        graph,
        Descriptor::new(kind, Measure::coherent(), Role::Difference)?,
        budget,
    )
}

/// Valid physical representations can still carry the wrong role for an angular-frequency output.
pub(crate) fn proposed_descriptor(
    graph: &mut Cas,
    descriptor: Descriptor,
    budget: &mut Budget,
) -> TestResult<Proposed> {
    let (context, symbols) = declarations(graph, budget)?;
    let value = representation(
        graph,
        &context,
        [&symbols[0], &symbols[1]],
        descriptor,
        budget,
    )?;
    let companions = companions(graph, &context, &value, &symbols, budget)?;
    let covariance_error =
        ParameterErrorReading::define(graph, &symbols[9], &symbols[8], &companions[2], budget)?;
    let error_descriptor = covariance_error.units()[0]
        .uncertainty_descriptor()
        .ok_or("parameter error units")?
        .clone();
    let covariance_error_value = representation(
        graph,
        &context,
        [&symbols[8], &symbols[10]],
        error_descriptor,
        budget,
    )?;
    let angle = Descriptor::new(Kind::Angle, Measure::coherent(), Role::Difference)?;
    let radian = EntryMeaning::new(
        angle.clone(),
        ReferenceDefinition::unit("LC exact radian", &angle)?,
        None,
        budget,
    )?;
    let mut registry = Registry::new(ExponentPolicy::Rational);
    registry.register_binary(
        BinaryOp::Multiply,
        Kind::rate()?,
        Kind::Angle,
        Kind::angular_frequency()?,
    )?;
    Ok(Proposed {
        symbols: LcAngularFrequencySymbols {
            value,
            companions,
            covariance_error,
            covariance_error_value,
        },
        radian,
        registry,
    })
}

/// Declare the complete literal symbol inventory before creating any derived expression.
fn declarations(graph: &mut Cas, budget: &mut Budget) -> TestResult<(Handle, Vec<Handle>)> {
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
    ];
    let declarations = keys
        .iter()
        .enumerate()
        .map(|(index, key)| Declaration {
            key: (*key).into(),
            display_name: (*key).into(),
            sort: if index == 1 || index == 10 {
                real()
            } else {
                matrix(1, 1)
            },
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
        let draft = Draft::Expression(ExpressionDraft {
            header: Reference::Existing(nodes[0].clone()),
            scope: Vec::new(),
            operator: Operator::Free(key.into()),
            sort: if index == 1 || index == 10 {
                real()
            } else {
                matrix(1, 1)
            },
            operands: Vec::new(),
            space: None,
            binding: None,
        });
        symbols.push(
            graph
                .admit(&[draft], budget)?
                .pop()
                .ok_or("output declaration")?,
        );
    }
    Ok((nodes[2].clone(), symbols))
}

/// The declared scalar reads and reconstructs exactly one original coordinate with unit scale.
fn representation(
    graph: &mut Cas,
    context: &Handle,
    symbols: [&Handle; 2],
    descriptor: Descriptor,
    budget: &mut Budget,
) -> TestResult<Representation> {
    let origin = if descriptor.role() == Role::Point {
        Some(nrz_bindings::OriginDefinition::new(
            "LC output origin",
            &descriptor,
        )?)
    } else {
        None
    };
    let reference = ReferenceDefinition::unit("LC output coordinate", &descriptor.difference())?;
    let meaning = EntryMeaning::new(descriptor, reference, origin, budget)?;
    let family = CoordinateFamily::new(graph, context, symbols[0], vec![meaning.clone()], budget)?;
    let reading = PhysicalReading::define(
        graph,
        &family,
        ReadingDefinition {
            symbol: symbols[1].clone(),
            selection: SelectionSpec::Entry(EntryIndex { row: 0, column: 0 }),
            map: AffineMap::identity(1, budget)?,
            entries: vec![meaning],
        },
        budget,
    )?;
    Ok(Representation::prove(
        graph,
        &reading,
        RepresentationDefinition::Defined {
            symbol: symbols[1].clone(),
            expansion: AffineMap::identity(1, budget)?,
        },
        budget,
    )?)
}

/// Each parameter has its own distinct original symbol and the exact ordered output targets.
fn companions(
    graph: &mut Cas,
    context: &Handle,
    value: &Representation,
    symbols: &[Handle],
    budget: &mut Budget,
) -> TestResult<[CompanionReading; 3]> {
    let kinds = [
        CompanionReadingKind::Radius(CompanionKind::DeterministicBound),
        CompanionReadingKind::Radius(CompanionKind::NumericalError),
        CompanionReadingKind::Covariance,
    ];
    let mut result = Vec::new();
    for (index, kind) in kinds.into_iter().enumerate() {
        let targets = vec![value.reading().clone(); if index == 2 { 2 } else { 1 }];
        let parameters = CompanionParameters::new(
            graph,
            context,
            &symbols[index * 2 + 2],
            kind,
            &targets,
            budget,
        )?;
        result.push(CompanionReading::define(
            graph,
            &symbols[index * 2 + 3],
            &parameters,
            &targets,
            budget,
        )?);
    }
    result
        .try_into()
        .map_err(|_| "three output companions".into())
}
