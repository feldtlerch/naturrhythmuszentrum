// SPDX-License-Identifier: AGPL-3.0-only
//! One finite shared-voltage statistical family follows the selected original current continuations.

use super::{AT, LcDrivenContinuations, LcDrivenError as Error, LcDrivenStatus as Status};
use crate::{CompanionExecution, Heap, PhasorStatistics, ReadingBinding};
use nrz_bindings::{
    Budget,
    readings::{
        ParameterErrorReading,
        algebra::limits::{BudgetKind, BudgetUsage},
    },
};
use nrz_scalar::{ArithmeticContext, LiteralLimits, Scalar};
use std::rc::Rc;

/// A predeclared two-current statistical consumer inside one cumulative driven attempt.
///
/// The original variance, cross-covariance and variance laws precede execution. Missing
/// selected means block the whole family; no marginal or zero placeholder escapes.
#[derive(Debug)]
pub struct LcDrivenStatistics<P: Scalar> {
    /// Original finite row and continuation owner, retained without copying its outputs.
    driven: LcDrivenContinuations<P>,
    /// Original left variance, cross block and right variance error transports.
    exact: [Rc<ParameterErrorReading>; 3],
    /// One actual original variance and parameter-error binding pair.
    sources: [ReadingBinding<P>; 2],
    /// Two distinct original row ordinals in declared left/right order.
    rows: [usize; 2],
    /// Caller-selected bounded coefficient import policy.
    literals: LiteralLimits,
    /// Complete statistical result exists only after successful later execution.
    result: Option<PhasorStatistics<P>>,
    /// Historical single-attempt outcome, independently revalidated on numerical access.
    status: Status,
    /// Original failure remains inspectable without manufacturing numerical output.
    failure: Option<Error>,
    /// Caller snapshots around the complete driven and statistical attempt.
    usage: Option<[BudgetUsage; 2]>,
}

