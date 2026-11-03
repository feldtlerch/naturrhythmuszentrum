// SPDX-License-Identifier: AGPL-3.0-only
//! Whole-question inclusion checks original physical inputs and every declared measurement support.

use super::{AT, LcRelationError, Question, storage};
use crate::{Budget, LcInputRefinement, LcSampleTimes, LcSharedClock, LcSharedTiming};
use nrz_cas::{Rational, limits::BudgetUsage};
use std::{cmp::Ordering, rc::Rc};

/// Exact sufficient inclusion of one original clock measurement question in another.
#[derive(Clone, Debug)]
pub struct LcClockRefinementInputs(pub(super) Rc<Data>);

/// Physical input inclusion and the unchanged nominal-coordinate map retain every original equation.
#[derive(Debug)]
pub(super) struct Data {
    /// Exact previous joint question authority.
    pub(super) previous: Question,
    /// Complete refined question, including every added original observation.
    pub(super) refined: Question,
    /// Independent exact original-physical-input inclusion.
    pub(super) original: LcInputRefinement,
    /// Before/after cumulative checking snapshots.
    usage: [BudgetUsage; 2],
    /// Conservative retained inclusion metadata.
    bytes: u64,
}

impl LcSharedClock {
    /// Check whole-question inclusion with unchanged old nominal coordinates and a contained declared offset.
    /// # Errors
    /// Refuses changed old nominal coordinates, weakened physical inputs, offset widening and exhausted resources.
    pub fn refinement_inputs(
        &self,
        refined: &Self,
        budget: &mut Budget,
    ) -> Result<LcClockRefinementInputs, LcRelationError> {
        let before = budget.usage();
        budget.work(1, AT)?;
        let original = self
            .relation()
            .refinement_inputs(refined.relation(), budget)?;
        declarations(
            self.nominal_times(),
            refined.nominal_times(),
            self.offset(),
            refined.offset(),
            budget,
        )?;
        finish(
            Question::Clock(self.clone()),
            Question::Clock(refined.clone()),
            original,
            &before,
            budget,
        )
    }
}

impl LcSharedTiming {
    /// Check physical input inclusion plus unchanged old nominal coordinates and contained declared error layers.
    /// # Errors
    /// Refuses weakened physical inputs, changed nominal coordinates, widened offsets/errors and exhausted resources.
    pub fn refinement_inputs(
        &self,
        refined: &Self,
        budget: &mut Budget,
    ) -> Result<LcClockRefinementInputs, LcRelationError> {
        let before = budget.usage();
        budget.work(1, AT)?;
        let original = self
            .relation()
            .refinement_inputs(refined.relation(), budget)?;
        declarations(
            self.nominal_times(),
            refined.nominal_times(),
            self.offset(),
            refined.offset(),
            budget,
        )?;
        for (previous, next) in self.errors().iter().zip(refined.errors()) {
            contained(previous, next, budget)?;
        }
        finish(
            Question::SharedTiming(self.clone()),
            Question::SharedTiming(refined.clone()),
            original,
            &before,
            budget,
        )
    }
}

impl LcSampleTimes {
    /// Check original physical input inclusion with unchanged independent-error nominal coordinates.
    /// # Errors
    /// Refuses weakened physical inputs, changed old nominal coordinates or exhausted resources.
    pub fn refinement_inputs(
        &self,
        refined: &Self,
        budget: &mut Budget,
    ) -> Result<LcClockRefinementInputs, LcRelationError> {
        let before = budget.usage();
        budget.work(1, AT)?;
        let original = self
            .relation()
            .refinement_inputs(refined.relation(), budget)?;
        nominals(self.nominal_times(), refined.nominal_times(), budget)?;
        for (previous, next) in self.errors().iter().zip(refined.errors()) {
            contained(previous, next, budget)?;
        }
        finish(
            Question::Timing(self.clone()),
            Question::Timing(refined.clone()),
            original,
            &before,
            budget,
        )
    }
}

