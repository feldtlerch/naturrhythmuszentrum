// SPDX-License-Identifier: AGPL-3.0-only
//! One bounded forward phase set retains the complete original calibrated element source.

use super::{
    AT, LcCalibratedElementsCover, LcCalibratedElementsParameters, LcCalibratedElementsWitness,
    LcRelationError, Q, calibration, storage,
};
use crate::{Budget, LcCertainty, LcDeterminacy, LcJointElementsParameters, LcRelation};
use nrz_cas::{Rational, limits::BudgetUsage, pi_bounds::CheckedPiBounds};
use std::{cmp::Ordering, rc::Rc};

/// Complete single-arc forward image for a positive query-time window at fixed zero initial phase.
#[derive(Clone, Debug)]
pub struct LcCalibratedForwardCover(Rc<Data>);

/// Complete original authority, exact time/phase correlation and checked endpoint attainment.
#[derive(Debug)]
struct Data {
    /// Complete original coupled element, calibration and timing cover.
    source: LcCalibratedElementsCover,
    /// Positive deterministic query window relative to the unchanged original epoch.
    times: [Rational; 2],
    /// Exact lifted and wrapped phase image; both endpoints lie strictly inside the zero-cycle cell.
    phases: [Rational; 2],
    /// Existing checked circle evidence, reused without recomputing its expansion.
    pi: CheckedPiBounds,
    /// Both physical and query endpoints are attained in the complete original source.
    witnesses: [LcCalibratedForwardWitness; 2],
    /// Cumulative complete query construction snapshots.
    usage: [BudgetUsage; 2],
    /// Conservative retained source, query and endpoint ownership.
    bytes: u64,
}

/// One original physical tuple with an exact query time and its attained phase.
#[derive(Clone, Debug)]
pub struct LcCalibratedForwardWitness {
    /// Every original physical equation, observation and calibration declaration is checked.
    original: LcCalibratedElementsWitness,
    /// Exact selected time in the query window, relative to the original source epoch.
    time: Rational,
    /// Exact physical lifted phase, equal to its wrapped representative in the certified cell.
    phase: Rational,
    /// Cumulative original reconstruction and query substitution snapshots.
    usage: [BudgetUsage; 2],
}

impl LcCalibratedElementsCover {
    /// Query the complete bounded phase set over a positive time window in the original epoch.
    ///
    /// Original initial phase must intersect to exactly zero. The entire image must lie strictly
    /// below the existing checked lower full-turn bound, proving the single zero-cycle branch.
    /// # Errors
    /// Refuses missing/nonzero initial phase, unsupported wrap ranges, invalid windows and exhausted Work.
    pub fn forward_phase_window(
        &self,
        times: [Rational; 2],
        budget: &mut Budget,
    ) -> Result<LcCalibratedForwardCover, LcRelationError> {
        let before = budget.usage();
        fixed_zero(self.relation(), budget)?;
        calibration::contains(&times, &times[0], budget)?;
        calibration::positive(&times[1], budget)?;
        let phases = [
            self.rate()[0].mul(&times[0], budget)?,
            self.rate()[1].mul(&times[1], budget)?,
        ];
        let pi = self.0.effective.checked_pi().clone();
        let turn_lower = pi
            .contract()
            .lower
            .mul(&Rational::from_ratio(2, 1, budget)?, budget)?;
        if phases[1].compare(&turn_lower, budget)? != Ordering::Less {
            return Err(LcRelationError::CycleCover);
        }
        storage::retained(
            self.retained_payload_bytes()
                .saturating_mul(3)
                .saturating_add(16_384),
            budget,
        )?;
        let lower = point(self, &times, &phases[0], budget)?;
        let upper = budget.with_retained_payload(
            lower
                .original
                .original()
                .retained_payload_bytes()
                .saturating_add(self.retained_payload_bytes())
                .saturating_add(pi.retained_bytes())
                .saturating_add(8_192),
            AT,
            |budget| point(self, &times, &phases[1], budget),
        )?;
        let witnesses = [lower, upper];
        let bytes = witnesses.iter().fold(
            self.retained_payload_bytes()
                .saturating_add(pi.retained_bytes())
                .saturating_add(4_096),
            |sum, witness| {
                sum.saturating_add(witness.original.original().retained_payload_bytes())
                    .saturating_add(storage::rational(witness.original.calibration()))
                    .saturating_add(storage::rational(&witness.time))
                    .saturating_add(storage::rational(&witness.phase))
            },
        );
        let bytes = times.iter().chain(&phases).fold(bytes, |sum, value| {
            sum.saturating_add(storage::rational(value))
        });
        storage::retained(bytes, budget)?;
        Ok(LcCalibratedForwardCover(Rc::new(Data {
            source: self.clone(),
            times,
            phases,
            pi,
            witnesses,
            usage: [before, budget.usage()],
            bytes,
        })))
    }
}

impl LcCalibratedForwardCover {
    /// Complete original joint cover, including the independent complete branch proof.
    #[must_use]
    pub fn source(&self) -> &LcCalibratedElementsCover {
        &self.0.source
    }

    /// Every original physical declaration, observation and unchanged epoch.
    #[must_use]
    pub fn relation(&self) -> &LcRelation {
        self.source().relation()
    }

    /// Deterministic query-time support, distinct from any original observed timestamp.
    #[must_use]
    pub fn time_window(&self) -> &[Rational; 2] {
        &self.0.times
    }

    /// Exact complete closed phase interval, entirely within the single certified zero-cycle cell.
    #[must_use]
    pub fn phases(&self) -> &[Rational; 2] {
        &self.0.phases
    }

