// SPDX-License-Identifier: AGPL-3.0-only
//! Public recipe fixtures independently build original syntax and the expected minus-Pi quarter.

use super::common::{TestResult, default_budget};
use super::half_pi::{Fixture, term};
use crate::test_api::domain::{
    AmbientDomain, AssumptionDraft, ContextDraft, HypothesisRef, Layer, RegionDraft, RegionExpr,
};
use crate::test_api::proof::{
    ApplicationDraft, Certificate, GuardedObligation, Primitive, Statement,
};
use crate::test_api::syntax::{Draft, Operator, PredicateKind};
use crate::test_api::{Handle, Rational, Reference};

/// Receivers prepare syntax and expectations without running the producer or proof checker.
pub(crate) fn fixture(kind: usize, reordered: bool) -> TestResult<(Fixture, Handle)> {
    let mut f = super::trig_values::atan("-1", true, reordered)?;
    f.steps.pop().ok_or("old consumer")?;
    let Statement::Equality { left: literal, .. } = &f.steps.get(1).ok_or("equality")?.conclusion
    else {
        return Err("equality")?;
    };
    let literal = literal.clone();
    let original = original(&mut f, kind, literal)?;
    let predicate = super::trig_parity::predicate(&mut f.cas, &f.header, &original)?;
    f.context = f
        .cas
        .admit(
            &[Draft::Context(ContextDraft::Root {
                header: Reference::Existing(f.header.clone()),
                admissible: Reference::Existing(f.domain.clone()),
                assumptions: vec![AssumptionDraft {
                    key: "original-defined".into(),
                    condition: Reference::Existing(predicate.clone()),
                }],
            })],
            &mut default_budget()?,
        )?
        .pop()
        .ok_or("context")?;
    let guard = f.steps.first_mut().ok_or("guard")?;
    guard.context = f.context.clone();
    guard.conclusion = Statement::PredicateFact {
        region: f.domain.clone(),
        predicate,
    };
    guard.certificate = Certificate::Hypothesis(HypothesisRef {
        context: f.context.clone(),
        slot: 0,
    });
    let equality = f.steps.get_mut(1).ok_or("equality")?;
    equality.context = f.context.clone();
    equality.rule = match kind {
        0 => Primitive::Reflexivity,
        1 | 3 | 4 => Primitive::Exact,
        _ => Primitive::RationalIdentity,
    };
    let Statement::Equality { left, .. } = &mut equality.conclusion else {
        return Err("equality".into());
    };
    *left = original.clone();
    let function = term(&mut f.cas, &f.header, Operator::Arctangent, &[original])?;
    let Statement::Equality { right, .. } = &f.expected.statement else {
        return Err("expected")?;
    };
    let right = right.clone();
    let negative = term(&mut f.cas, &f.header, Operator::Neg, &[right])?;
    f.expected.context = f.context.clone();
    let Statement::Equality { left, right, .. } = &mut f.expected.statement else {
        return Err("expected")?;
    };
    *left = function.clone();
    *right = negative;
    if kind == 2 {
        conditional_expectation(&mut f)?;
    }
    Ok((f, function))
}

/// Independently retain the two residual regions produced by local closure and the return transport.
fn conditional_expectation(f: &mut Fixture) -> TestResult {
    let x = term(&mut f.cas, &f.header, Operator::Free("x".into()), &[])?;
    let predicate = f
        .cas
        .admit(
            &[Draft::Condition {
                header: Reference::Existing(f.header.clone()),
                scope: vec![],
                kind: PredicateKind::NonZero,
                arguments: vec![Reference::Existing(x)],
            }],
            &mut default_budget()?,
        )?
        .pop()
        .ok_or("nonzero predicate")?;
    let domain = f.domain.clone();
    let local = meet(f, std::slice::from_ref(&domain))?;
    let transported = meet(f, &[f.domain.clone(), local])?;
    let returned = meet(f, &[f.domain.clone(), transported.clone()])?;
    let mut regions = [transported, returned];
    regions.sort_by_key(Handle::logical_id);
    f.expected.residuals = regions
        .into_iter()
        .map(|region| GuardedObligation {
            home: f.context.clone(),
            scope: vec![],
            layer: Layer::Pointwise,
            space: None,
            region,
            predicate: predicate.clone(),
        })
        .collect();
    Ok(())
}

/// Admit exact meet syntax without running conversion or the recipe.
fn meet(f: &mut Fixture, members: &[Handle]) -> TestResult<Handle> {
    f.cas
        .admit(
            &[Draft::Region(RegionDraft {
                header: Reference::Existing(f.header.clone()),
                scope: vec![],
                ambient: AmbientDomain::Real("x".into()),
                expression: RegionExpr::Meet(
                    members.iter().cloned().map(Reference::Existing).collect(),
                ),
            })],
            &mut default_budget()?,
        )?
        .pop()
        .ok_or_else(|| "meet region".into())
}