impl LcClockRefinementInputs {
    /// Previous independent timing question, when applicable.
    #[must_use]
    pub fn previous_timing(&self) -> Option<&LcSampleTimes> {
        self.0.previous.timing()
    }
    /// Refined independent timing question, when applicable.
    #[must_use]
    pub fn refined_timing(&self) -> Option<&LcSampleTimes> {
        self.0.refined.timing()
    }
    /// Original physical-input inclusion; measurement-domain inclusion is retained by this owner.
    #[must_use]
    pub fn original_inputs(&self) -> &LcInputRefinement {
        &self.0.original
    }
    /// Previous single-clock question, when applicable.
    #[must_use]
    pub fn previous_clock(&self) -> Option<&LcSharedClock> {
        self.0.previous.clock()
    }
    /// Refined single-clock question, when applicable.
    #[must_use]
    pub fn refined_clock(&self) -> Option<&LcSharedClock> {
        self.0.refined.clock()
    }
    /// Previous two-layer timing question, when applicable.
    #[must_use]
    pub fn previous_shared_timing(&self) -> Option<&LcSharedTiming> {
        self.0.previous.shared_timing()
    }
    /// Refined two-layer timing question, when applicable.
    #[must_use]
    pub fn refined_shared_timing(&self) -> Option<&LcSharedTiming> {
        self.0.refined.shared_timing()
    }
    /// Complete physical and measurement inclusion checking snapshots.
    #[must_use]
    pub fn usage(&self) -> [BudgetUsage; 2] {
        self.0.usage
    }
    /// Conservative retained inclusion metadata, excluding shared source owners.
    #[must_use]
    pub fn retained_payload_bytes(&self) -> u64 {
        self.0.bytes
    }
}

/// Existing observation identities retain the same reported coordinates and one contained offset.
fn declarations(
    previous: &[Rational],
    next: &[Rational],
    old_offset: &[Rational; 2],
    new_offset: &[Rational; 2],
    budget: &mut Budget,
) -> Result<(), LcRelationError> {
    nominals(previous, next, budget)?;
    contained(old_offset, new_offset, budget)
}

/// Existing sample identities retain their exact nominal coordinates in every measurement model.
fn nominals(
    previous: &[Rational],
    next: &[Rational],
    budget: &mut Budget,
) -> Result<(), LcRelationError> {
    budget.work(1, AT)?;
    if previous.len() > next.len() {
        return Err(LcRelationError::Observation);
    }
    for (old, new) in previous.iter().zip(next) {
        budget.work(1, AT)?;
        if old.compare(new, budget)? != Ordering::Equal {
            return Err(LcRelationError::Containment);
        }
    }
    Ok(())
}

/// Signed offset/error supports preserve both endpoints under one cumulative ledger.
pub(super) fn contained(
    previous: &[Rational; 2],
    next: &[Rational; 2],
    budget: &mut Budget,
) -> Result<(), LcRelationError> {
    budget.work(1, AT)?;
    let lower = next[0].compare(&previous[0], budget)? != Ordering::Less;
    let upper = next[1].compare(&previous[1], budget)? != Ordering::Greater;
    if !lower || !upper {
        return Err(LcRelationError::Containment);
    }
    Ok(())
}

/// Original owners remain retained without replacing them by their coincident marginal intersections.
fn finish(
    previous: Question,
    refined: Question,
    original: LcInputRefinement,
    before: &BudgetUsage,
    budget: &mut Budget,
) -> Result<LcClockRefinementInputs, LcRelationError> {
    budget.work(1, AT)?;
    let bytes = original.retained_payload_bytes().saturating_add(2_048);
    storage::retained(
        bytes
            .saturating_add(previous.bytes())
            .saturating_add(previous.relation().retained_payload_bytes())
            .saturating_add(refined.bytes())
            .saturating_add(refined.relation().retained_payload_bytes()),
        budget,
    )?;
    Ok(LcClockRefinementInputs(Rc::new(Data {
        previous,
        refined,
        original,
        usage: [*before, budget.usage()],
        bytes,
    })))
}
