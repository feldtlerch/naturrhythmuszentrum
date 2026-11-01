// SPDX-License-Identifier: AGPL-3.0-only
//! Independent expected claims for portable trigonometric syntax.

use super::common::{TestResult, default_budget};
use super::trig::Fixture;
use crate::test_api::domain::{
    AmbientDomain, AssumptionDraft, ContextDraft, HypothesisRef, Layer, RegionDraft, RegionExpr,
};
use crate::test_api::proof::{
    ApplicationDraft, Certificate, Judgment, Primitive, Statement, VerifiedWitness,
};
use crate::test_api::syntax::{Draft, Operator, PredicateKind};
use crate::test_api::{Cas, Handle, Reference};

/// Construct syntax and an expected claim without checking or publishing a proof.
pub(crate) fn prepared(operator: Operator, reordered: bool) -> TestResult<(Cas, Judgment)> {
    let mut f = Fixture::new()?;
    if reordered {
        f.term(Operator::Free("y".into()), &[])?;
        f.term(Operator::Rational(crate::test_api::Rational::zero()), &[])?;
    }
    let region = region(&mut f)?;
    let projection = f.term(Operator::RealPart, &[f.z.clone()])?;
    let inner = f.term(operator.clone(), &[projection])?;
    let node = f.term(operator, &[inner])?;
    let predicate = f
        .cas
        .admit(
            &[Draft::Condition {
                header: Reference::Existing(f.header.clone()),
                scope: Vec::new(),
                kind: PredicateKind::Named("family-property".into()),
                arguments: vec![Reference::Existing(node)],
            }],
            &mut default_budget()?,
        )?
        .pop()
        .ok_or("predicate")?;
    let context = f
        .cas
        .admit(
            &[Draft::Context(ContextDraft::Root {
                header: Reference::Existing(f.header),
                admissible: Reference::Existing(region.clone()),
                assumptions: vec![AssumptionDraft {
                    key: "supplied".into(),
                    condition: Reference::Existing(predicate.clone()),
                }],
            })],
            &mut default_budget()?,
        )?
        .pop()
        .ok_or("context")?;
    Ok((
        f.cas,
        Judgment {
            context,
            layer: Layer::Logic,
            space: None,
            statement: Statement::PredicateFact { region, predicate },
            residuals: Vec::new(),
        },
    ))
}

pub(crate) fn region(f: &mut Fixture) -> TestResult<Handle> {
    Ok(f.cas
        .admit(
            &[Draft::Region(RegionDraft {
                header: Reference::Existing(f.header.clone()),
                scope: Vec::new(),
                ambient: AmbientDomain::Real("x".into()),
                expression: RegionExpr::Ambient,
            })],
            &mut default_budget()?,
        )?
        .pop()
        .ok_or("region")?)
}

/// Only the producer checks the explicit hypothesis proof.
pub(crate) fn witness(operator: Operator) -> TestResult<(Cas, VerifiedWitness)> {
    let (mut cas, expected) = prepared(operator, false)?;
    let step = ApplicationDraft {
        rule: Primitive::ContextHypothesis,
        context: expected.context.clone(),
        conclusion: expected.statement,
        premises: Vec::new(),
        substitutions: Vec::new(),
        certificate: Certificate::Hypothesis(HypothesisRef {
            context: expected.context.clone(),
            slot: 0,
        }),
    };
    let proof = cas.check_proof(&expected.context, &[step], &mut default_budget()?)?;
    Ok((cas, proof))
}