    /// Independent already-prepared circle evidence proving full branch coverage.
    #[must_use]
    pub fn pi_bounds(&self) -> &CheckedPiBounds {
        &self.0.pi
    }

    /// Complete coverage and physical attainment are certified for this restricted query.
    #[must_use]
    pub const fn certainty(&self) -> LcCertainty {
        LcCertainty::Certified
    }

    /// The original query determines one bounded continuous phase image.
    #[must_use]
    pub const fn determinacy(&self) -> LcDeterminacy {
        LcDeterminacy::DeterminedFinite
    }

    /// Full original and query witnesses for both exact phase endpoints.
    #[must_use]
    pub fn endpoint_witnesses(&self) -> &[LcCalibratedForwardWitness; 2] {
        &self.0.witnesses
    }

    /// Reconstruct a physical solution and a compatible query time at any rational point in the image.
    /// # Errors
    /// Refuses phases outside the complete image and exhausted resources.
    pub fn witness_at_phase(
        &self,
        phase: &Rational,
        budget: &mut Budget,
    ) -> Result<LcCalibratedForwardWitness, LcRelationError> {
        budget.with_retained_payload(self.0.bytes, AT, |budget| {
            calibration::contains(self.phases(), phase, budget)?;
            point(self.source(), self.time_window(), phase, budget)
        })
    }

    /// Complete construction snapshots preserve the caller's consumed prefix.
    #[must_use]
    pub fn usage(&self) -> [BudgetUsage; 2] {
        self.0.usage
    }

    /// Conservative complete source, query and endpoint ownership.
    #[must_use]
    pub fn retained_payload_bytes(&self) -> u64 {
        self.0.bytes
    }
}

impl LcCalibratedForwardWitness {
    /// All original physical and measurement declarations remain checked and retained.
    #[must_use]
    pub const fn original(&self) -> &LcCalibratedElementsWitness {
        &self.original
    }

    /// Compatible exact query time in the original epoch.
    #[must_use]
    pub const fn time(&self) -> &Rational {
        &self.time
    }

    /// Attained physical phase in the complete certified zero-cycle image.
    #[must_use]
    pub const fn phase(&self) -> &Rational {
        &self.phase
    }

    /// Original and query substitution snapshots on the cumulative ledger.
    #[must_use]
    pub const fn usage(&self) -> [BudgetUsage; 2] {
        self.usage
    }
}

/// Exact zero original initial phase fixes the same physical lift for every source tuple.
fn fixed_zero(relation: &LcRelation, budget: &mut Budget) -> Result<(), LcRelationError> {
    budget.work(1, AT)?;
    let mut initial = None;
    for input in relation.inputs() {
        budget.work(1, AT)?;
        if input.quantity() == Q::InitialPhase {
            let next = crate::lc_relation::phase::admit::bounds(input.interval(), budget)?;
            if let Some(previous) = &mut initial {
                calibration::intersect(previous, &next, budget)?;
            } else {
                initial = Some(next);
            }
        }
    }
    let initial = initial.ok_or(LcRelationError::Projection)?;
    if !initial[0].is_zero() || !initial[1].is_zero() {
        return Err(LcRelationError::Projection);
    }
    Ok(())
}

/// Every phase in the positive product interval has a nonempty rate fibre and original reconstruction.
fn point(
    source: &LcCalibratedElementsCover,
    times: &[Rational; 2],
    phase: &Rational,
    budget: &mut Budget,
) -> Result<LcCalibratedForwardWitness, LcRelationError> {
    let before = budget.usage();
    budget.work(1, AT)?;
    let mut rates = [phase.div(&times[1], budget)?, phase.div(&times[0], budget)?];
    calibration::intersect(&mut rates, source.rate(), budget)?;
    let rate = rates[0].clone();
    let original = original_at(source, &rate, budget)?;
    let time = phase.div(&rate, budget)?;
    calibration::contains(times, &time, budget)?;
    if rate.mul(&time, budget)?.compare(phase, budget)? != Ordering::Equal {
        return Err(LcRelationError::Equation);
    }
    Ok(LcCalibratedForwardWitness {
        original,
        time,
        phase: phase.clone(),
        usage: [before, budget.usage()],
    })
}

/// A sealed original tuple is reusable when its exact rate and initial phase match the query point.
fn original_at(
    source: &LcCalibratedElementsCover,
    rate: &Rational,
    budget: &mut Budget,
) -> Result<LcCalibratedElementsWitness, LcRelationError> {
    budget.work(1, AT)?;
    let retained = source.feasibility_witness();
    let values = retained.original().values();
    let same_rate = match values.rate.as_rational() {
        Some(original_rate) => original_rate.compare(rate, budget)? == Ordering::Equal,
        None => false,
    };
    let zero_initial = values
        .initial_phase
        .as_ref()
        .is_some_and(crate::ExactConstant::is_zero);
    if same_rate && zero_initial {
        return Ok(retained.clone());
    }
    let capacitance = source.capacitance_at_rate(rate, budget)?[0].clone();
    let inductance =
        Rational::one().div(&rate.pow(2, budget)?.mul(&capacitance, budget)?, budget)?;
    let gain = source
        .question()
        .calibration_at_inductance(&inductance, budget)?[0]
        .clone();
    source.witness(
        &LcCalibratedElementsParameters {
            physical: LcJointElementsParameters {
                rate: rate.clone(),
                initial_phase: Rational::zero(),
                gauge: Rational::zero(),
                capacitance,
                voltage: Rational::one(),
            },
            calibration: gain,
        },
        budget,
    )
}
