// SPDX-License-Identifier: AGPL-3.0-only
//! Inspect the written original target before accepting its exact closed evaluation.

use super::{Budget, Error};
use crate::AxisPhaseCase;
use nrz_cas::{
    Cas, Handle, Rational,
    source::{SyntaxNode, SyntaxRole},
    syntax::{AtomicSort, Field, Operator, Sort},
};

/// The target is exactly omega*L - 1/(omega*C), with the same original omega occurrence.
pub(super) fn check(
    case: &AxisPhaseCase,
    point: &[Rational; 2],
    frequency: &Rational,
    budget: &mut Budget,
) -> Result<(), Error> {
    let graph = case.0.graph.lock().map_err(|_| Error::GraphPoisoned)?;
    let difference = operands(
        &graph,
        &case.specification().target,
        &Operator::Sub,
        2,
        budget,
    )?;
    let inductive = operands(&graph, &difference[0], &Operator::Mul, 2, budget)?;
    let reciprocal = operands(&graph, &difference[1], &Operator::Div, 2, budget)?;
    operands(
        &graph,
        &reciprocal[0],
        &Operator::Rational(Rational::one()),
        0,
        budget,
    )?;
    let capacitive = operands(&graph, &reciprocal[1], &Operator::Mul, 2, budget)?;
    if inductive[0] != capacitive[0] {
        return Err(Error::Association);
    }
    for (node, value) in [
        (&inductive[0], frequency),
        (&inductive[1], &point[0]),
        (&capacitive[1], &point[1]),
    ] {
        operands(&graph, node, &Operator::Rational(value.clone()), 0, budget)?;
    }
    drop(graph);
    Ok(())
}

/// No hidden scopes, attachments or extra operands can disappear in a simplified value.
fn operands(
    graph: &Cas,
    node: &Handle,
    operator: &Operator,
    count: usize,
    budget: &mut Budget,
) -> Result<Vec<Handle>, Error> {
    let syntax = graph.describe_syntax(node, budget)?;
    let SyntaxNode::Expression(expression) = syntax.node else {
        return Err(Error::Association);
    };
    if &expression.operator != operator
        || expression.sort != Sort::Value(AtomicSort::Scalar(Field::Real))
        || !syntax.scope.is_empty()
        || syntax.binding.is_some()
        || syntax.dependencies.len() != count
    {
        return Err(Error::Association);
    }
    let mut result = Vec::new();
    result
        .try_reserve_exact(count)
        .map_err(|_| Error::Capacity)?;
    for (index, edge) in syntax.dependencies.into_iter().enumerate() {
        if edge.role != SyntaxRole::Operand(u32::try_from(index).map_err(|_| Error::Capacity)?) {
            return Err(Error::Association);
        }
        result.push(edge.target);
    }
    Ok(result)
}
