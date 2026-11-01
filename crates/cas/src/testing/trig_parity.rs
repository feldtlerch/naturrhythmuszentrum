// SPDX-License-Identifier: AGPL-3.0-only
//! Independent parity syntax keeps two equal-spelled declarations and original guard provenance.

use super::common::{TestResult, default_budget, real};
use super::half_pi::{Fixture, term};
use crate::test_api::domain::{
    AmbientDomain, AssumptionDraft, ContextDraft, HeaderDraft, HypothesisRef, Layer, RegionDraft,
    RegionExpr,
};
use crate::test_api::proof::{
    ApplicationDraft, Certificate, DomainContract, Judgment, Primitive, Statement,
};
use crate::test_api::syntax::{Constant, Declaration, Draft, Environment, Operator, PredicateKind};
use crate::test_api::{Cas, Handle, Reference};

pub(crate) const fn operator(kind: usize) -> Operator {
    match kind {
        0 => Operator::Sine,
        1 => Operator::Cosine,
        _ => Operator::Arctangent,
    }
}

pub(crate) const fn rule(kind: usize) -> Primitive {
    match kind {
        0 => Primitive::SinOdd,
        1 => Primitive::CosEven,
        _ => Primitive::AtanOdd,
    }
}

pub(crate) fn predicate(cas: &mut Cas, header: &Handle, value: &Handle) -> TestResult<Handle> {
    Ok(cas
        .admit(
            &[Draft::Condition {
                header: Reference::Existing(header.clone()),
                scope: Vec::new(),
                kind: PredicateKind::Defined,
                arguments: vec![Reference::Existing(value.clone())],
            }],
            &mut default_budget()?,
        )?
        .pop()
        .ok_or("predicate")?)
}

/// Expected claims are built from syntax, never by running a checker in the receiver.
pub(crate) fn fixture(
    kind: usize,
    composite: bool,
    nested: bool,
    reordered: bool,
) -> TestResult<(Fixture, Handle, Handle)> {
    prepared(kind, composite, nested, reordered, OutputSyntax::Present)
}

/// Retain the original function and guard without admitting any parity output wrapper.
pub(crate) fn source(kind: usize) -> TestResult<(Fixture, Handle, Handle)> {
    prepared(kind, true, false, false, OutputSyntax::Absent)
}

#[derive(Clone, Copy)]
enum OutputSyntax {
    Absent,
    Present,
}

fn prepared(
    kind: usize,
    composite: bool,
    nested: bool,
    reordered: bool,
    outputs: OutputSyntax,
) -> TestResult<(Fixture, Handle, Handle)> {
    let (mut cas, header) = declarations(reordered)?;
    let x = term(&mut cas, &header, Operator::Free("x".into()), &[])?;
    let mut u = if composite {
        term(&mut cas, &header, Operator::Mul, &[x.clone(), x])?
    } else {
        x
    };
    let domain = ambient(&mut cas, &header)?;
    let guard = predicate(&mut cas, &header, &u)?;
    let context = cas
        .admit(
            &[Draft::Context(ContextDraft::Root {
                header: Reference::Existing(header.clone()),
                admissible: Reference::Existing(domain.clone()),
                assumptions: vec![AssumptionDraft {
                    key: "original-defined".into(),
                    condition: Reference::Existing(guard.clone()),
                }],
            })],
            &mut default_budget()?,
        )?
        .pop()
        .ok_or("context")?;
    let mut steps = vec![ApplicationDraft {
        rule: Primitive::ContextHypothesis,
        context: context.clone(),
        conclusion: Statement::PredicateFact {
            region: domain.clone(),
            predicate: guard,
        },
        certificate: Certificate::Hypothesis(HypothesisRef {
            context: context.clone(),
            slot: 0,
        }),
        premises: Vec::new(),
        substitutions: Vec::new(),
    }];
    if nested {
        u = term(&mut cas, &header, Operator::Sine, &[u])?;
        let guard = predicate(&mut cas, &header, &u)?;
        steps.push(ApplicationDraft {
            rule: Primitive::TrigDefined,
            context: context.clone(),
            conclusion: Statement::PredicateFact {
                region: domain.clone(),
                predicate: guard,
            },
            certificate: Certificate::None,
            premises: vec![0],
            substitutions: Vec::new(),
        });
    }
    let function = term(&mut cas, &header, operator(kind), std::slice::from_ref(&u))?;
    let (left, right) = if matches!(outputs, OutputSyntax::Present) {
        endpoints(&mut cas, &header, kind, &function, &u)?
    } else {
        (function.clone(), function.clone())
    };
    let statement = Statement::Equality {
        left,
        right,
        domain: domain.clone(),
        domain_contract: DomainContract::DefinedAndEqual,
    };
    if matches!(outputs, OutputSyntax::Present) {
        steps.push(ApplicationDraft {
            rule: rule(kind),
            context: context.clone(),
            conclusion: statement.clone(),
            certificate: Certificate::None,
            premises: vec![u32::from(nested)],
            substitutions: Vec::new(),
        });
    }
    let expected = Judgment {
        context: context.clone(),
        layer: Layer::Pointwise,
        space: None,
        residuals: Vec::new(),
        statement,
    };
    Ok((
        Fixture {
            cas,
            header,
            context,
            domain,
            steps,
            expected,
        },
        function,
        u,
    ))
}

/// Construct the expected endpoints independently after measured output-absent production.
pub(crate) fn endpoints(
    cas: &mut Cas,
    header: &Handle,
    kind: usize,
    function: &Handle,
    original: &Handle,
) -> TestResult<(Handle, Handle)> {
    let negative = term(cas, header, Operator::Neg, std::slice::from_ref(original))?;
    let left = term(cas, header, operator(kind), &[negative])?;
    let right = if kind == 1 {
        function.clone()
    } else {
        term(cas, header, Operator::Neg, std::slice::from_ref(function))?
    };
    Ok((left, right))
}

/// Keep the exact original Real region independent of the parity endpoints.
fn ambient(cas: &mut Cas, header: &Handle) -> TestResult<Handle> {
    Ok(cas
        .admit(
            &[Draft::Region(RegionDraft {
                header: Reference::Existing(header.clone()),
                scope: Vec::new(),
                ambient: AmbientDomain::Real("x".into()),
                expression: RegionExpr::Ambient,
            })],
            &mut default_budget()?,
        )?
        .pop()
        .ok_or("domain")?)
}

/// Prepare equal-spelled declarations without deriving any expected proof.
fn declarations(reordered: bool) -> TestResult<(Cas, Handle)> {
    let mut cas = Cas::new(default_budget()?.limits().clone())?;
    let header = cas
        .admit(
            &[Draft::Header(HeaderDraft::Core(Environment {
                declarations: ["x", "y"]
                    .into_iter()
                    .map(|key| Declaration {
                        key: key.into(),
                        display_name: "u".into(),
                        sort: real(),
                    })
                    .collect(),
                ..Environment::default()
            }))],
            &mut default_budget()?,
        )?
        .pop()
        .ok_or("header")?;
    if reordered {
        term(&mut cas, &header, Operator::Constant(Constant::E), &[])?;
        term(&mut cas, &header, Operator::Free("y".into()), &[])?;
    }
    Ok((cas, header))
}
