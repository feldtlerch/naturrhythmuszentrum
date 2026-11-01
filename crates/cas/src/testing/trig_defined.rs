// SPDX-License-Identifier: AGPL-3.0-only
//! Independently constructed totality claims for public checking and portable replay.

use super::common::{TestResult, default_budget};
use super::half_pi::{Fixture, original, prepared, term};
use crate::test_api::domain::Layer;
use crate::test_api::proof::{ApplicationDraft, Certificate, Primitive, Statement};
use crate::test_api::syntax::{Draft, Operator, PredicateKind};
use crate::test_api::{Handle, Reference};

pub(crate) fn fixture(
    operator: Operator,
    composite: bool,
    reordered: bool,
) -> TestResult<(Fixture, Handle)> {
    let (mut f, function) = source(operator, composite, reordered)?;
    let predicate = expected_predicate(&mut f, &function)?;
    f.expected.statement = Statement::PredicateFact {
        region: f.domain.clone(),
        predicate,
    };
    f.expected.layer = Layer::Logic;
    f.steps.push(ApplicationDraft {
        rule: Primitive::TrigDefined,
        context: f.context.clone(),
        conclusion: f.expected.statement.clone(),
        premises: vec![0],
        substitutions: Vec::new(),
        certificate: Certificate::None,
    });
    Ok((f, function))
}

/// Admit only the original function and guard, leaving the output predicate absent.
pub(crate) fn source(
    operator: Operator,
    composite: bool,
    reordered: bool,
) -> TestResult<(Fixture, Handle)> {
    let mut f = prepared("0", "0", true, reordered)?;
    let x = term(&mut f.cas, &f.header, Operator::Free("x".into()), &[])?;
    let u = if composite {
        term(&mut f.cas, &f.header, Operator::Mul, &[x.clone(), x])?
    } else {
        x
    };
    original(&mut f, u.clone())?;
    f.steps.truncate(1);
    let function = term(&mut f.cas, &f.header, operator, &[u])?;
    Ok((f, function))
}

/// Construct the expected syntax independently, after measured production has finished.
pub(crate) fn expected_predicate(f: &mut Fixture, function: &Handle) -> TestResult<Handle> {
    Ok(f.cas
        .admit(
            &[Draft::Condition {
                header: Reference::Existing(f.header.clone()),
                scope: Vec::new(),
                kind: PredicateKind::Defined,
                arguments: vec![Reference::Existing(function.clone())],
            }],
            &mut default_budget()?,
        )?
        .pop()
        .ok_or("predicate")?)
}
