// SPDX-License-Identifier: AGPL-3.0-only
//! A retained root is usable only under its original live plan and source binding.

use super::{
    AT, Error, Heap, LcSourceBinding, LcSourceStatistics, LcSpectralEvaluation, LcSpectrum,
};
use nrz_bindings::Budget;
use nrz_scalar::{ArithmeticContext, Scalar};
use std::rc::Rc;

impl<P: Scalar> LcSourceBinding<P> {
    /// Original bound spectral plan, available for one separately scheduled evaluation.
    ///
    /// Inspection grants no current numerical authority; evaluation validates its live inputs.
    #[must_use]
    pub fn spectrum(&self) -> &LcSpectrum<P> {
        &self.0.spectrum
    }
}

impl Heap {
    /// Compose source statistics using the original retained root without another root evaluation.
    ///
    /// The complete original plan and live source closure must match. The retained
    /// numerical-only root enclosure supplies the implicit derivative denominator;
    /// insufficient separation refuses without a fresh-evaluation fallback.
    /// # Errors
    /// Refuses foreign or independently prepared plans, stale original values/companions,
    /// incomplete covariance, nonpositive root bounds and cumulative resource exhaustion.
    pub fn evaluate_lc_source_statistics_reusing<P: Scalar + 'static>(
        &self,
        binding: &LcSourceBinding<P>,
        spectrum: &LcSpectralEvaluation<P>,
        context: &ArithmeticContext,
        budget: &mut Budget,
    ) -> Result<LcSourceStatistics<P>, Error> {
        let before = budget.usage();
        binding.validate(self)?;
        spectrum.validate(self, context)?;
        budget.work(1, AT)?;
        if !Rc::ptr_eq(&binding.0.spectrum.0, &spectrum.plan().0) {
            return Err(Error::Identity);
        }
        let retained = super::super::statistics::evaluation_bytes(spectrum.plan())?;
        let payload = budget.with_retained_payload(retained, AT, |budget| {
            budget.with_retained_payload(binding.0.bytes, AT, |budget| {
                super::numeric::reuse(self, &binding.0, spectrum, context, budget)
            })
        })?;
        Ok(LcSourceStatistics {
            binding: binding.clone(),
            spectrum: spectrum.clone(),
            payload: Rc::new(payload),
            revision: context.revision(),
            usage: [before, budget.usage()],
        })
    }
}
