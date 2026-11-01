// SPDX-License-Identifier: AGPL-3.0-only
//! Original syntax and independently supplied expected values, without proof production.

use super::common::{TestResult, default_budget};
use super::half_pi::{Fixture, original, prepared, term};
use crate::test_api::Rational;
use crate::test_api::proof::{ApplicationDraft, Certificate, DomainContract, Primitive, Statement};
use crate::test_api::syntax::{Constant, Operator};

pub(crate) fn circular(
    k: &str,
    q: &str,
    cosine: bool,
    expected: i64,
    zero: bool,
    reordered: bool,
) -> TestResult<Fixture> {
    let mut f = prepared(k, q, zero, reordered)?;
    let Statement::HalfPiWitness { u, .. } = &f.expected.statement else {
        return Err("recognition".into());
    };
    let u = u.clone();
    let left = term(
        &mut f.cas,
        &f.header,
        if cosine {
            Operator::Cosine
        } else {
            Operator::Sine
        },
        &[u],
    )?;
    let right = term(
        &mut f.cas,
        &f.header,
        Operator::Rational(Rational::from_ratio(expected, 1, &mut default_budget()?)?),
        &[],
    )?;
    f.expected.statement = Statement::Equality {
        left,
        right,
        domain: f.domain.clone(),
        domain_contract: DomainContract::DefinedAndEqual,
    };
    f.steps.push(ApplicationDraft {
        rule: if cosine {
            Primitive::CosHalfPi
        } else {
            Primitive::SinHalfPi
        },
        context: f.context.clone(),
        conclusion: f.expected.statement.clone(),
        premises: vec![u32::try_from(f.steps.len() - 1)?],
        substitutions: Vec::new(),
        certificate: Certificate::None,
    });
    Ok(f)
}

pub(crate) fn atan(input: &str, one: bool, reordered: bool) -> TestResult<Fixture> {
    let mut f = prepared("2", "1", false, reordered)?;
    let u = term(
        &mut f.cas,
        &f.header,
        Operator::Rational(Rational::parse(input, &mut default_budget()?)?),
        &[],
    )?;
    original(&mut f, u.clone())?;
    f.steps.truncate(1);
    f.steps.push(ApplicationDraft {
        rule: Primitive::Reflexivity,
        context: f.context.clone(),
        conclusion: Statement::Equality {
            left: u.clone(),
            right: u.clone(),
            domain: f.domain.clone(),
            domain_contract: DomainContract::DefinedAndEqual,
        },
        premises: Vec::new(),
        substitutions: Vec::new(),
        certificate: Certificate::None,
    });
    let left = term(&mut f.cas, &f.header, Operator::Arctangent, &[u])?;
    let right = if one {
        let pi = term(&mut f.cas, &f.header, Operator::Constant(Constant::Pi), &[])?;
        let quarter = term(
            &mut f.cas,
            &f.header,
            Operator::Rational(Rational::parse("1/4", &mut default_budget()?)?),
            &[],
        )?;
        term(&mut f.cas, &f.header, Operator::Mul, &[pi, quarter])?
    } else {
        term(
            &mut f.cas,
            &f.header,
            Operator::Rational(Rational::zero()),
            &[],
        )?
    };
    f.expected.statement = Statement::Equality {
        left,
        right,
        domain: f.domain.clone(),
        domain_contract: DomainContract::DefinedAndEqual,
    };
    f.steps.push(ApplicationDraft {
        rule: if one {
            Primitive::AtanOne
        } else {
            Primitive::AtanZero
        },
        context: f.context.clone(),
        conclusion: f.expected.statement.clone(),
        premises: vec![0, 1],
        substitutions: Vec::new(),
        certificate: Certificate::None,
    });
    Ok(f)
}
