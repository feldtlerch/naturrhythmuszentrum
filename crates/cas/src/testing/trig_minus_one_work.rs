// SPDX-License-Identifier: AGPL-3.0-only
//! Complete recipe operations retain prior usage and compare transactional rollback.

use super::common::{TestResult, default_budget};
use super::half_pi_limits::ceiling;
use super::trig_minus_one::{fixture, premises};
use crate::test_api::limits::{Budget, BudgetKind};
use crate::test_api::proof::{Judgment, TrigRecipeOutcome, VerifiedWitness};
use crate::test_api::{Cas, Fault, FaultKind, Handle, InvalidCode, Location};

#[derive(Clone, Copy, Debug)]
pub(crate) enum Operation {
    Produce,
    Export,
    Replay,
}

enum Action {
    Produce(Handle, [Handle; 2], Judgment),
    Export(VerifiedWitness),
    Replay(Vec<u8>, Judgment),
}

impl Action {
    fn run(&self, cas: &mut Cas, budget: &mut Budget) -> Result<(), Fault> {
        match self {
            Self::Produce(function, [guard, equality], expected) => {
                let result = cas.trig_atan_minus_one(function, guard, equality, budget)?;
                let TrigRecipeOutcome::Proved(witness) = result else {
                    return Err(Fault::invalid(
                        InvalidCode::Rule,
                        Location::new("supported recipe fixture"),
                    ));
                };
                assert_eq!(witness.judgment(), expected);
                Ok(())
            }
            Self::Export(witness) => cas.export_witness(witness, budget).map(|_| ()),
            Self::Replay(bytes, expected) => cas.replay(bytes, expected, budget).map(|_| ()),
        }
    }
}

pub(crate) fn attempt(
    kind: usize,
    operation: Operation,
    resource: BudgetKind,
    amount: u64,
) -> TestResult<(bool, u64)> {
    let (mut f, function) = fixture(kind, false)?;
    let proofs = premises(&mut f)?;
    let (mut cas, action) = match operation {
        Operation::Produce => (f.cas, Action::Produce(function, proofs, f.expected)),
        Operation::Export | Operation::Replay => {
            let TrigRecipeOutcome::Proved(witness) = f.cas.trig_atan_minus_one(
                &function,
                &proofs[0],
                &proofs[1],
                &mut default_budget()?,
            )?
            else {
                return Err("supported recipe".into());
            };
            if matches!(operation, Operation::Export) {
                (f.cas, Action::Export(*witness))
            } else {
                let bytes = f.cas.export_witness(&witness, &mut default_budget()?)?;
                let (receiver, _) = fixture(kind, true)?;
                (receiver.cas, Action::Replay(bytes, receiver.expected))
            }
        }
    };
    let before = cas.canonical_snapshot(&mut default_budget()?)?;
    let counts = cas.counts();
    let limits = ceiling(resource, amount);
    let mut budget = Budget::new(limits.clone())?;
    budget.work(37, Location::new("prior recipe Work"))?;
    let result =
        budget.with_retained_payload(19, Location::new("prior recipe payload"), |budget| {
            action.run(&mut cas, budget)
        });
    assert_eq!(budget.limits(), &limits);
    if let Err(fault) = result {
        assert!(
            matches!(fault.kind, FaultKind::Exhausted { budget, .. } if budget == resource),
            "{operation:?} {resource:?}: {fault:?}"
        );
        assert_eq!(cas.counts(), counts);
        assert_eq!(cas.canonical_snapshot(&mut default_budget()?)?, before);
        Ok((false, budget.usage().work))
    } else {
        Ok((true, budget.usage().work))
    }
}
