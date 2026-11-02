// SPDX-License-Identifier: AGPL-3.0-only
//! Every selected continuation rechecks the original attachment in its own later stratum.

use super::{
    AT, LcDrivenContinuations, LcDrivenError, LcDrivenStatus as Status, LcDrivenStratum, Row,
};
use crate::{Heap, LcSpectralStatus};
use nrz_bindings::{AxisPhaseOutcome, Budget};
use nrz_scalar::{ArithmeticContext, Scalar};

impl<P: Scalar + 'static> LcDrivenContinuations<P> {
    /// Execute the one invariant/decision/current/phase attempt on one cumulative ledger.
    ///
    /// Every point's decision precedes every current publication, and every current attempt
    /// precedes the phase stratum. No retry or default branch can refresh the caller budget.
    /// # Errors
    /// Refuses a repeated attempt or foreign owner/arithmetic. Individual row failures remain
    /// in their stable table slots; a resource refusal blocks every pending later node.
    pub fn execute(
        &mut self,
        heap: &mut Heap,
        context: &ArithmeticContext,
        budget: &mut Budget,
    ) -> Result<(), LcDrivenError> {
        if self.next != 0 {
            return Err(LcDrivenError::Identity);
        }
        for stratum in [
            LcDrivenStratum::Invariant,
            LcDrivenStratum::Decision,
            LcDrivenStratum::Current,
            LcDrivenStratum::Phase,
        ] {
            self.execute_stratum(stratum, heap, context, budget)?;
        }
        Ok(())
    }

    /// Execute exactly the next declared stratum on the common caller ledger.
    ///
    /// Resource refusal blocks the pending suffix; ordinary row failures retain independent
    /// successes. A selected attachment is never used without a fresh live-source check.
    /// # Errors
    /// Refuses out-of-order or repeated strata and foreign heap/arithmetic owners. Row failures
    /// are recorded in their slots; invariant preflight failures also return their typed cause.
    fn execute_stratum(
        &mut self,
        stratum: LcDrivenStratum,
        heap: &mut Heap,
        context: &ArithmeticContext,
        budget: &mut Budget,
    ) -> Result<(), LcDrivenError> {
        let index = stratum.index();
        if index != self.next {
            return Err(LcDrivenError::Identity);
        }
        self.next += 1;
        let before = budget.usage();
        let result = self.run(stratum, heap, context, budget);
        self.usage[index] = Some([before, budget.usage()]);
        if result.is_err() {
            self.stopped = true;
        }
        if self.stopped {
            self.block_pending();
        }
        result
    }

    /// The invariant producer and all three later tables share the same finite attempt.
    fn run(
        &mut self,
        stratum: LcDrivenStratum,
        heap: &mut Heap,
        context: &ArithmeticContext,
        budget: &mut Budget,
    ) -> Result<(), LcDrivenError> {
        self.invariants.check_owner(heap, context)?;
        super::admission::admit(self.inventory.observed_capacity_bytes, budget)?;
        if self.stopped {
            return Ok(());
        }
        if stratum == LcDrivenStratum::Invariant {
            self.invariants.execute(heap, context, budget)?;
            self.stopped = self.invariants.rows.iter().enumerate().any(|(index, _)| {
                matches!(
                    self.invariants.status(index),
                    Some(LcSpectralStatus::ResourceRefused | LcSpectralStatus::Blocked)
                )
            });
            return Ok(());
        }
        let slot = stratum.index() - 1;
        for row in &mut self.rows {
            if row.status[slot] != Status::Pending {
                continue;
            }
            let result = budget
                .work(1, AT)
                .map_err(LcDrivenError::from)
                .and_then(|()| row.run(stratum, heap, context, budget));
            match result {
                Ok(status) => row.status[slot] = status,
                Err(cause) => {
                    let status = cause.status();
                    row.status[slot] = status;
                    row.failure[slot] = Some(cause);
                    if status == Status::ResourceRefused {
                        self.stopped = true;
                        break;
                    }
                }
            }
        }
        Ok(())
    }

    /// No pending output is replaced with a numerical zero after the common attempt stops.
    fn block_pending(&mut self) {
        for row in &mut self.rows {
            for status in &mut row.status {
                if *status == Status::Pending {
                    *status = Status::Blocked;
                }
            }
        }
    }
}

impl<P: Scalar + 'static> Row<P> {
    /// Revalidate the original selected case before entering its later dependent operation.
    fn run(
        &mut self,
        stratum: LcDrivenStratum,
        heap: &mut Heap,
        context: &ArithmeticContext,
        budget: &mut Budget,
    ) -> Result<Status, LcDrivenError> {
        if stratum == LcDrivenStratum::Decision {
            return self.decide(heap, context, budget);
        }
        let Some(selection) = &self.selection else {
            return Ok(Status::Blocked);
        };
        let outcome = selection.current(heap, context, budget)?.outcome();
        match outcome {
            AxisPhaseOutcome::Undefined => Ok(if stratum == LcDrivenStratum::Current {
                Status::DomainUnavailable
            } else {
                Status::Undefined
            }),
            AxisPhaseOutcome::Undecided => Ok(Status::Undecided),
            AxisPhaseOutcome::Negative | AxisPhaseOutcome::Positive => {
                match stratum {
                    LcDrivenStratum::Current => {
                        self.current =
                            Some(heap.publish_phasor_solve(&self.plan.current, context, budget)?);
                    }
                    LcDrivenStratum::Phase => {
                        selection.phase(heap, context, budget)?;
                    }
                    LcDrivenStratum::Invariant | LcDrivenStratum::Decision => {
                        return Err(LcDrivenError::Identity);
                    }
                }
                Ok(Status::Successful)
            }
        }
    }

    /// Consume the one predeclared certificate attempt over the actual original impedance.
    fn decide(
        &mut self,
        heap: &Heap,
        context: &ArithmeticContext,
        budget: &mut Budget,
    ) -> Result<Status, LcDrivenError> {
        let frozen = heap
            .freeze_phasor_axis_phase_input(
                &self.plan.impedance,
                &self.plan.current.solve().operands()[0],
                budget,
            )
            .map_err(crate::LiveDecisionError::from)?;
        let prepared = heap.prepare_axis_phase(
            frozen,
            &mut self.plan.phase.case().attempts(1),
            &self.plan.enclosure,
            &self.plan.phase,
            context,
            budget,
        )?;
        let selection = heap.publish_axis_phase(prepared, budget)?;
        let outcome = selection.current(heap, context, budget)?.outcome();
        self.verdict = Some(outcome);
        self.selection = Some(selection);
        Ok(if outcome == AxisPhaseOutcome::Undecided {
            Status::Undecided
        } else {
            Status::Successful
        })
    }
}
