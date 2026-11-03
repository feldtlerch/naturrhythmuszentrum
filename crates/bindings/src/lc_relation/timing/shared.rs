// SPDX-License-Identifier: AGPL-3.0-only
//! One shared clock offset and independent sample errors remain separate joint constraints.

mod admit;
mod horizon;
mod project;
mod witness;

use super::{AT, Bounds, LcRelation, LcRelationError, Rational, storage};
use crate::{
    Budget, LcCertainty, LcClockParameters, LcClockPhaseParameters, LcConsistency, LcDeterminacy,
    LcFreeDirection, LcInterval, LcQuantity, LcTimingCover, LcWitness,
};
use nrz_cas::{limits::BudgetUsage, pi_bounds::PiBoundCertificate};
use std::rc::Rc;

/// Joint time model `t_i=nominal_i+offset+delta_i`, with one offset and separate errors.
#[derive(Clone, Debug)]
pub struct LcSharedTiming(Rc<Question>);

/// The two uncertainty layers retain distinct owners and their original common epoch.
#[derive(Debug)]
struct Question {
    /// Every original physical declaration, including total timestamp support.
    relation: LcRelation,
    /// Nominal observation coordinates in coherent seconds.
    nominal: Vec<Rational>,
    /// One independent finite error support per original observation.
    errors: Vec<Bounds>,
    /// One finite offset shared by every original timestamp.
    offset: Bounds,
    /// Original positive finite rate support for the exact auxiliary construction.
    rate: Bounds,
    /// Optional original physical initial-phase intersection; absence keeps all integer gauges.
    initial: Option<Bounds>,
    /// Conservative new exact ownership, excluding the original relation.
    bytes: u64,
}

/// Original physical witness checked against both independent errors and one common offset.
#[derive(Clone, Debug)]
pub struct LcSharedTimingWitness {
    /// Identical original joint timing question, not just marginal time boxes.
    timing: LcSharedTiming,
    /// Full original physical tuple after every equation and input check.
    original: LcWitness,
    /// The same checked offset used in all observation equations.
    offset: Rational,
    /// Complete cumulative witness-checking snapshots.
    usage: [BudgetUsage; 2],
}

/// Complete joint rate/offset fibres with both timing layers retained.
#[derive(Clone, Debug)]
pub struct LcSharedTimingCover(Rc<Cover>);

/// The auxiliary initial phase is `beta=alpha+a*offset`, retaining physical alpha separately.
#[derive(Debug)]
struct Cover {
    /// Original joint question, including every per-sample support.
    timing: LcSharedTiming,
    /// Exact correlated `beta,a` domain, with independent sample times and offset faces.
    effective: LcTimingCover,
    /// Restored physical tuple checking every original constraint and both timing layers.
    witness: LcSharedTimingWitness,
    /// Cumulative construction Work and capacity snapshots.
    usage: [BudgetUsage; 2],
    /// Conservative retained metadata including all original and auxiliary owners.
    bytes: u64,
}

impl LcSharedTiming {
    /// Admit both timing layers with optional bounded physical initial phase and positive rate support.
    ///
    /// Each original time constraint must contain the full nominal-plus-error-plus-offset
    /// image. Narrower coupled restrictions require another adapter and are never dropped.
    /// # Errors
    /// Refuses unsupported supports or phase, omitted original restrictions and exhausted Work.
    pub fn new(
        relation: LcRelation,
        nominal: Vec<Rational>,
        errors: &[LcInterval],
        offset: &LcInterval,
        budget: &mut Budget,
    ) -> Result<Self, LcRelationError> {
        budget.work(1, AT)?;
        storage::retained(storage::size(errors.len())?.saturating_mul(2_048), budget)?;
        let offset = super::admit::bounds(offset, budget)?;
        let errors = errors
            .iter()
            .map(|error| super::admit::bounds(error, budget))
            .collect::<Result<Vec<_>, _>>()?;
        let premises = admit::question(&relation, &nominal, &errors, &offset, budget)?;
        let bytes = nominal
            .iter()
            .chain(errors.iter().flatten())
            .chain(&offset)
            .chain(&premises.rate)
            .chain(premises.initial.iter().flatten())
            .fold(2_048_u64, |sum, value| {
                sum.saturating_add(storage::rational(value))
            });
        storage::retained(
            bytes.saturating_add(relation.retained_payload_bytes()),
            budget,
        )?;
        Ok(Self(Rc::new(Question {
            relation,
            nominal,
            errors,
            offset,
            rate: premises.rate,
            initial: premises.initial,
            bytes,
        })))
    }

    /// Original physical declaration and common epoch.
    #[must_use]
    pub fn relation(&self) -> &LcRelation {
        &self.0.relation
    }

