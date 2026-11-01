// SPDX-License-Identifier: AGPL-3.0-only
//! Exact quantity powers retain the original law and replay its raw-coordinate derivative.

use super::{
    CheckedDeclarations, CheckedExpression, CheckedSensitivity, Context, Graph, JacobianEntry,
    JacobianEvidence, PairingWitness, SensitivityEntry, SensitivityError, SensitivityKernel,
    SensitivityRecipe, native, replay,
};
use crate::{Budget, Declaration, Descriptor, Evidence, Registry};
use nrz_cas::Rational;
use nrz_cas::syntax::Operator;

impl CheckedSensitivity {
    /// Prepare `n*x^(n-1)` under the original quantity measure and caller ledger.
    ///
    /// Zero powers retain the original nonzero premise; their derivative is exactly zero.
    /// Unit powers retain a formal one without adding an artificial divisor obligation.
    /// # Errors
    /// Refuses physical, exact-exponent, original, symbolic and finite-resource failures.
    pub fn quantity_integer_power(
        registry: &Registry,
        input: &Descriptor,
        exponent: i64,
        ledger: &mut Budget,
    ) -> Result<Self, SensitivityError> {
        let output = registry.coordinate_integer_power(input, exponent, ledger)?;
        let mut builder = Context::quantity_context(registry.clone(), &["x"], ledger)?;
        builder.declare(
            "x".into(),
            Declaration::Input {
                descriptor: input.clone(),
                nonzero: exponent <= 0,
                evidence: Evidence::default(),
            },
        )?;
        let source = builder.parse_cumulative(
            "quantity integer-power derivative",
            &format!("x^({exponent})"),
            ledger,
        )?;
        let checks = builder.check_all_cumulative(ledger)?;
        let coordinates = vec![String::from("x")];
        let inputs = native::coordinate_checks(&builder, &checks, &coordinates)?;
        let original = builder.check_cumulative(&source, ledger)?;
        let mut graph = Graph::cumulative(&builder, ledger)?;
        let result = lower(&mut graph, &original, &inputs[0], &output, exponent);
        graph.return_ledger(ledger);
        drop(graph);
        let (jacobian, entry) = result?;
        Ok(Self {
            declarations: CheckedDeclarations::new(builder, checks),
            recipe: SensitivityRecipe::QuantityIntegerPower { exponent },
            coordinates,
            originals: vec![original],
            auxiliary: Vec::new(),
            inputs: vec![input.clone()],
            outputs: vec![output],
            jacobian,
            entries: vec![entry],
            obligations: if exponent <= 0 {
                vec![super::super::SensitivityObligation::Nonzero { coordinate: 0 }]
            } else {
                Vec::new()
            },
        })
    }
}

/// Prove the exact derivative against the closed instruction, retaining every receipt.
fn lower(
    graph: &mut Graph<'_>,
    original: &CheckedExpression,
    input: &CheckedExpression,
    output: &Descriptor,
    exponent: i64,
) -> Result<(JacobianEvidence, SensitivityEntry), SensitivityError> {
    graph.extent(1, 1)?;
    if exponent <= 0 {
        graph.nonzero_context(&[(0, input.original().syntax.clone())])?;
    }
    let witness = graph.coordinate_partial(original, "x")?;
    let expression = replay::coordinate_derivative(graph, &witness, original, "x")?;
    let expected = match exponent {
        0 => graph.constant(Rational::zero())?,
        1 => graph.constant(Rational::one())?,
        _ => {
            let power = exponent
                .checked_sub(1)
                .ok_or(SensitivityError::UnsupportedRecipe)?;
            let value = Rational::from_ratio(exponent, 1, &mut graph.budget)?;
            let factor = graph.constant(value)?;
            let power = graph.term(
                Operator::IntegerPower(power),
                std::slice::from_ref(&input.original().syntax),
            )?;
            graph.term(Operator::Mul, &[factor, power])?
        }
    };
    let identity = graph.equal(&expression, &expected)?;
    let pairing = PairingWitness::sensitivity(output, input.descriptor())?;
    let entry = SensitivityEntry {
        pairing: pairing.clone(),
        kernel: SensitivityKernel::QuantityIntegerPower { exponent },
    };
    Ok((
        JacobianEvidence {
            originals: vec![original.clone()],
            coordinates: vec![String::from("x")],
            witnesses: vec![witness],
            work: graph.work,
            entries: vec![JacobianEntry {
                expression,
                witness_index: 0,
                pairing,
                identity,
            }],
        },
        entry,
    ))
}
