// SPDX-License-Identifier: AGPL-3.0-only
//! Complete inverse zero times with separate covering and total-existence evidence.

mod rectangle;

use super::{AT, LcCertainty, LcDeterminacy, LcRelation, LcRelationError, LcWitness, storage};
use crate::{Budget, Descriptor, LcQuantity};
use nrz_cas::{
    Rational,
    lattice::{PiZeroFamily, PiZeroTail},
    limits::BudgetUsage,
};
use rectangle::Rectangle;
use std::rc::Rc;

pub use nrz_cas::lattice::PiZeroBox as LcTimeBox;

/// Tail assurance never silently equates outer coverage with per-time feasibility.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LcTailStatus {
    /// Possible everywhere, numbering undecided; no per-time feasibility assertion.
    CoverageOnly,
    /// Feasible everywhere, numbering undecided; alternative oscillators realize every time.
    FeasibleEverywhere,
}

/// A complete inverse answer, including the exact epoch and every nonnegative cycle.
#[derive(Clone, Debug)]
pub struct LcZeroTimes(Rc<Data>);

/// Original physical authority is distinct from the mathematical integer-family certificate.
#[derive(Debug)]
struct Data {
    /// Every admitted original constraint and the named clock epoch.
    relation: LcRelation,
    /// Complete mathematical family, including the zero-spread exception.
    family: PiZeroFamily,
    /// Checked rectangular parameter map, keeping the chosen positive energy.
    rectangle: Rectangle,
    /// Coherent duration difference meaning shared by all boxes and the tail.
    descriptor: Descriptor,
    /// Before/after exact checking snapshots.
    usage: [BudgetUsage; 2],
    /// Conservative new ownership, excluding the shared relation.
    bytes: u64,
}

/// Failed overlap attempts retain the complete family and the unresolved obligation.
#[derive(Debug)]
pub enum LcTailOutcome {
    /// Exact minimal overlap certificate; assurance initially covers only the outer union.
    Certified(Box<LcTimeTail>),
    /// Exact zero spread proves there is no finite connected tail; all indices remain.
    SeparatedInfinite(LcZeroTimes),
    /// No tail start is asserted; the original complete covering family is still available.
    Undecided {
        /// Complete previous cover, with every index retained.
        cover: LcZeroTimes,
        /// Failed exact overlap obligation, including resource exhaustion.
        reason: LcRelationError,
    },
}

/// A checked overlap suffix, retaining all numbered boxes and its own assurance evidence.
#[derive(Clone, Debug)]
pub struct LcTimeTail {
    /// Exact original complete answer and physical declaration.
    cover: LcZeroTimes,
    /// Checked minimality and continued overlap, independent of feasibility.
    mathematical: PiZeroTail,
    /// Additional original-equation evidence for the admitted total parameter map.
    existence: Option<LcWitness>,
}

impl LcRelation {
    /// Query all nonnegative times of target phase zero from exact initial phase zero.
    ///
    /// This adapter accepts a positive rational rate rectangle, exact zero starting phase,
    /// and optional finite positive energy support. Every other constraint refuses.
    /// # Errors
    /// Refuses unsupported/empty inputs, nonzero phase and exhausted caller resources.
    pub fn zero_times(&self, budget: &mut Budget) -> Result<LcZeroTimes, LcRelationError> {
        let before = budget.usage();
        let (rectangle, rate) = Rectangle::check(self, budget)?;
        let family = PiZeroFamily::check(rate, budget)?;
        let descriptor = LcQuantity::NaturalTime.descriptor()?;
        let bytes = family
            .retained_payload_bytes()
            .saturating_add(rectangle.bytes())
            .saturating_add(2_048);
        storage::retained(bytes.saturating_add(self.retained_payload_bytes()), budget)?;
        Ok(LcZeroTimes(Rc::new(Data {
            relation: self.clone(),
            family,
            rectangle,
            descriptor,
            usage: [before, budget.usage()],
            bytes,
        })))
    }
}

impl LcZeroTimes {
    /// Identity of the complete immutable time question, not equality of rate endpoints.
    #[must_use]
    pub fn shares_allocation(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }

    /// Original relation, including its explicit common epoch.
    #[must_use]
    pub fn relation(&self) -> &LcRelation {
        &self.0.relation
    }

    /// Coherent seconds with duration difference kind.
    #[must_use]
    pub fn descriptor(&self) -> &Descriptor {
        &self.0.descriptor
    }

    /// Complete mathematical index domain and exact positive rate support.
    #[must_use]
    pub fn family(&self) -> &PiZeroFamily {
        &self.0.family
    }

    /// Exact zero target angle, with no rounded phase representative.
    #[must_use]
    pub fn target_phase(&self) -> Rational {
        Rational::zero()
    }