impl<P: Scalar + 'static> LcDrivenStatistics<P> {
    /// Attach all three exact scalar covariance slots before the original driven attempt.
    /// # Errors
    /// Refuses used owners, wrong row order, substituted equations, sources or targets and resources.
    pub fn prepare(
        driven: LcDrivenContinuations<P>,
        exact: [Rc<ParameterErrorReading>; 3],
        sources: [ReadingBinding<P>; 2],
        rows: [usize; 2],
        literals: LiteralLimits,
        budget: &mut Budget,
    ) -> Result<Self, Error> {
        if driven.next != 0 || rows[0] == rows[1] {
            return Err(Error::Identity);
        }
        for (definition, pair) in exact.iter().zip([[0, 0], [0, 1], [1, 1]]) {
            definition
                .check_sources(sources[0].reading(), sources[1].reading(), budget)
                .map_err(crate::CompanionExecutionError::from)?;
            let law = definition.target().phasor_law().ok_or(Error::Identity)?;
            for ((equation, target), side) in law
                .equations()
                .iter()
                .zip(definition.target().targets())
                .zip(pair)
            {
                budget.work(1, AT)?;
                let point = driven.point(rows[side]).ok_or(Error::Identity)?;
                let current = point.current_publication();
                if !equation.shares_allocation(current.solve().definition())
                    || target.entries().len() != 1
                    || !current
                        .coverage()
                        .readings()
                        .iter()
                        .any(|item| item.reading() == target)
                {
                    return Err(Error::Identity);
                }
            }
        }
        let first = driven
            .point(rows[0])
            .ok_or(Error::Identity)?
            .current_publication()
            .solve()
            .operands()[1]
            .whole();
        let second = driven
            .point(rows[1])
            .ok_or(Error::Identity)?
            .current_publication()
            .solve()
            .operands()[1]
            .whole();
        if !Rc::ptr_eq(&first.group, &second.group) || first.index != second.index {
            return Err(Error::Identity);
        }
        capacity::<P>(budget)?;
        Ok(Self {
            driven,
            exact,
            sources,
            rows,
            literals,
            result: None,
            status: Status::Pending,
            failure: None,
            usage: None,
        })
    }

    /// Execute the original strata and then all three statistical blocks on the same caller ledger.
    ///
    /// Refusals retain their typed cause and leave the statistical family absent. An unavailable
    /// selected current blocks statistics while preserving independent row and invariant results.
    /// # Errors
    /// Refuses any second attempt, including after a resource or domain refusal.
    pub fn execute(
        &mut self,
        heap: &mut Heap,
        context: &ArithmeticContext,
        budget: &mut Budget,
    ) -> Result<(), Error> {
        if self.status != Status::Pending {
            return Err(Error::Identity);
        }
        self.status = Status::Blocked;
        let before = budget.usage();
        let result = self.run(heap, context, budget);
        self.usage = Some([before, budget.usage()]);
        match result {
            Ok(Some(result)) => {
                self.result = Some(result);
                self.status = Status::Successful;
            }
            Ok(None) => {}
            Err(cause) => {
                self.status = cause.status();
                self.failure = Some(cause);
            }
        }
        Ok(())
    }

    /// Original stable rows and their independent invariant remain accessible after a statistical refusal.
    #[must_use]
    pub const fn driven(&self) -> &LcDrivenContinuations<P> {
        &self.driven
    }
    /// Historical outcome; successful numerical access separately revalidates its entire closure.
    #[must_use]
    pub const fn status(&self) -> Status {
        self.status
    }
    /// Original typed statistical or earlier execution refusal, if any.
    #[must_use]
    pub const fn failure(&self) -> Option<&Error> {
        self.failure.as_ref()
    }
    /// Complete attempted prefix on the common caller ledger, excluding prior declaration admission.
    #[must_use]
    pub const fn usage(&self) -> Option<[BudgetUsage; 2]> {
        self.usage
    }
    /// Fixed new wrapper ownership; shared exact definitions, driven owner and result payload are excluded.
    #[must_use]
    pub const fn wrapper_bytes(&self) -> usize {
        size_of::<Self>() - size_of::<LcDrivenContinuations<P>>()
    }
    /// Lend the complete shared-voltage family only while both selected decisions and all targets are live.
    /// # Errors
    /// Refuses unavailable families, stale sources or selected outputs, foreign owners and resources.
    pub fn statistics(
        &self,
        heap: &Heap,
        context: &ArithmeticContext,
        budget: &mut Budget,
    ) -> Result<&PhasorStatistics<P>, Error> {
        for row in self.rows {
            self.driven.current(row, heap, context, budget)?;
        }
        let result = self.result.as_ref().ok_or(Error::Identity)?;
        result.moments(heap, context)?;
        Ok(result)
    }

    /// Every exact target is resolved to the actual result of its predeclared current producer.
    fn run(
        &mut self,
        heap: &mut Heap,
        context: &ArithmeticContext,
        budget: &mut Budget,
    ) -> Result<Option<PhasorStatistics<P>>, Error> {
        capacity::<P>(budget)?;
        self.driven.execute(heap, context, budget)?;
        if self.driven.stopped
            || self
                .rows
                .iter()
                .any(|&row| self.driven.rows[row].current.is_none())
        {
            return Ok(None);
        }
        let mut executions = Vec::new();
        executions
            .try_reserve_exact(3)
            .map_err(|_| crate::HeapError::Capacity)?;
        for (definition, pair) in self.exact.iter().zip([[0, 0], [0, 1], [1, 1]]) {
            let mut targets = Vec::new();
            targets
                .try_reserve_exact(2)
                .map_err(|_| crate::HeapError::Capacity)?;
            for (expected, side) in definition.target().targets().iter().zip(pair) {
                let current = self
                    .driven
                    .current(self.rows[side], heap, context, budget)?;
                let target = current
                    .readings()
                    .bindings()
                    .iter()
                    .find(|actual| actual.reading() == expected)
                    .ok_or(Error::Identity)?;
                targets.push(target.clone());
            }
            executions.push(CompanionExecution::new(
                Rc::clone(definition),
                self.sources.clone(),
                targets,
                self.literals,
                budget,
            )?);
        }
        let executions = executions.try_into().map_err(|_| Error::Identity)?;
        Ok(Some(heap.evaluate_phasor_statistics(
            &executions,
            context,
            budget,
        )?))
    }
}

/// The new fixed wrapper is scoped separately from all shared child and backend owners.
fn capacity<P: Scalar>(budget: &Budget) -> Result<(), Error> {
    let bytes = size_of::<LcDrivenStatistics<P>>() - size_of::<LcDrivenContinuations<P>>();
    let bytes = u64::try_from(bytes).map_err(|_| crate::HeapError::ExtentOverflow)?;
    budget.check(BudgetKind::StagedPayloadBytes, bytes, AT)?;
    budget.check(BudgetKind::OutputBytes, bytes, AT)?;
    Ok(())
}
