// SPDX-License-Identifier: AGPL-3.0-only
//! The exact positive coordinate root follows checked polynomial partials of `t^2-x`.

use super::{
    CheckedDeclarations, CheckedExpression, CheckedSensitivity, Context, Graph, JacobianEntry,
    JacobianEvidence, PairingWitness, SensitivityEntry, SensitivityError, SensitivityKernel,
    SensitivityRecipe, native, replay,
};
use crate::{Budget, Declaration, Descriptor, Evidence, Registry, SensitivityObligation};
use nrz_cas::{Rational, syntax::Operator};

impl CheckedSensitivity {
    /// Prepare the fixed positive-root implication after replaying both polynomial partials.
    ///
    /// The original root and reverse square kind rule must both be admitted. The auxiliary
    /// root placeholder is not a source; strict positivity remains a numerical obligation.
    /// # Errors
    /// Preserves physical, exact-scale, original, symbolic and finite-resource refusals.
    pub fn quantity_positive_root(
        registry: &Registry,
        input: &Descriptor,
        ledger: &mut Budget,
    ) -> Result<Self, SensitivityError> {
        let registry = registry.clone();
        let output = registry.coordinate_square_root(input, ledger)?;
        let mut builder = Context::quantity_root_context(registry, ledger)?;
        for (key, descriptor) in [("x", input), ("t", &output)] {
            builder.declare(
                key.into(),
                Declaration::Input {
                    descriptor: descriptor.clone(),
                    nonzero: false,
                    evidence: Evidence::default(),
                },
            )?;
        }
        let source =
            builder.parse_cumulative("quantity positive-root derivative", "sqrt(x)", ledger)?;
        let relation =
            builder.parse_cumulative("quantity positive-root relation", "t^2-x", ledger)?;
        let checks = builder.check_all_cumulative(ledger)?;
        let inputs =
            native::coordinate_checks(&builder, &checks, &[String::from("x"), String::from("t")])?;
        let original = builder.check_cumulative(&source, ledger)?;
        let relation = builder.check_cumulative(&relation, ledger)?;
        let mut graph = Graph::cumulative(&builder, ledger)?;
        let result = lower(&mut graph, &relation, &inputs);
        graph.return_ledger(ledger);
        drop(graph);
        let jacobian = result?;
        let pairing = PairingWitness::sensitivity(&output, input)?;
        Ok(Self {
            declarations: CheckedDeclarations::new(builder, checks),
            recipe: SensitivityRecipe::QuantityPositiveRoot,
            coordinates: vec![String::from("x")],
            originals: vec![original],
            auxiliary: vec![String::from("t")],
            inputs: vec![input.clone()],
            outputs: vec![output],
            jacobian,
            entries: vec![SensitivityEntry {
                pairing,
                kernel: SensitivityKernel::QuantityPositiveRoot,
            }],
            obligations: vec![SensitivityObligation::PositiveRoot {
                coordinate: 0,
                output: 0,
            }],
        })
    }
}

/// The partials `2*t` and `-1` authorize only the fixed implication `2*t*dt=dx`.
fn lower(
    graph: &mut Graph<'_>,
    relation: &CheckedExpression,
    inputs: &[CheckedExpression],
) -> Result<JacobianEvidence, SensitivityError> {
    graph.extent(1, 2)?;
    let mut witnesses = Vec::new();
    let mut entries = Vec::new();
    for (index, key) in [(1, "t"), (0, "x")] {
        let witness = graph.coordinate_partial(relation, key)?;
        let expression = replay::coordinate_derivative(graph, &witness, relation, key)?;
        let numerator = if index == 1 { 2 } else { -1 };
        let value = Rational::from_ratio(numerator, 1, &mut graph.budget)?;
        let factor = graph.constant(value)?;
        let expected = if index == 1 {
            graph.term(
                Operator::Mul,
                &[factor, inputs[1].original().syntax.clone()],
            )?
        } else {
            factor
        };
        let identity = graph.equal(&expression, &expected)?;
        let pairing =
            PairingWitness::sensitivity(inputs[0].descriptor(), inputs[index].descriptor())?;
        entries.push(JacobianEntry {
            expression,
            witness_index: witnesses.len(),
            pairing,
            identity,
        });
        witnesses.push(witness);
    }
    Ok(JacobianEvidence {
        originals: vec![relation.clone()],
        coordinates: vec![String::from("t"), String::from("x")],
        witnesses,
        work: graph.work,
        entries,
    })
}
