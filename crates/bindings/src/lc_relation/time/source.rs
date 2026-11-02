// SPDX-License-Identifier: AGPL-3.0-only
//! Original total feasibility maps remain attached to every zero-time projection.

use super::{LcRelation, LcRelationError, LcWitness, rectangle::Rectangle};
use crate::{Budget, LcPairParameters, LcPairsCover};
use nrz_cas::Rational;

/// Each source proves every rate in the retained support has exact starting phase zero.
#[derive(Debug)]
pub(super) enum Source {
    /// Independent original rate/energy supports.
    Rectangle(Rectangle),
    /// Complete known-zero-phase pair family, retaining scales and every observation.
    Pairs(LcPairsCover),
}

impl Source {
    /// Reuse the original source's full-equation parameter map at any admissible rate.
    pub(super) fn witness(
        &self,
        relation: &LcRelation,
        support: &[Rational; 2],
        rate: Rational,
        budget: &mut Budget,
    ) -> Result<LcWitness, LcRelationError> {
        match self {
            Self::Rectangle(rectangle) => rectangle.witness(relation, support, rate, budget),
            Self::Pairs(cover) => cover.witness(
                &LcPairParameters {
                    rate,
                    initial_phase: Rational::zero(),
                    ratio: Rational::one(),
                    voltage: Rational::one(),
                    gauge: Rational::zero(),
                },
                budget,
            ),
        }
    }

    /// Conservative shared source ownership; original relation allocation is counted separately.
    pub(super) fn bytes(&self) -> u64 {
        match self {
            Self::Rectangle(rectangle) => rectangle.bytes(),
            Self::Pairs(cover) => cover.retained_payload_bytes(),
        }
    }
}
