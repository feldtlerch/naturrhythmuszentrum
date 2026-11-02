// SPDX-License-Identifier: AGPL-3.0-only
//! Public recipe fixtures independently build original syntax and the expected minus-Pi quarter.

use super::common::{TestResult, default_budget};
use super::half_pi::{Fixture, term};
use crate::test_api::domain::{AssumptionDraft, ContextDraft, HypothesisRef};
use crate::test_api::proof::{Certificate, Primitive, Statement};
use crate::test_api::syntax::{Draft, Operator};
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
