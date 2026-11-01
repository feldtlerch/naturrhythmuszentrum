// SPDX-License-Identifier: AGPL-3.0-only
//! Quantity quotients retain both ordered coordinate partials and the divisor premise.

use super::{
    CheckedDeclarations, CheckedExpression, CheckedSensitivity, Context, Graph, JacobianEntry,
    JacobianEvidence, PairingWitness, SensitivityEntry, SensitivityError, SensitivityKernel,
    SensitivityRecipe, native, replay,
};
use crate::{BinaryOp, Budget, Declaration, Descriptor, Evidence, Registry, SensitivityObligation};
use nrz_cas::{Rational, syntax::Operator};

impl CheckedSensitivity {
    /// Prepare the exact quotient law and both partials on the original caller ledger.
    ///
    /// The symbolic nonzero premise authorizes only the identity. The numerical owner
    /// must still prove the entire divisor box excludes zero for each operation.
    /// # Errors
    /// Retains physical, original, symbolic, finite-resource and replay refusals.
    pub fn quantity_quotient(
        registry: &Registry,
        descriptors: [&Descriptor; 2],
        ledger: &mut Budget,
    ) -> Result<Self, SensitivityError> {
        let mut builder = Context::quantity_context(registry.clone(), &["x", "y"], ledger)?;
        for (key, descriptor) in ["x", "y"].into_iter().zip(descriptors) {
            builder.declare(
                key.into(),
                Declaration::Input {
                    descriptor: descriptor.clone(),
                    nonzero: key == "y",
                    evidence: Evidence::default(),
                },
            )?;
        }
        let source = builder.parse_cumulative("quantity quotient derivative", "x/y", ledger)?;
        let checks = builder.check_all_cumulative(ledger)?;
        let coordinates = vec![String::from("x"), String::from("y")];
        let inputs = native::coordinate_checks(&builder, &checks, &coordinates)?;
        let original = builder.check_cumulative(&source, ledger)?;
        let mut graph = Graph::cumulative(&builder, ledger)?;
        let result = (|| {
            let output = builder.registry.coordinate_binary(
                BinaryOp::Divide,
                descriptors[0],
                descriptors[1],
                &mut graph.budget,
            )?;
            let (jacobian, entries) = lower(&mut graph, &original, &inputs, &output)?;
            Ok::<_, SensitivityError>((output, jacobian, entries))
        })();
        graph.return_ledger(ledger);
        drop(graph);
        let (output, jacobian, entries) = result?;
        Ok(Self {
            declarations: CheckedDeclarations::new(builder, checks),
            recipe: SensitivityRecipe::QuantityQuotient,
            coordinates,
            originals: vec![original],
            auxiliary: Vec::new(),
            inputs: descriptors.into_iter().cloned().collect(),
            outputs: vec![output],
            jacobian,
            entries,
            obligations: vec![SensitivityObligation::Nonzero { coordinate: 1 }],
        })
    }
}

/// Replay each formal partial and its equality to the fixed numerical coefficient.
fn lower(
    graph: &mut Graph<'_>,
    original: &CheckedExpression,
    inputs: &[CheckedExpression],
    output: &Descriptor,
) -> Result<(JacobianEvidence, Vec<SensitivityEntry>), SensitivityError> {
    graph.extent(1, 2)?;
    graph.nonzero_context(&[(1, inputs[1].original().syntax.clone())])?;
    let mut witnesses = Vec::new();
    let mut derivatives = Vec::new();
    let mut entries = Vec::new();
    for (coordinate, key) in ["x", "y"].into_iter().enumerate() {
        let witness = graph.coordinate_partial(original, key)?;
        let expression = replay::coordinate_derivative(graph, &witness, original, key)?;
        let divisor = inputs[1].original().syntax.clone();
        let expected = if coordinate == 0 {
            let one = graph.constant(Rational::one())?;
            graph.term(Operator::Div, &[one, divisor])?
        } else {
            let square = graph.term(Operator::Mul, &[divisor.clone(), divisor])?;
            let negative = graph.term(
                Operator::Neg,
                std::slice::from_ref(&inputs[0].original().syntax),
            )?;
            graph.term(Operator::Div, &[negative, square])?
        };
        let identity = graph.equal(&expression, &expected)?;
        let pairing = PairingWitness::sensitivity(output, inputs[coordinate].descriptor())?;
        derivatives.push(JacobianEntry {
            expression,
            witness_index: witnesses.len(),
            pairing: pairing.clone(),
            identity,
        });
        witnesses.push(witness);
        entries.push(SensitivityEntry {
            pairing,
            kernel: SensitivityKernel::QuantityQuotient { coordinate },
        });
    }
    Ok((
        JacobianEvidence {
            originals: vec![original.clone()],
            coordinates: vec![String::from("x"), String::from("y")],
            witnesses,
            work: graph.work,
            entries: derivatives,
        },
        entries,
    ))
}