    /// Nominal coordinates, without either error layer folded into their values.
    #[must_use]
    pub fn nominal_times(&self) -> &[Rational] {
        &self.0.nominal
    }

    /// Separate per-sample errors in original observation order.
    #[must_use]
    pub fn errors(&self) -> &[Bounds] {
        &self.0.errors
    }

    /// Single shared offset domain.
    #[must_use]
    pub fn offset(&self) -> &Bounds {
        &self.0.offset
    }

    /// Original bounded physical initial phase; absence retains every common integer gauge.
    #[must_use]
    pub fn initial_phase_support(&self) -> Option<&Bounds> {
        self.0.initial.as_ref()
    }

    /// Exact original joint question identity.
    #[must_use]
    pub fn shares_allocation(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }

    /// Complete exact projection retaining shared-offset and independent-error correlation.
    /// # Errors
    /// Refuses unsupported physical constraints, incomplete cycles and exhausted resources.
    pub fn pairs_cover(
        &self,
        cycles: Vec<Rational>,
        pi: &PiBoundCertificate,
        budget: &mut Budget,
    ) -> Result<LcSharedTimingCover, LcRelationError> {
        project::cover(self, cycles, pi, budget)
    }

    /// Check every original equation, each independent error and the identical shared offset.
    /// # Errors
    /// Refuses any original or joint constraint violation and exhausted Work.
    pub fn check_witness(
        &self,
        values: crate::LcValues,
        offset: Rational,
        budget: &mut Budget,
    ) -> Result<LcSharedTimingWitness, LcRelationError> {
        witness::check(self, values, offset, budget)
    }

    /// Conservative new joint-question metadata, excluding the original relation.
    #[must_use]
    pub fn retained_payload_bytes(&self) -> u64 {
        self.0.bytes
    }
}

impl LcSharedTimingWitness {
    /// Original two-layer timing question.
    #[must_use]
    pub const fn timing(&self) -> &LcSharedTiming {
        &self.timing
    }

    /// Exact physical tuple after all original checks.
    #[must_use]
    pub const fn original(&self) -> &LcWitness {
        &self.original
    }

    /// One offset shared by every restored timestamp.
    #[must_use]
    pub const fn offset(&self) -> &Rational {
        &self.offset
    }

    /// Cumulative snapshots include original and joint timing checks.
    #[must_use]
    pub const fn usage(&self) -> [BudgetUsage; 2] {
        self.usage
    }
}

impl LcSharedTimingCover {
    /// Original joint timing owner.
    #[must_use]
    pub fn timing(&self) -> &LcSharedTiming {
        &self.0.timing
    }

    /// Original complete physical relation, without auxiliary strips.
    #[must_use]
    pub fn relation(&self) -> &LcRelation {
        self.timing().relation()
    }

    /// Complete projected rate support with the joint offset/error fibres retained.
    #[must_use]
    pub fn rate(&self) -> &Bounds {
        self.0.effective.rate()
    }

    /// Original cycle associations, excluding the two derived offset faces.
    #[must_use]
    pub fn cycles(&self) -> &[Rational] {
        &self.0.effective.cycles()[..self.relation().observations()]
    }

    /// Original nonemptiness and complete branch coverage have separate checked witnesses.
    #[must_use]
    pub const fn certainty(&self) -> LcCertainty {
        LcCertainty::Certified
    }

    /// Projection determinacy retains the original physical free directions.
    /// # Errors
    /// Refuses quantities outside the retained physical scale projection.
    pub fn determinacy(&self, quantity: LcQuantity) -> Result<LcDeterminacy, LcRelationError> {
        if quantity == LcQuantity::InitialPhase {
            self.fixed_initial()?;
            return Ok(LcDeterminacy::DeterminedFinite);
        }
        self.0.effective.determinacy(quantity)
    }

