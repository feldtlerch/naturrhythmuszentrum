// SPDX-License-Identifier: AGPL-3.0-only
//! Independent hand fixtures for order-changing rational object calculus.

use super::cases::Fixture as Base;
use super::common::{TestResult, default_budget};
use super::series_proof::{Fixture, Production, defined, object, rational, step};
use crate::test_api::Handle;
use crate::test_api::domain::Layer;
use crate::test_api::proof::Judgment;
use crate::test_api::series::{
    SeriesCertificate as C, SeriesDerivativeRow, SeriesIntegralRow, SeriesOperation as O,
    SeriesStatement as S,
};
use crate::test_api::solving::{CoefficientDomainData, SolveDraft};
use crate::test_api::syntax::Draft;

/// Explicit input vectors include internal and trailing zeros and negative fractions.
fn inputs(kind: usize) -> TestResult<&'static [&'static str]> {
    Ok(match kind {
        7 | 12 => &["1", "2", "3"],
        8 => &["7", "0"],
        9 => &["2", "6"],
        10 => &["1", "-2", "3"],
        11 => &["0", "0"],
        _ => return Err("calculus fixture kind".into()),
    })
}

/// Original syntax is independent of all proposed outputs and row arithmetic.
fn basis(kind: usize, reordered: bool) -> TestResult<(Base, Handle, Handle)> {
    let mut base = Base::new()?;
    if reordered {
        for text in ["6", "-1", "-1/2", "3", "-2", "2", "7"] {
            base.number(text)?;
        }
    }
    let field = base
        .cas
        .admit(
            &[Draft::Solve(SolveDraft::CoefficientDomain(
                CoefficientDomainData::Rational,
            ))],
            &mut default_budget()?,
        )?
        .pop()
        .ok_or("Q")?;
    let mut input = object(&mut base, &field, inputs(kind)?)?;
    if kind == 12 {
        let restricted = base.cas.compute_series(
            &O::Truncate { input, order: 2 },
            &[],
            &mut default_budget()?,
        )?;
        let crate::test_api::proof::Statement::Series(statement) = &restricted.judgment().statement
        else {
            return Err("restricted series judgment".into());
        };
        let S::Truncate { output, .. } = statement.as_ref() else {
            return Err("explicit restriction".into());
        };
        input = output.clone();
    }
    Ok((base, field, input))
}

/// Literal constants are not reconstructed from derivative or integral output.
const fn constant(kind: usize) -> &'static str {
    match kind {
        10 => "-1/2",
        11 => "0",
        _ => "1",
    }
}

/// Production starts without output-only syntax and with one independently checked ordinary fact.
pub(crate) fn production(kind: usize) -> TestResult<Production> {
    let (mut base, _, input) = basis(kind, false)?;
    let mut proofs = Vec::new();
    let operation = if kind < 9 || kind == 12 {
        O::Derive { input }
    } else {
        let constant = base.number(constant(kind))?;
        let fact = defined(&mut base, &constant)?;
        proofs.push(
            base.cas
                .check_proof(&base.context, &[fact], &mut default_budget()?)?
                .proof()
                .clone(),
        );
        O::Integrate { input, constant }
    };
    Ok(Production {
        base,
        operation,
        proofs,
    })
}

/// Hand literal integer, inverse, input and output tuple.
type IntegralHand<'a> = (&'a [&'a str], &'a [(&'a str, &'a str, &'a str, &'a str)]);

/// Each input, integer, inverse and result is a hand literal, not a producer-generated oracle.
pub(crate) fn fixture(kind: usize, reordered: bool) -> TestResult<Fixture> {
    let (mut base, field, input) = basis(kind, reordered)?;
    let mut steps = Vec::new();
    let root = if kind < 9 || kind == 12 {
        let (output, hand): (&[&str], &[(&str, &str, &str)]) = if kind == 7 {
            (&["2", "6"], &[("2", "1", "2"), ("3", "2", "6")])
        } else if kind == 8 {
            (&["0"], &[("0", "1", "0")])
        } else {
            (&["2"], &[("2", "1", "2")])
        };
        let output = object(&mut base, &field, output)?;
        let rows = hand
            .iter()
            .enumerate()
            .map(|(index, (input, multiplier, result))| {
                Ok(SeriesDerivativeRow {
                    index: u32::try_from(index)?,
                    input: rational(input)?,
                    multiplier: rational(multiplier)?,
                    result: rational(result)?,
                })
            })
            .collect::<TestResult<Vec<_>>>()?;
        step(
            &base,
            S::Derive { input, output },
            C::Derive { form: 0, rows },
            Vec::new(),
        )
    } else {
        let constant = base.number(constant(kind))?;
        steps.push(defined(&mut base, &constant)?);
        let (output, hand): IntegralHand<'_> = match kind {
            9 => (
                &["1", "2", "3"],
                &[("1", "1", "2", "2"), ("2", "1/2", "6", "3")],
            ),
            10 => (
                &["-1/2", "1", "-1", "1"],
                &[
                    ("1", "1", "1", "1"),
                    ("2", "1/2", "-2", "-1"),
                    ("3", "1/3", "3", "1"),
                ],
            ),
            11 => (
                &["0", "0", "0"],
                &[("1", "1", "0", "0"), ("2", "1/2", "0", "0")],
            ),
            _ => return Err("integral fixture kind".into()),
        };
        let output = object(&mut base, &field, output)?;
        let rows = hand
            .iter()
            .enumerate()
            .map(|(index, (integer, inverse, input, result))| {
                Ok(SeriesIntegralRow {
                    index: u32::try_from(index)?,
                    integer: rational(integer)?,
                    inverse: rational(inverse)?,
                    input: rational(input)?,
                    result: rational(result)?,
                })
            })
            .collect::<TestResult<Vec<_>>>()?;
        step(
            &base,
            S::Integrate {
                input,
                constant,
                output,
            },
            C::Integrate { form: 0, rows },
            vec![0],
        )
    };
    let expected = Judgment {
        context: base.context.clone(),
        layer: Layer::Logic,
        space: None,
        statement: root.conclusion.clone(),
        residuals: Vec::new(),
    };
    steps.push(root);
    Ok(Fixture {
        base,
        field,
        steps,
        expected,
    })
}
