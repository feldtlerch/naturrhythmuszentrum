// SPDX-License-Identifier: AGPL-3.0-only
//! Original typed causes remain inspectable; common resource failures block later strata.

use super::LcDrivenStatus;
use crate::{HeapError, LiveDecisionError, PhasorError, SolveError, SpectralError};
use nrz_bindings::readings::algebra::{Fault, FaultKind};
use std::error::Error;

/// A continuation admission or execution refusal, preserving its original child cause.
#[derive(Debug, thiserror::Error)]
pub enum LcDrivenError {
    /// Original live sign or phase consumer refused.
    #[error("LC driven decision: {0}")]
    Decision(#[source] Box<LiveDecisionError>),
    /// Original physical current publication refused.
    #[error("LC driven current: {0}")]
    Current(#[source] Box<PhasorError>),
    /// The declared shared-voltage statistical family refused.
    #[error("LC driven statistics: {0}")]
    Statistics(#[source] Box<crate::CompanionExecutionError>),
    /// The retained autonomous owner refused.
    #[error("LC driven invariant: {0}")]
    Invariant(#[from] SpectralError),
    /// New table storage or an original heap lease refused.
    #[error("LC driven storage: {0}")]
    Heap(#[from] HeapError),
    /// The common cumulative caller allowance refused.
    #[error("LC driven resources: {0}")]
    Resources(#[from] Fault),
    /// Point order, owner, or the declared stratum differs.
    #[error("LC driven schedule identity mismatch")]
    Identity,
}

impl From<crate::CompanionExecutionError> for LcDrivenError {
    fn from(cause: crate::CompanionExecutionError) -> Self {
        Self::Statistics(Box::new(cause))
    }
}
impl From<PhasorError> for LcDrivenError {
    fn from(cause: PhasorError) -> Self {
        Self::Current(Box::new(cause))
    }
}
impl From<LiveDecisionError> for LcDrivenError {
    fn from(cause: LiveDecisionError) -> Self {
        Self::Decision(Box::new(cause))
    }
}

impl LcDrivenError {
    /// Walk the closed typed cause chain, preserving resource versus numerical refusals.
    pub(super) fn status(&self) -> LcDrivenStatus {
        let mut cause: Option<&(dyn Error + 'static)> = Some(self);
        let mut numerical = false;
        while let Some(error) = cause {
            let exhausted = error
                .downcast_ref::<Fault>()
                .is_some_and(|fault| matches!(fault.kind, FaultKind::Exhausted { .. }));
            let capacity = error.downcast_ref::<HeapError>().is_some_and(|fault| {
                matches!(
                    fault,
                    HeapError::Capacity | HeapError::ExtentOverflow | HeapError::WorkLimit
                )
            });
            let solve = error.downcast_ref::<SolveError>();
            if exhausted || capacity || matches!(solve, Some(SolveError::ResourceLimit)) {
                return LcDrivenStatus::ResourceRefused;
            }
            numerical |= error.is::<nrz_scalar::ArithmeticFailure>() || solve.is_some();
            cause = error.source();
        }
        if numerical {
            LcDrivenStatus::NumericallyFailed
        } else {
            LcDrivenStatus::BindingRefused
        }
    }
}