    /// The zero query retains infinitely many exact cycle identities.
    #[must_use]
    pub const fn determinacy(&self) -> LcDeterminacy {
        LcDeterminacy::DeterminedInfinite
    }

    /// Complete coverage and nonemptiness follow from the admitted rectangle map.
    #[must_use]
    pub const fn certainty(&self) -> LcCertainty {
        LcCertainty::Certified
    }

    /// Materialize any numbered closed box; index zero is the separate exact epoch.
    /// # Errors
    /// Refuses invalid indices or exhausted caller resources without modifying the family.
    pub fn member(
        &self,
        index: Rational,
        budget: &mut Budget,
    ) -> Result<LcTimeBox, LcRelationError> {
        Ok(self.0.family.member(index, budget)?)
    }

    /// Attempt the minimal overlap certificate while preserving coverage on every refusal.
    pub fn overlap_tail(&self, first: Rational, budget: &mut Budget) -> LcTailOutcome {
        let result = budget
            .work(1, AT)
            .map_err(LcRelationError::from)
            .and_then(|()| {
                if self.0.family.width().is_zero() {
                    return Ok(None);
                }
                let mathematical = self.0.family.overlap(first, budget)?;
                let bytes = mathematical
                    .retained_payload_bytes()
                    .saturating_add(storage::size(size_of::<LcTimeTail>())?)
                    .saturating_add(self.retained_payload_bytes())
                    .saturating_add(self.relation().retained_payload_bytes());
                storage::retained(bytes, budget)?;
                Ok(Some(mathematical))
            });
        match result {
            Ok(Some(mathematical)) => LcTailOutcome::Certified(Box::new(LcTimeTail {
                cover: self.clone(),
                mathematical,
                existence: None,
            })),
            Ok(None) => LcTailOutcome::SeparatedInfinite(self.clone()),
            Err(reason) => LcTailOutcome::Undecided {
                cover: self.clone(),
                reason,
            },
        }
    }

    /// Check one rate member of the total positive element/amplitude parameterization.
    /// # Errors
    /// Refuses rates outside the retained support and exhausted caller resources.
    pub fn witness(
        &self,
        rate: Rational,
        budget: &mut Budget,
    ) -> Result<LcWitness, LcRelationError> {
        self.0
            .rectangle
            .witness(&self.0.relation, self.0.family.rate(), rate, budget)
    }

    /// Original admission snapshots, excluding later tail and numerical work.
    #[must_use]
    pub fn usage(&self) -> [BudgetUsage; 2] {
        self.0.usage
    }

    /// Conservative new exact ownership, excluding the shared declaration.
    #[must_use]
    pub fn retained_payload_bytes(&self) -> u64 {
        self.0.bytes
    }
}

impl LcTimeTail {
    /// Original complete family; no indices disappear when the suffix is aggregated.
    #[must_use]
    pub const fn cover(&self) -> &LcZeroTimes {
        &self.cover
    }

    /// Exact overlap, minimality, start and last-separate-index evidence.
    #[must_use]
    pub const fn mathematical(&self) -> &PiZeroTail {
        &self.mathematical
    }

    /// Explicit assurance attached to this immutable result.
    #[must_use]
    pub const fn status(&self) -> LcTailStatus {
        if self.existence.is_some() {
            LcTailStatus::FeasibleEverywhere
        } else {
            LcTailStatus::CoverageOnly
        }
    }

    /// Original-equation anchor for the admitted total parameter map, if checked.
    #[must_use]
    pub const fn existence_witness(&self) -> Option<&LcWitness> {
        self.existence.as_ref()
    }

    /// Add total feasibility using the admitted independent rectangle parameter map.
    ///
    /// For every `t` in a positive member, `a=2*pi*n/t` lies in the original rate support.
    /// The checked map `L=C=1/a`, `V=I=sqrt(2*E*a)` realizes every such positive rate.
    /// Continuous positive operations extend this map to all real rates, not only samples.
    /// # Errors
    /// Refuses exact original-equation checking or exhausted resources; the prior tail survives.
    pub fn certify_feasibility(&self, budget: &mut Budget) -> Result<Self, LcRelationError> {
        budget.work(1, AT)?;
        let witness = self
            .cover
            .witness(self.cover.family().rate()[0].clone(), budget)?;
        let bytes = self
            .mathematical
            .retained_payload_bytes()
            .saturating_add(self.cover.retained_payload_bytes())
            .saturating_add(self.cover.relation().retained_payload_bytes())
            .saturating_add(witness.retained_payload_bytes())
            .saturating_add(512);
        storage::retained(bytes, budget)?;
        Ok(Self {
            cover: self.cover.clone(),
            mathematical: self.mathematical.clone(),
            existence: Some(witness),
        })
    }
}
