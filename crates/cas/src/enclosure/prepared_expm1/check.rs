// SPDX-License-Identifier: AGPL-3.0-only
//! Independent exact inequalities; no call into the proposal generator.

use super::{
    AT, Coefficient, Counts, Policy, Prepared, Proof, Request, Stage, StageKind, count, invalid,
    math,
};
use crate::budget::Budget;
use crate::{Fault, InvalidCode, Rational};
use std::cmp::Ordering;

/// Independently authenticate the original claim and every finite proof obligation.
pub(super) fn verify(
    request: &Request,
    claim: &super::service::Claim,
    policy: Policy,
    proof: &Proof,
    budget: &mut Budget,
    counts: &mut Counts,
) -> Result<Prepared, Fault> {
    count(&mut counts.checks, "independent check")?;
    super::service::parameters(proof.degree, proof.coefficient_bits, policy, budget)?;
    math::capacity(proof.degree, budget)?;
    // Bound direct-API record inventories before the footprint traversal, not only wire inputs.
    if proof.coefficients.len() != proof.degree as usize {
        return Err(invalid(InvalidCode::Expm1Coefficient));
    }
    if proof.stages.len() != (2 * proof.degree - 1) as usize {
        return Err(invalid(InvalidCode::Expm1Rounding));
    }
    let bytes =
        super::proof_bytes(proof).saturating_add(super::super::request_bytes(&request.original));
    let copies = bytes.saturating_add(super::workspace_bytes(proof.degree, budget));
    budget.with_retained_payload(copies, AT, |budget| {
        budget.work(bytes, AT)?;
        if proof.version != 1 {
            return Err(invalid(InvalidCode::UnsupportedVersion));
        }
        if proof.scope != claim.scope {
            return Err(invalid(InvalidCode::Expm1Claim));
        }
        let ideals = coefficients(proof, budget)?;
        let (magnitude, error) = stages(proof, &ideals, budget)?;
        analytic(proof, budget)?;
        export(proof, &magnitude, &error, claim.zero, budget)?;
        if !claim.zero
            && proof
                .endpoint_ratio
                .compare(&math::two_power(-i64::from(request.bits), budget)?, budget)?
                == Ordering::Greater
        {
            return Err(invalid(InvalidCode::Expm1Target));
        }
        let statement = super::wire::statement(proof, budget)?;
        Ok(Prepared {
            request: request.clone(),
            proof: proof.clone(),
            statement,
        })
    })
}

/// Reconstruct factorials multiplicatively, then check nearest-neighbor midpoints.
fn coefficients(proof: &Proof, budget: &mut Budget) -> Result<Vec<Rational>, Fault> {
    budget.work(u64::from(proof.degree), AT)?;
    let mut factorial = Rational::one();
    let mut ideals = Vec::new();
    ideals
        .try_reserve_exact(proof.degree as usize)
        .map_err(|_| Fault::allocation(AT))?;
    for (index, coefficient) in proof.coefficients.iter().enumerate() {
        budget.work(1, AT)?;
        factorial = factorial.mul(
            &Rational::from_ratio(
                i64::try_from(index + 1).map_err(|_| invalid(InvalidCode::Expm1Coefficient))?,
                1,
                budget,
            )?,
            budget,
        )?;
        let ideal = Rational::one().div(&factorial, budget)?;
        nearest(coefficient, proof.coefficient_bits, &ideal, budget)?;
        ideals.push(ideal);
    }
    Ok(ideals)
}

/// Adjacent p-bit dyadics give a checker independent of producer binade/floor arithmetic.
fn nearest(c: &Coefficient, bits: u32, ideal: &Rational, budget: &mut Budget) -> Result<(), Fault> {
    let minimum = 1_u64 << (bits - 1);
    let maximum = 1_u64 << bits;
    if c.significand < minimum || c.significand >= maximum || !(-1022..=0).contains(&c.exponent) {
        return Err(invalid(InvalidCode::Expm1Coefficient));
    }
    let value = c.exact(budget)?;
    let step = math::two_power(i64::from(c.exponent), budget)?;
    let down = if c.significand == minimum {
        step.div(&Rational::from_ratio(2, 1, budget)?, budget)?
    } else {
        step.clone()
    };
    let low_midpoint = value.sub(
        &down.div(&Rational::from_ratio(2, 1, budget)?, budget)?,
        budget,
    )?;
    let high_midpoint = value.add(
        &step.div(&Rational::from_ratio(2, 1, budget)?, budget)?,
        budget,
    )?;
    let low = ideal.compare(&low_midpoint, budget)?;
    let high = ideal.compare(&high_midpoint, budget)?;
    let outside = low == Ordering::Less || high == Ordering::Greater;
    let odd_tie = (low == Ordering::Equal || high == Ordering::Equal) && c.significand % 2 != 0;
    if outside || odd_tie {
        return Err(invalid(InvalidCode::Expm1Coefficient));
    }
    Ok(())
}

