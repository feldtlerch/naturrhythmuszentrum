// SPDX-License-Identifier: AGPL-3.0-only
//! Independent ceilings and exact consumed-prefix assertions for half-Pi evidence.

use super::common::TestResult;
use crate::test_api::limits::{BudgetKind, Limits};

pub(crate) const KINDS: [BudgetKind; 18] = [
    BudgetKind::Work,
    BudgetKind::ArithmeticSteps,
    BudgetKind::ConditionSteps,
    BudgetKind::Depth,
    BudgetKind::StagedPayloadBytes,
    BudgetKind::PayloadBytes,
    BudgetKind::WitnessBytes,
    BudgetKind::OutputBytes,
    BudgetKind::ReplayWork,
    BudgetKind::Vertices,
    BudgetKind::Edges,
    BudgetKind::StagedVertices,
    BudgetKind::StagedEdges,
    BudgetKind::ProofNodes,
    BudgetKind::ConditionAtoms,
    BudgetKind::NameBytes,
    BudgetKind::LiteralBytes,
    BudgetKind::SourceBytes,
];

pub(crate) fn ceiling(kind: BudgetKind, amount: u64) -> Limits {
    let mut limits = Limits::default();
    match kind {
        BudgetKind::Work => limits.work = amount,
        BudgetKind::ArithmeticSteps => limits.arithmetic_steps = amount,
        BudgetKind::ConditionSteps => limits.condition_steps = amount,
        BudgetKind::ComponentBits => limits.component_bits = amount,
        // Keep the independently varied intermediate ceiling a valid policy.
        BudgetKind::IntermediateBits => {
            limits.intermediate_bits = amount;
            limits.component_bits = amount.min(limits.component_bits);
        }
        BudgetKind::Depth => limits.depth = amount,
        BudgetKind::StagedPayloadBytes => limits.staged_payload_bytes = amount,
        BudgetKind::PayloadBytes => limits.payload_bytes = amount,
        BudgetKind::WitnessBytes => limits.witness_bytes = amount,
        BudgetKind::OutputBytes => limits.output_bytes = amount,
        BudgetKind::ReplayWork => limits.replay_work = amount,
        BudgetKind::Vertices => limits.vertices = amount,
        BudgetKind::Edges => limits.edges = amount,
        BudgetKind::StagedVertices => limits.staged_vertices = amount,
        BudgetKind::StagedEdges => limits.staged_edges = amount,
        BudgetKind::ProofNodes => limits.proof_nodes = amount,
        BudgetKind::ConditionAtoms => limits.condition_atoms = amount,
        BudgetKind::NameBytes => limits.name_bytes = amount,
        BudgetKind::LiteralBytes => limits.literal_bytes = amount,
        BudgetKind::SourceBytes => limits.source_bytes = amount,
        _ => {}
    }
    limits
}

/// Costs exclude 37 prior Work; failed prefixes exclude it as well.
/// Zero dimensions execute with a zero ceiling rather than skipping the operation.
pub(crate) fn verify(
    kinds: &[BudgetKind],
    work: impl Fn(BudgetKind) -> u64,
    costs: &[u64],
    prefixes: &[u64],
    mut attempt: impl FnMut(BudgetKind, u64) -> TestResult<(bool, u64)>,
) -> TestResult {
    for ((kind, cost), prefix) in kinds
        .iter()
        .copied()
        .zip(costs.iter().copied())
        .zip(prefixes.iter().copied())
    {
        let amount = cost + if kind == BudgetKind::Work { 37 } else { 0 };
        let (accepted, spent) = attempt(kind, amount)?;
        assert!(accepted, "{kind:?} at {amount}");
        assert_eq!(spent, work(kind) + 37, "{kind:?}");
        if cost > 0 {
            let (accepted, spent) = attempt(kind, amount - 1)?;
            assert!(!accepted, "{kind:?} below {amount}");
            assert_eq!(spent, prefix + 37, "{kind:?} consumed prefix");
        }
    }
    Ok(())
}