/// A checked subset between independent roots does not supply ancestor-chain authority.
pub(crate) fn independent_root() -> TestResult<(Fixture, Handle)> {
    let (mut f, function) = fixture(3, false)?;
    let source = f.steps[1].clone();
    let original_domain = f.domain.clone();
    let x = term(&mut f.cas, &f.header, Operator::Free("x".into()), &[])?;
    let selector = f
        .cas
        .admit(
            &[Draft::Condition {
                header: Reference::Existing(f.header.clone()),
                scope: vec![],
                kind: PredicateKind::NonZero,
                arguments: vec![Reference::Existing(x)],
            }],
            &mut default_budget()?,
        )?
        .pop()
        .ok_or("selector")?;
    f.domain = f
        .cas
        .admit(
            &[Draft::Region(RegionDraft {
                header: Reference::Existing(f.header.clone()),
                scope: vec![],
                ambient: AmbientDomain::Real("x".into()),
                expression: RegionExpr::GuardTrue {
                    base: Reference::Existing(original_domain.clone()),
                    predicate: Reference::Existing(selector),
                },
            })],
            &mut default_budget()?,
        )?
        .pop()
        .ok_or("restricted domain")?;
    let Statement::PredicateFact { predicate, .. } = &f.steps[0].conclusion else {
        return Err("original guard".into());
    };
    let predicate = predicate.clone();
    f.context = f
        .cas
        .admit(
            &[Draft::Context(ContextDraft::Root {
                header: Reference::Existing(f.header.clone()),
                admissible: Reference::Existing(f.domain.clone()),
                assumptions: vec![AssumptionDraft {
                    key: "original-defined".into(),
                    condition: Reference::Existing(predicate.clone()),
                }],
            })],
            &mut default_budget()?,
        )?
        .pop()
        .ok_or("independent context")?;
    let guard = ApplicationDraft {
        context: f.context.clone(),
        rule: Primitive::ContextHypothesis,
        conclusion: Statement::PredicateFact {
            region: f.domain.clone(),
            predicate,
        },
        premises: vec![],
        substitutions: vec![],
        certificate: Certificate::Hypothesis(HypothesisRef {
            context: f.context.clone(),
            slot: 0,
        }),
    };
    let subset = ApplicationDraft {
        context: f.context.clone(),
        rule: Primitive::RegionConstruct,
        conclusion: Statement::Subset {
            subset: f.domain.clone(),
            superset: original_domain,
        },
        premises: vec![],
        substitutions: vec![],
        certificate: Certificate::None,
    };
    let mut weakened = source.clone();
    weakened.context = f.context.clone();
    weakened.rule = Primitive::ContextWeaken;
    weakened.premises = vec![0, 1];
    let Statement::Equality { domain, .. } = &mut weakened.conclusion else {
        return Err("original equality".into());
    };
    *domain = f.domain.clone();
    f.steps = vec![guard, source, subset, weakened];
    f.expected.context = f.context.clone();
    let Statement::Equality { domain, .. } = &mut f.expected.statement else {
        return Err("expected equality".into());
    };
    *domain = f.domain.clone();
    Ok((f, function))
}

/// The ordered original premises are checked independently; neither is an unused DAG node.
pub(crate) fn premises(f: &mut Fixture) -> TestResult<[Handle; 2]> {
    let guard = f
        .cas
        .check_proof(&f.context, &f.steps[..1], &mut default_budget()?)?;
    let equality = f
        .cas
        .check_proof(&f.context, &f.steps[1..], &mut default_budget()?)?;
    Ok([guard.proof().clone(), equality.proof().clone()])
}

/// Build each original spelling without deriving any expected proof.
fn original(f: &mut Fixture, kind: usize, literal: Handle) -> TestResult<Handle> {
    let value = match kind {
        0 => literal,
        1 => {
            let one = term(
                &mut f.cas,
                &f.header,
                Operator::Rational(Rational::one()),
                &[],
            )?;
            term(&mut f.cas, &f.header, Operator::Neg, &[one])?
        }
        3 => {
            let numerator = term(
                &mut f.cas,
                &f.header,
                Operator::Rational(Rational::from_ratio(-2, 1, &mut default_budget()?)?),
                &[],
            )?;
            let denominator = term(
                &mut f.cas,
                &f.header,
                Operator::Rational(Rational::from_ratio(2, 1, &mut default_budget()?)?),
                &[],
            )?;
            term(
                &mut f.cas,
                &f.header,
                Operator::Div,
                &[numerator, denominator],
            )?
        }
        4 => {
            let negative = term(
                &mut f.cas,
                &f.header,
                Operator::Rational(Rational::from_ratio(-2, 1, &mut default_budget()?)?),
                &[],
            )?;
            let one = term(
                &mut f.cas,
                &f.header,
                Operator::Rational(Rational::one()),
                &[],
            )?;
            term(&mut f.cas, &f.header, Operator::Add, &[negative, one])?
        }
        _ => {
            let x = term(&mut f.cas, &f.header, Operator::Free("x".into()), &[])?;
            let zero = term(
                &mut f.cas,
                &f.header,
                Operator::Rational(Rational::zero()),
                &[],
            )?;
            let quotient = term(&mut f.cas, &f.header, Operator::Div, &[zero, x])?;
            term(&mut f.cas, &f.header, Operator::Add, &[literal, quotient])?
        }
    };
    Ok(value)
}
