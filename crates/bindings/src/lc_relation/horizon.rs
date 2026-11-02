// SPDX-License-Identifier: AGPL-3.0-only
//! Published forward certainty horizons retain their exact joint phase/rate derivation.

use super::{AT, LcCertainty, LcPairsCover, LcRelation, LcRelationError, LcZeroTimes, storage};
use crate::{Budget, Descriptor, LcQuantity};
use nrz_cas::{
    Rational,
    lattice::PiWidthHorizon,
    limits::BudgetUsage,
    pi_bounds::{CheckedPiBounds, PiBoundCertificate},
};
use std::rc::Rc;

pub use nrz_cas::lattice::PiHorizonValue as LcHorizonValue;

/// Original joint feasible set; neither alternative replaces it with marginal boxes.
#[derive(Clone, Debug)]
enum Source {
    /// Independent positive rate rectangle with exact starting phase zero.
    Rectangle(LcZeroTimes),
    /// Retained correlated phase-at-rate fibres and complete original cycle branch.
    Pairs(LcPairsCover),
}

impl Source {
    /// Exact original physical question.
    fn relation(&self) -> &LcRelation {
        match self {
            Self::Rectangle(cover) => cover.relation(),
            Self::Pairs(cover) => cover.relation(),
        }
    }

    /// Conservative shared original source ownership.
    fn bytes(&self) -> u64 {
        match self {
            Self::Rectangle(cover) => cover.retained_payload_bytes(),
            Self::Pairs(cover) => cover.retained_payload_bytes(),
        }
    }
}

/// Eventual loss of phase exclusion after an explicit future reference, in original clock seconds.
#[derive(Clone, Debug)]
pub struct LcHorizon(Rc<Data>);

/// Exact endpoint parameters give the whole lifted future image by convexity.
#[derive(Debug)]
struct Data {
    /// Complete original relation and constructive feasible family.
    source: Source,
    /// Lower and upper extremizers, each in starting-phase/rate order.
    extrema: [[Rational; 2]; 2],
    /// Checked affine future width and finite/infinite/unresolved full-turn threshold.
    mathematical: PiWidthHorizon,
    /// Seconds and duration difference kind; no affine clock offset is inferred.
    descriptor: Descriptor,
    /// Before/after exact derivation counters.
    usage: [BudgetUsage; 2],
    /// Conservative newly retained exact ownership.
    bytes: u64,
}

impl LcZeroTimes {
    /// Derive the independent rectangle horizon `2*pi/(a_max-a_min)`.
    /// # Errors
    /// Refuses false pi evidence and exhausted resources; the complete time family survives.
    pub fn forward_horizon(
        &self,
        pi: &PiBoundCertificate,
        budget: &mut Budget,
    ) -> Result<LcHorizon, LcRelationError> {
        let before = budget.usage();
        budget.work(1, AT)?;
        let pi = pi.check(budget)?;
        let extrema = [
            [Rational::zero(), self.family().rate()[0].clone()],
            [Rational::zero(), self.family().rate()[1].clone()],
        ];
        LcHorizon::new(
            Source::Rectangle(self.clone()),
            extrema,
            Rational::zero(),
            pi,
            &before,
            budget,
        )
    }
}

impl LcHorizon {
    /// Preserve a proved correlated pair extremization, including the prepared pi certificate.
    pub(super) fn pairs(
        source: LcPairsCover,
        extrema: [[Rational; 2]; 2],
        reference: Rational,
        before: &BudgetUsage,
        budget: &mut Budget,
    ) -> Result<Self, LcRelationError> {
        let pi = source.pi_bounds().clone();
        Self::new(
            Source::Pairs(source),
            extrema,
            reference,
            pi,
            before,
            budget,
        )
    }

    /// Exact positive-source extrema derive the entire future lifted interval.
    fn new(
        source: Source,
        extrema: [[Rational; 2]; 2],
        reference: Rational,
        pi: CheckedPiBounds,
        before: &BudgetUsage,
        budget: &mut Budget,
    ) -> Result<Self, LcRelationError> {
        budget.work(1, AT)?;
        let width = [
            extrema[1][0].sub(&extrema[0][0], budget)?,
            extrema[1][1].sub(&extrema[0][1], budget)?,
        ];
        let mathematical = PiWidthHorizon::check(width, reference, pi, budget)?;
        let bytes = extrema.iter().flatten().fold(
            mathematical.retained_payload_bytes().saturating_add(2_048),
            |sum, v| sum.saturating_add(storage::rational(v)),
        );
        storage::retained(
            bytes
                .saturating_add(source.bytes())
                .saturating_add(source.relation().retained_payload_bytes()),
            budget,
        )?;
        Ok(Self(Rc::new(Data {
            source,
            extrema,
            mathematical,
            descriptor: LcQuantity::NaturalTime.descriptor()?,
            usage: [*before, budget.usage()],
            bytes,
        })))
    }

    /// Original simultaneous relation with its unchanged common epoch and branch constraints.
    #[must_use]
    pub fn relation(&self) -> &LcRelation {
        self.0.source.relation()
    }

    /// Coherent seconds with duration difference meaning.
    #[must_use]
    pub fn descriptor(&self) -> &Descriptor {
        &self.0.descriptor
    }

    /// Exact affine width, future reference and constant evidence.
    #[must_use]
    pub fn mathematical(&self) -> &PiWidthHorizon {
        &self.0.mathematical
    }

    /// Exact value or explicit infinity/unresolved outcome; no cutoff determines this result.
    #[must_use]
    pub fn value(&self) -> &LcHorizonValue {
        self.0.mathematical.value()
    }

    /// Retained feasible extremizers in starting-phase/rate order, on the original local lift.
    #[must_use]
    pub fn extremal_parameters(&self) -> &[[Rational; 2]; 2] {
        &self.0.extrema
    }

    /// Whole-interval existence uses the admitted continuous rectangle or convex strip family.
    #[must_use]
    pub fn certainty(&self) -> LcCertainty {
        match self.value() {
            LcHorizonValue::Finite(_) | LcHorizonValue::Infinite => LcCertainty::Certified,
            LcHorizonValue::Undecided => LcCertainty::Undecided,
        }
    }

    /// Original correlated pair cover, when this horizon was recovered from observations.
    #[must_use]
    pub fn pairs_cover(&self) -> Option<&LcPairsCover> {
        match &self.0.source {
            Source::Pairs(cover) => Some(cover),
            Source::Rectangle(_) => None,
        }
    }

    /// Original independent complete zero-time family, when applicable.
    #[must_use]
    pub fn zero_times(&self) -> Option<&LcZeroTimes> {
        match &self.0.source {
            Source::Rectangle(cover) => Some(cover),
            Source::Pairs(_) => None,
        }
    }

    /// Cumulative derivation snapshots, excluding numerical lowering.
    #[must_use]
    pub fn usage(&self) -> [BudgetUsage; 2] {
        self.0.usage
    }

    /// Conservative new exact ownership, excluding shared original sources.
    #[must_use]
    pub fn retained_payload_bytes(&self) -> u64 {
        self.0.bytes
    }

    /// Conservative scoped source ownership for downstream numerical checking.
    #[must_use]
    pub fn source_payload_bytes(&self) -> u64 {
        self.0
            .source
            .bytes()
            .saturating_add(self.relation().retained_payload_bytes())
    }
}
