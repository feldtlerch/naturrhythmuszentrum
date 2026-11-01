// SPDX-License-Identifier: AGPL-3.0-only
//! Both native and original block routes exercise the same literal cumulative resource contract.

use super::super::{Case, TestResult, ledger, phasor::BlockCase};
use nrz_bindings::{Budget, Limits, readings::algebra::Location};
use nrz_matrix::{Heap, PreparedAxisPhase, PublishedAxisPhase};
use nrz_scalar::{ArithmeticContext, Scalar};

/// Original target, represented native center, full radius and covering certificate.
pub(super) const CASES: [(i64, i64, &str, &str, &str, &str); 4] = [
    (1000, -75, "-75", "0.25", "-76", "-74"),
    (2000, 0, "0", "0", "0", "0"),
    (2000, 0, "0", "0.25", "-1/4", "1/4"),
    (4000, 75, "75", "0.25", "74", "76"),
];
/// Previous caller operations consume both Work and retained staging capacity.
pub(super) const PRIOR: u64 = 17;
/// Earlier caller charges have an independent accounting location.
pub(super) const AT: Location = Location::new("axis-phase prior caller prefix");

/// Preparation/publication/current/reciprocal Work and staged/output/owner/input bytes.
#[derive(Clone, Copy)]
pub(super) struct Pin {
    /// Preparation, publication, current read and reciprocal use on the actual caller ledger.
    pub(super) work: [u64; 4],
    /// Peak preparation staging, output ceiling, retained publication and frozen input.
    pub(super) bytes: [u64; 4],
}

/// The provider changes source representation without changing the original sign question.
pub(super) enum Source<P: Scalar> {
    /// Native normalized scalar source and both original radius versions.
    Native(Box<Case<P>>),
    /// Actual original impedance block and every stored copy.
    Block(Box<BlockCase<P>>),
}
impl<P: Scalar + 'static> Source<P> {
    /// Construct all declarations, input bindings and certified pi before the measured consumer.
    pub(super) fn new(index: usize, block: bool) -> TestResult<Self> {
        let (frequency, reactance, center, radius, _, _) = CASES[index];
        if block {
            Ok(Self::Block(Box::new(BlockCase::new(
                frequency,
                reactance,
                radius,
                reactance != 0,
            )?)))
        } else {
            Ok(Self::Native(Box::new(Case::new(
                frequency, center, radius,
            )?)))
        }
    }
    /// The original allocating owner remains the only live authority.
    pub(super) fn heap(&self) -> &Heap {
        match self {
            Self::Native(case) => &case.as_ref().heap,
            Self::Block(case) => &case.as_ref().physical.heap,
        }
    }
    /// Original arithmetic proof revision.
    pub(super) fn context(&self) -> &ArithmeticContext {
        match self {
            Self::Native(case) => &case.as_ref().context,
            Self::Block(case) => &case.as_ref().context,
        }
    }
    /// Freeze and prepare on the actual caller ledger, with one original finite attempt identity.
    pub(super) fn prepare(
        &self,
        index: usize,
        budget: &mut Budget,
    ) -> TestResult<PreparedAxisPhase<P>> {
        let (_, _, _, _, lower, upper) = CASES[index];
        match self {
            Self::Native(case) => case.prepare(lower, upper, budget),
            Self::Block(case) => case.prepare(lower, upper, budget),
        }
    }
    /// Complete setup for measuring independent current-use limits.
    pub(super) fn published(&self, index: usize) -> TestResult<PublishedAxisPhase<P>> {
        let mut budget = ledger()?;
        Ok(self
            .heap()
            .publish_axis_phase(self.prepare(index, &mut budget)?, &mut budget)?)
    }
}
/// Only the relevant allowance changes; production defaults remain the outer policy.
pub(super) fn work(required: u64) -> TestResult<Budget> {
    Ok(Budget::new(Limits {
        work: required,
        ..Limits::default()
    })?)
}
