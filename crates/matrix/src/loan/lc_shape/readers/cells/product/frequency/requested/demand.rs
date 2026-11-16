// SPDX-License-Identifier: AGPL-3.0-only
//! Closed nonempty requested subsets select the native dependency path before execution.

/// Requested local quantities; cyclic frequency and angular frequency remain distinct.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LcQuantityDemand {
    /// Angular frequency only; no pi prelude is used.
    Omega,
    /// Cyclic frequency only.
    Frequency,
    /// Period only; no reciprocal is evaluated.
    Period,
    /// Angular and cyclic frequency share one reciprocal.
    OmegaFrequency,
    /// Angular frequency and period share one root.
    OmegaPeriod,
    /// Cyclic frequency and period share one root and prelude.
    FrequencyPeriod,
    /// All three quantities share their dependency path.
    All,
}
impl LcQuantityDemand {
    /// Requested omega, cyclic frequency and period flags in that fixed order.
    #[must_use]
    pub const fn selection(self) -> [bool; 3] {
        match self {
            Self::Omega => [true, false, false],
            Self::Frequency => [false, true, false],
            Self::Period => [false, false, true],
            Self::OmegaFrequency => [true, true, false],
            Self::OmegaPeriod => [true, false, true],
            Self::FrequencyPeriod => [false, true, true],
            Self::All => [true; 3],
        }
    }
    /// Fixed trace vocabulary carries demand without readings or allocations.
    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::Omega => "omega",
            Self::Frequency => "frequency",
            Self::Period => "period",
            Self::OmegaFrequency => "omega-frequency",
            Self::OmegaPeriod => "omega-period",
            Self::FrequencyPeriod => "frequency-period",
            Self::All => "omega-frequency-period",
        }
    }
    /// Beyond observation: product, three signs, root, root sign, then demanded
    /// reciprocal, prelude compatibility and final scale kernels.
    pub(super) const fn extra_work(self) -> usize {
        let [omega, frequency, period] = self.selection();
        6 + (omega || frequency) as usize
            + (frequency || period) as usize
            + frequency as usize
            + period as usize
    }
}
