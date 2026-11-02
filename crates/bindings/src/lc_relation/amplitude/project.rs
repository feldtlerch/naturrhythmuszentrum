// SPDX-License-Identifier: AGPL-3.0-only
//! Complete independent-time phase coverage composes with independent positive scale coordinates.

use super::{
    AT, Cover, LcAmplitudeCover, LcAmplitudeSamples, LcRelationError, Rational, storage, witness,
};
use crate::{Budget, LcQuantity as Q, LcRelation, LcSampleTimes};
use nrz_cas::pi_bounds::PiBoundCertificate;
use std::rc::Rc;

/// All omitted physical inputs are exactly represented in the retained amplitude owner.
pub(super) fn cover(
    amplitude: &LcAmplitudeSamples,
    cycles: Vec<Rational>,
    pi: &PiBoundCertificate,
    budget: &mut Budget,
) -> Result<LcAmplitudeCover, LcRelationError> {
    let before = budget.usage();
    budget.work(1, AT)?;
    storage::retained(
        amplitude
            .relation()
            .retained_payload_bytes()
            .saturating_mul(2),
        budget,
    )?;
    let mut inputs = Vec::new();
    for input in amplitude.relation().inputs() {
        budget.work(1, AT)?;
        let measured_channel = input.quantity() == amplitude.channel();
        let energy = input.quantity() == Q::Energy;
        if !measured_channel && !energy {
            inputs.push(input.clone());
        }
    }
    let relation = LcRelation::new(
        amplitude.relation().epoch().into(),
        amplitude.relation().observations(),
        inputs,
        budget,
    )?;
    let timing = LcSampleTimes::new(
        relation,
        amplitude.timing().nominal_times().to_vec(),
        budget,
    )?;
    let effective = timing.pairs_cover(cycles, pi, budget)?;
    let energy = amplitude
        .energy()
        .map_or_else(Rational::one, |range| range[0].clone());
    let witness = witness::from_point(
        amplitude,
        effective.feasibility_witness(),
        &amplitude.measured()[0],
        &amplitude.gain()[0],
        &energy,
        budget,
    )?;
    let bytes = amplitude
        .retained_payload_bytes()
        .saturating_add(amplitude.timing().retained_payload_bytes())
        .saturating_add(amplitude.relation().retained_payload_bytes())
        .saturating_add(effective.retained_payload_bytes())
        .saturating_add(witness.original().retained_payload_bytes())
        .saturating_add(storage::rational(witness.gain()))
        .saturating_add(2_048);
    storage::retained(bytes, budget)?;
    Ok(LcAmplitudeCover(Rc::new(Cover {
        amplitude: amplitude.clone(),
        effective,
        witness,
        usage: [before, budget.usage()],
        bytes,
    })))
}