/// Bound each separate rounded operation with u*|exact result| + eta, including underflow.
fn stages(
    proof: &Proof,
    ideals: &[Rational],
    budget: &mut Budget,
) -> Result<(Rational, Rational), Fault> {
    let last = ideals.len() - 1;
    let u = math::two_power(-53, budget)?;
    let eta = math::two_power(-1075, budget)?;
    let two = Rational::from_ratio(2, 1, budget)?;
    let mut magnitude = ideals[last].clone();
    let mut error = math::abs(
        &proof.coefficients[last]
            .exact(budget)?
            .sub(&magnitude, budget)?,
        budget,
    )?;
    let mut cursor = 0;
    for index in (0..last).rev() {
        let exact_product = magnitude.mul(&two, budget)?;
        let input_error = error.mul(&two, budget)?;
        let minimum = exact_product
            .add(&input_error, budget)?
            .mul(&u, budget)?
            .add(&eta, budget)?
            .add(&input_error, budget)?;
        stage(
            &proof.stages[cursor],
            StageKind::Multiply,
            &exact_product,
            &minimum,
            budget,
        )?;
        let product_error = &proof.stages[cursor].error;
        cursor += 1;
        let imported = proof.coefficients[index].exact(budget)?;
        let discrepancy = math::abs(&imported.sub(&ideals[index], budget)?, budget)?;
        let rounding = exact_product
            .add(product_error, budget)?
            .add(&imported, budget)?
            .mul(&u, budget)?
            .add(&eta, budget)?;
        let minimum = rounding
            .add(product_error, budget)?
            .add(&discrepancy, budget)?;
        magnitude = exact_product.add(&ideals[index], budget)?;
        stage(
            &proof.stages[cursor],
            StageKind::Add,
            &magnitude,
            &minimum,
            budget,
        )?;
        error = proof.stages[cursor].error.clone();
        cursor += 1;
    }
    let final_magnitude = magnitude.mul(&two, budget)?;
    let propagated = error.mul(&two, budget)?;
    let minimum = final_magnitude
        .add(&propagated, budget)?
        .mul(&u, budget)?
        .add(&eta, budget)?
        .add(&propagated, budget)?;
    stage(
        &proof.stages[cursor],
        StageKind::FinalMultiply,
        &final_magnitude,
        &minimum,
        budget,
    )?;
    Ok((magnitude, error))
}

/// All stage inputs and outputs stay far from overflow; negative error bounds cannot pass.
fn stage(
    record: &Stage,
    kind: StageKind,
    magnitude: &Rational,
    error: &Rational,
    budget: &mut Budget,
) -> Result<(), Fault> {
    budget.work(1, AT)?;
    let shape = record.kind == kind && &record.magnitude == magnitude;
    let error_bound = record.error.compare(error, budget)? != Ordering::Less;
    let finite = record
        .magnitude
        .add(&record.error, budget)?
        .compare(&Rational::from_ratio(16, 1, budget)?, budget)?
        == Ordering::Less;
    if !(shape && error_bound && finite) {
        return Err(invalid(InvalidCode::Expm1Rounding));
    }
    Ok(())
}

/// The analytic schema supplies convergence and a/3; finite factorial identities are checked here.
fn analytic(proof: &Proof, budget: &mut Budget) -> Result<(), Fault> {
    let mut factorial = Rational::one();
    for j in 1..=proof.degree + 1 {
        factorial = factorial.mul(&Rational::from_ratio(i64::from(j), 1, budget)?, budget)?;
    }
    let tail = math::two_power(i64::from(proof.degree), budget)?.div(&factorial, budget)?;
    let absolute = tail.mul(&Rational::from_ratio(2, 1, budget)?, budget)?;
    let relative = tail.mul(&Rational::from_ratio(3, 1, budget)?, budget)?;
    let ratio = Rational::from_ratio(2, u64::from(proof.degree) + 2, budget)?;
    if absolute != proof.absolute_tail
        || relative != proof.relative_tail
        || ratio.compare(&Rational::one(), budget)? != Ordering::Less
    {
        return Err(invalid(InvalidCode::Expm1Analytic));
    }
    Ok(())
}

/// Compose nearest import, nominal error, outward radius, and both explicit endpoint roundings.
fn export(
    proof: &Proof,
    magnitude: &Rational,
    error: &Rational,
    zero: bool,
    budget: &mut Budget,
) -> Result<(), Fault> {
    let one = Rational::one();
    let u = math::two_power(-53, budget)?;
    let two_u = u.mul(&Rational::from_ratio(2, 1, budget)?, budget)?;
    let three = Rational::from_ratio(3, 1, budget)?;
    if proof.import_ratio != two_u {
        return Err(invalid(InvalidCode::Expm1Import));
    }
    if !proof.restoration_error.is_zero() || proof.radius_bits > 1074 {
        return Err(invalid(InvalidCode::Expm1Export));
    }
    let alpha = one.add(&two_u, budget)?;
    let tail = proof
        .absolute_tail
        .div(&Rational::from_ratio(2, 1, budget)?, budget)?;
    let scaled_error = magnitude
        .add(error, budget)?
        .mul(&u, budget)?
        .add(error, budget)?
        .add(&tail, budget)?;
    let central = scaled_error
        .mul(&alpha, budget)?
        .add(&three.mul(&u, budget)?, budget)?;
    if proof.central_ratio.compare(&central, budget)? == Ordering::Less {
        return Err(invalid(InvalidCode::Expm1Rounding));
    }
    let q = math::two_power(-i64::from(proof.radius_bits), budget)?;
    let covered = q
        .mul(&one.sub(&two_u, budget)?, budget)?
        .compare(&proof.central_ratio, budget)?
        != Ordering::Less;
    if !zero && !covered {
        return Err(invalid(InvalidCode::Expm1Export));
    }
    let factor = alpha.mul(&one.add(&u, budget)?, budget)?;
    let additive = three.add(&two_u, budget)?;
    let radius = q
        .mul(&alpha, budget)?
        .mul(&factor, budget)?
        .add(&additive.mul(&u, budget)?, budget)?;
    let deviation = proof.central_ratio.add(&radius, budget)?;
    let rounding = one
        .add(&deviation, budget)?
        .mul(&factor.sub(&one, budget)?, budget)?
        .add(&additive.mul(&u, budget)?, budget)?;
    let endpoint = deviation.add(&rounding, budget)?.mul(&three, budget)?;
    if proof.endpoint_ratio.compare(&endpoint, budget)? == Ordering::Less {
        return Err(invalid(InvalidCode::Expm1Export));
    }
    Ok(())
}
