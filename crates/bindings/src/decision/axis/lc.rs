// SPDX-License-Identifier: AGPL-3.0-only
//! Explicit deterministic L/C declarations retain the written driven expression and original law.

mod original;
use super::{AxisPhaseImpedance, DecisionError as Error, storage};
use crate::readings::{EntryMeaning, LcSpectralLaw};
use crate::{Budget, Descriptor, ExactConstant, Kind, Measure, Role};
use nrz_cas::Rational;
use std::rc::Rc;

/// One explicitly declared deterministic L/C point and original driven impedance expression.
///
/// The point is supplied source data, not inferred from equal floating centers. Numerical
/// clients must bind the original law's L/C readings and cover these exact coordinates with
/// their numerical-only radii; physical L/C uncertainty is outside this closed route.
#[derive(Clone, Debug)]
pub struct LcDrivenImpedance(Rc<Data>);

/// The original schema, point assignment and complete expression remain one immutable owner.
#[derive(Debug)]
struct Data {
    /// Original positive LC model and its ordered formal source identities.
    law: LcSpectralLaw,
    /// Explicit coherent physical L/C assignment, both strictly positive.
    point: [Rational; 2],
    /// Original angular-frequency coordinate and its physical meaning.
    drive: (EntryMeaning, Rational),
    /// Explicit source-free one-radian divisor restores the ordinary rate in the formula.
    radian: EntryMeaning,
    /// Original written sign question and complete fixed impedance/current equation.
    impedance: AxisPhaseImpedance,
    /// Scoped exact metadata allowance, excluding shared model and case graphs.
    bytes: u64,
}

impl LcDrivenImpedance {
    /// Declare a deterministic point and authenticate the complete written driven expression.
    /// # Errors
    /// Refuses nonpositive inputs, wrong frequency/radian meanings, changed original operands,
    /// a substituted fixed output, nonrational references or caller resource exhaustion.
    pub fn new(
        law: LcSpectralLaw,
        point: [Rational; 2],
        drive: (EntryMeaning, Rational),
        radian: EntryMeaning,
        impedance: AxisPhaseImpedance,
        budget: &mut Budget,
    ) -> Result<Self, Error> {
        meanings(&drive, &radian, budget)?;
        for value in point.iter().chain(std::iter::once(&drive.1)) {
            budget.work(1, storage::AT)?;
            if value.is_zero() || value.is_negative() {
                return Err(Error::Source);
            }
        }
        original::check(impedance.case(), &point, &drive.1, budget)?;
        let value = evaluate(&point, &drive.1, budget)?;
        if !impedance
            .case()
            .source()
            .exact()
            .ok_or(Error::Source)?
            .structurally_equal(&ExactConstant::from(value), budget)?
        {
            return Err(Error::Target);
        }
        let mut bytes = u64::try_from(size_of::<Data>() + 2 * size_of::<usize>())
            .map_err(|_| Error::Capacity)?;
        for value in point.iter().chain(std::iter::once(&drive.1)) {
            bytes = bytes
                .checked_add(storage::rational_bytes(value))
                .ok_or(Error::Capacity)?;
        }
        storage::retained(bytes, budget)?;
        Ok(Self(Rc::new(Data {
            law,
            point,
            drive,
            radian,
            impedance,
            bytes,
        })))
    }
    /// Evaluate the retained original point expression using exact arithmetic on the caller ledger.
    /// # Errors
    /// Refuses exhausted exact arithmetic; a rounded cancellation never establishes zero.
    pub fn evaluate(&self, budget: &mut Budget) -> Result<Rational, Error> {
        evaluate(&self.0.point, &self.0.drive.1, budget)
    }
    /// Original LC schema, including the ordered formal source identities and references.
    #[must_use]
    pub fn law(&self) -> &LcSpectralLaw {
        &self.0.law
    }
    /// Explicit exact source assignment in coherent physical L/C coordinates.
    #[must_use]
    pub fn input_coordinates(&self) -> &[Rational; 2] {
        &self.0.point
    }
    /// Original checked driven question and complete impedance/current equation.
    #[must_use]
    pub fn impedance(&self) -> &AxisPhaseImpedance {
        &self.0.impedance
    }
    /// Original exact coherent angular frequency and its retained physical declaration.
    #[must_use]
    pub fn drive(&self) -> &(EntryMeaning, Rational) {
        &self.0.drive
    }
    /// The exact radian divisor is retained instead of casting angular frequency to a rate.
    #[must_use]
    pub fn radian(&self) -> &EntryMeaning {
        &self.0.radian
    }
    /// Scoped new metadata allowance, excluding the original shared graph/proof owners.
    #[must_use]
    pub fn retained_payload_bytes(&self) -> u64 {
        self.0.bytes
    }
}

/// The prescribed original series law keeps both multiplications and the capacitive divisor.
fn evaluate(
    point: &[Rational; 2],
    frequency: &Rational,
    budget: &mut Budget,
) -> Result<Rational, Error> {
    let inductive = frequency.mul(&point[0], budget)?;
    let denominator = frequency.mul(&point[1], budget)?;
    let capacitive = Rational::one().div(&denominator, budget)?;
    Ok(inductive.sub(&capacitive, budget)?)
}

/// Only coherent angular frequency and an explicit coherent unit angle enter this closed relation.
fn meanings(
    drive: &(EntryMeaning, Rational),
    radian: &EntryMeaning,
    budget: &mut Budget,
) -> Result<(), Error> {
    let frequency = Descriptor::new(
        Kind::angular_frequency()?,
        Measure::coherent(),
        Role::Difference,
    )?;
    let angle = Descriptor::new(Kind::Angle, Measure::coherent(), Role::Difference)?;
    budget.work(4, storage::AT)?;
    if drive.0.descriptor() != &frequency
        || radian.descriptor() != &angle
        || drive.0.magnitude().as_rational() != Some(&Rational::one())
        || radian.magnitude().as_rational() != Some(&Rational::one())
    {
        return Err(Error::Association);
    }
    Ok(())
}
