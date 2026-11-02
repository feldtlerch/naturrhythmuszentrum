// SPDX-License-Identifier: AGPL-3.0-only
//! Fixed physical meanings distinguish angles, derivative rates and angular frequencies.

use crate::{Descriptor, Dimension, Kind, Measure, Role, UnitError};

/// A coordinate of the same autonomous LC relation, in coherent SI units.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LcQuantity {
    /// Inductance in henries, strictly positive on both physical branches.
    Inductance,
    /// Capacitance in farads, strictly positive on both physical branches.
    Capacitance,
    /// Initial oscillation angle difference in radians, undefined at zero energy.
    InitialPhase,
    /// Duration in seconds from the relation's common epoch.
    Time(usize),
    /// Observed oscillation angle difference in radians at the indexed time.
    Phase(usize),
    /// Nonnegative voltage amplitude in volts, not an instantaneous signed voltage.
    VoltageAmplitude,
    /// Nonnegative current amplitude in amperes, not a driven source amplitude.
    CurrentAmplitude,
    /// Source-free energy in joules.
    Energy,
    /// Product L C, with duration-squared kind in seconds squared.
    Product,
    /// Positive natural time sqrt(L C) in seconds.
    NaturalTime,
    /// Amplitude ratio V/I in ohms, undefined at zero energy.
    AmplitudeRatio,
    /// Positive angular frequency in radians per second.
    AngularFrequency,
    /// Positive coherent rate a = omega/(1 rad) in inverse seconds.
    Rate,
}

impl LcQuantity {
    /// Construct the exact coherent difference descriptor prescribed for this coordinate.
    /// # Errors
    /// Preserves existing bounded dimension and descriptor construction refusals.
    pub fn descriptor(self) -> Result<Descriptor, UnitError> {
        let kind = match self {
            Self::InitialPhase | Self::Phase(_) => Kind::Angle,
            Self::AngularFrequency => Kind::angular_frequency()?,
            Self::Rate => Kind::rate()?,
            Self::Inductance => named("inductance", [2, 1, -2, -2, 0, 0, 0])?,
            Self::Capacitance => named("capacitance", [-2, -1, 4, 2, 0, 0, 0])?,
            Self::Time(_) | Self::NaturalTime => named("duration", [0, 0, 1, 0, 0, 0, 0])?,
            Self::VoltageAmplitude => named("voltage", [2, 1, -3, -1, 0, 0, 0])?,
            Self::CurrentAmplitude => named("current", [0, 0, 0, 1, 0, 0, 0])?,
            Self::Energy => named("energy", [2, 1, -2, 0, 0, 0, 0])?,
            Self::Product => named("duration_squared", [0, 0, 2, 0, 0, 0, 0])?,
            Self::AmplitudeRatio => named("impedance", [2, 1, -3, -2, 0, 0, 0])?,
        };
        Descriptor::new(kind, Measure::coherent(), Role::Difference)
    }

    /// Only paired coordinates carry an index in the declared observation inventory.
    pub(super) const fn observation(self) -> Option<usize> {
        match self {
            Self::Time(index) | Self::Phase(index) => Some(index),
            Self::Inductance
            | Self::Capacitance
            | Self::InitialPhase
            | Self::VoltageAmplitude
            | Self::CurrentAmplitude
            | Self::Energy
            | Self::Product
            | Self::NaturalTime
            | Self::AmplitudeRatio
            | Self::AngularFrequency
            | Self::Rate => None,
        }
    }
}

/// A named kind retains its semantic distinction even when dimensions coincide.
fn named(name: &str, dimensions: [i64; 7]) -> Result<Kind, UnitError> {
    Ok(Kind::Named(
        name.into(),
        Box::new(Dimension::integers(dimensions)?),
    ))
}