    /// Named physical freedoms are distinct from either bounded measurement layer.
    /// # Errors
    /// Refuses unsupported physical projections.
    pub fn free_directions(
        &self,
        quantity: LcQuantity,
    ) -> Result<&'static [LcFreeDirection], LcRelationError> {
        if quantity == LcQuantity::InitialPhase {
            self.fixed_initial()?;
            return Ok(&[]);
        }
        self.0.effective.free_directions(quantity)
    }

    /// Physical marginals retain the same joint rate/offset/error owner.
    /// # Errors
    /// Refuses unsupported quantities and exhausted resources.
    pub fn bounds(
        &self,
        quantity: LcQuantity,
        budget: &mut Budget,
    ) -> Result<LcInterval, LcRelationError> {
        budget.work(1, AT)?;
        if quantity == LcQuantity::InitialPhase {
            LcInterval::exact(self.fixed_initial()?.clone().into(), budget)
        } else {
            self.0.effective.bounds(quantity, budget)
        }
    }

    /// Scalar initial phase is published only for an exact original physical declaration.
    fn fixed_initial(&self) -> Result<&Rational, LcRelationError> {
        let initial = self
            .timing()
            .initial_phase_support()
            .ok_or(LcRelationError::Projection)?;
        if initial[0] != initial[1] {
            return Err(LcRelationError::Projection);
        }
        Ok(&initial[0])
    }

    /// Original physical consistency is backed by the joint-equation witness.
    #[must_use]
    pub fn consistency(&self) -> LcConsistency {
        self.0.effective.consistency()
    }

    /// Exact shared-offset fibre at a retained positive rate.
    /// # Errors
    /// Refuses rates outside the recovered support and exhausted resources.
    pub fn offset_at_rate(
        &self,
        rate: &Rational,
        budget: &mut Budget,
    ) -> Result<Bounds, LcRelationError> {
        let phase = self.0.effective.phase_at_rate(rate, budget)?;
        let mut offset = self.timing().offset().clone();
        if let Some([lower, upper]) = self.timing().initial_phase_support() {
            let image = [
                phase[0].sub(upper, budget)?.div(rate, budget)?,
                phase[1].sub(lower, budget)?.div(rate, budget)?,
            ];
            crate::lc_relation::calibration::intersect(&mut offset, &image, budget)?;
        }
        Ok(offset)
    }

    /// Complete common integer gauge of the original physical initial phase.
    #[must_use]
    pub fn phase_gauge(&self) -> crate::LcPhaseGauge {
        self.0.effective.phase_gauge()
    }

    /// Local physical initial-phase fibre at a rate, before the retained common integer gauge.
    /// # Errors
    /// Refuses out-of-domain rates and exhausted exact arithmetic.
    pub fn phase_at_rate(
        &self,
        rate: &Rational,
        budget: &mut Budget,
    ) -> Result<[Rational; 2], LcRelationError> {
        let beta = self.0.effective.phase_at_rate(rate, budget)?;
        let mut phase = [
            beta[0].sub(&rate.mul(&self.timing().offset()[1], budget)?, budget)?,
            beta[1].sub(&rate.mul(&self.timing().offset()[0], budget)?, budget)?,
        ];
        if let Some(initial) = self.timing().initial_phase_support() {
            crate::lc_relation::calibration::intersect(&mut phase, initial, budget)?;
        }
        Ok(phase)
    }

    /// Clock-offset fibre at a selected rate and local physical initial phase.
    /// # Errors
    /// Refuses crossed phase/rate/offset marginals and exhausted resources.
    pub fn offset_at_phase(
        &self,
        rate: &Rational,
        phase: &Rational,
        budget: &mut Budget,
    ) -> Result<[Rational; 2], LcRelationError> {
        budget.work(1, AT)?;
        phase.check(budget, AT)?;
        let mut selected = [phase.clone(), phase.clone()];
        if let Some(initial) = self.timing().initial_phase_support() {
            crate::lc_relation::calibration::intersect(&mut selected, initial, budget)?;
        }
        let beta = self.0.effective.phase_at_rate(rate, budget)?;
        let mut offset = self.timing().offset().clone();
        let image = [
            beta[0].sub(phase, budget)?.div(rate, budget)?,
            beta[1].sub(phase, budget)?.div(rate, budget)?,
        ];
        crate::lc_relation::calibration::intersect(&mut offset, &image, budget)?;
        Ok(offset)
    }

    /// Reconstruct physical initial phase and clock offset together, with every original gauge.
    /// # Errors
    /// Refuses crossed joint coordinates, original violations and exhausted resources.
    pub fn witness_with_phase(
        &self,
        parameters: &LcClockPhaseParameters,
        budget: &mut Budget,
    ) -> Result<LcSharedTimingWitness, LcRelationError> {
        witness::with_phase(self.timing(), &self.0.effective, parameters, budget)
    }

    /// Restore physical zero initial phase, every timestamp and both uncertainty layers.
    /// # Errors
    /// Refuses parameters outside the retained joint domain and exhausted Work.
    pub fn witness(
        &self,
        parameters: &LcClockParameters,
        budget: &mut Budget,
    ) -> Result<LcSharedTimingWitness, LcRelationError> {
        witness::parameters(self.timing(), &self.0.effective, parameters, budget)
    }

    /// Original and joint-equation nonemptiness witness.
    #[must_use]
    pub fn feasibility_witness(&self) -> &LcSharedTimingWitness {
        &self.0.witness
    }

    /// Cumulative projection snapshots, including the original witness restoration.
    #[must_use]
    pub fn usage(&self) -> [BudgetUsage; 2] {
        self.0.usage
    }

    /// Complete conservative retained exact ownership.
    #[must_use]
    pub fn retained_payload_bytes(&self) -> u64 {
        self.0.bytes
    }
}
