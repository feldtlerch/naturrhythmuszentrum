// SPDX-License-Identifier: AGPL-3.0-only
//! One infallible publication suffix exposes both frequency channels and their cross covariance.

use super::{AT, Error, Evidence, LcFrequencyPublication, Origin, plan::Plan, values};
use crate::{
    DependencyPolicy, Extent, Heap, HeapError, LcAngularFrequencyParameter, LcSpectralEvaluation,
    MomentAssurance, ReadingImport, ReadingRegion, SpectralValue,
};
use nrz_bindings::{
    Budget,
    readings::{LcFrequencyDefinition, algebra::limits::BudgetKind},
};
use nrz_scalar::{ArithmeticContext, Scalar};
use std::rc::Rc;

/// Both frequencies, deterministic channels and every local covariance parameter publish together.
#[derive(Clone, Debug)]
pub struct PublishedLcFrequency<P: Scalar> {
    /// Five value/error readings share all seven companions and one retirement boundary.
    readings: ReadingImport<P>,
    /// The heap group retains this exact and numerical origin independently of the wrapper.
    origin: Rc<Origin>,
    /// Original root evidence and source-free full-cycle transport share one immutable owner.
    evidence: Rc<Evidence<P>>,
}

impl<P: Scalar + 'static> PublishedLcFrequency<P> {
    /// Historical exact definition and complete ordered output inventory.
    #[must_use]
    pub fn definition(&self) -> &LcFrequencyDefinition {
        self.origin.definition()
    }

    /// Angular value/error, cyclic value/error and cross-error readings, in that order.
    #[must_use]
    pub const fn readings(&self) -> &ReadingImport<P> {
        &self.readings
    }

    /// Both finite values and their total/numerical channels, angular first.
    /// # Errors
    /// Refuses any stale original source, companion, output or arithmetic revision.
    pub fn values(
        &self,
        heap: &Heap,
        context: &ArithmeticContext,
    ) -> Result<[SpectralValue<P>; 2], Error> {
        self.validate(heap, context)?;
        Ok(self.evidence.values)
    }

    /// Original native evaluation; this accessor validates the entire connected publication.
    /// # Errors
    /// Refuses any stale original or published channel.
    pub fn evaluation(
        &self,
        heap: &Heap,
        context: &ArithmeticContext,
    ) -> Result<&LcSpectralEvaluation<P>, Error> {
        self.validate(heap, context)?;
        Ok(&self.evidence.angular.evaluation)
    }

    /// Original represented L/C center, with no rounded-intermediate substitution.
    /// # Errors
    /// Refuses every stale dependency of the connected bundle.
    pub fn operating_point(
        &self,
        heap: &Heap,
        context: &ArithmeticContext,
    ) -> Result<[P; 2], Error> {
        self.validate(heap, context)?;
        Ok(self.evidence.angular.point)
    }

    /// Angular then cyclic Jacobian rows, in physical L/C column order with numerical parameter errors.
    /// # Errors
    /// Refuses every stale dependency of the connected bundle.
    pub fn jacobians(
        &self,
        heap: &Heap,
        context: &ArithmeticContext,
    ) -> Result<[[LcAngularFrequencyParameter<P>; 2]; 2], Error> {
        self.validate(heap, context)?;
        Ok([self.evidence.angular.jacobian, self.evidence.jacobian])
    }

    /// Full symmetric local covariance, ordered angular then cyclic on both axes.
    /// # Errors
    /// Refuses every stale source, target, covariance or covariance-error version.
    pub fn covariance(
        &self,
        heap: &Heap,
        context: &ArithmeticContext,
    ) -> Result<[[LcAngularFrequencyParameter<P>; 2]; 2], Error> {
        self.validate(heap, context)?;
        let [angular, cyclic, cross] = self.evidence.covariance;
        Ok([[angular, cross], [cross, cyclic]])
    }

    /// Original immutable joint model, source identities and declared block assumptions.
    /// # Errors
    /// Refuses every stale dependency before lending statistical evidence.
    pub fn source_model<'h>(
        &self,
        heap: &'h Heap,
        context: &ArithmeticContext,
    ) -> Result<crate::StatisticalView<'h, P>, Error> {
        self.validate(heap, context)?;
        let (companion, _) = self
            .evidence
            .angular
            .evaluation
            .plan()
            .statistical_selection()
            .ok_or(Error::Identity)?;
        Ok(heap.statistical(companion)?)
    }

    /// Inherited local assurance creates no nonlinear mean or distributional claim.
    /// # Errors
    /// Refuses every stale dependency before lending assurance.
    pub fn assurance(
        &self,
        heap: &Heap,
        context: &ArithmeticContext,
    ) -> Result<MomentAssurance, Error> {
        self.validate(heap, context)?;
        Ok(MomentAssurance::LinearizedMoments)
    }

    /// Source-free numerical coefficient and its independent evaluation error.
    /// # Errors
    /// Refuses every stale dependency or incompatible arithmetic revision.
    pub fn coefficient(
        &self,
        heap: &Heap,
        context: &ArithmeticContext,
    ) -> Result<crate::ConstantCoefficient<P>, Error> {
        self.validate(heap, context)?;
        Ok(self.evidence.map.scale)
    }

    /// Original shared pi owner and arithmetic proof retained by this connected publication.
    /// # Errors
    /// Refuses every stale dependency before lending numerical provenance.
    pub fn constant(
        &self,
        heap: &Heap,
        context: &ArithmeticContext,
    ) -> Result<crate::PiConstantEvidence, Error> {
        self.validate(heap, context)?;
        Ok(self.evidence.pi.evidence())
    }

    /// Every accessor shares the original proof and the complete new group closure.
    fn validate(&self, heap: &Heap, context: &ArithmeticContext) -> Result<(), Error> {
        if !self.origin.owns(&self.evidence)
            || self.evidence.map.revision != context.revision()
            || self.evidence.pi.arithmetic_revision() != context.revision()
        {
            return Err(Error::Identity);
        }
        self.evidence.angular.evaluation.validate(heap, context)?;
        for reading in self.readings.bindings() {
            heap.validate_reading(reading)?;
        }
        Ok(())
    }
}

impl Heap {
    /// Publish both native frequencies, all deterministic companions and the full local covariance.
    /// # Errors
    /// Refuses stale evidence, reused outputs, arithmetic failure or any limit without a published prefix.
    pub fn publish_lc_frequency<P: Scalar + 'static>(
        &mut self,
        publication: &LcFrequencyPublication<P>,
        context: &ArithmeticContext,
        budget: &mut Budget,
    ) -> Result<PublishedLcFrequency<P>, Error> {
        let plan = &publication.0;
        fresh(self, plan, budget)?;
        let owned = produced(plan)?;
        budget.check(BudgetKind::OutputBytes, owned, AT)?;
        budget.with_retained_payload(plan.bytes, AT, |budget| {
            budget.with_retained_payload(owned, AT, |budget| commit(self, plan, context, budget))
        })
    }
}

/// Every initialized value and companion source declaration must be fresh in this heap.
fn fresh<P: Scalar>(heap: &Heap, plan: &Plan<P>, budget: &mut Budget) -> Result<(), Error> {
    for group in heap.symbols.groups() {
        let existing = group
            .coverage
            .readings()
            .iter()
            .map(|reading| reading.reading().family().root())
            .chain(
                group
                    .companions
                    .iter()
                    .map(|companion| companion.companion().parameters().node()),
            );
        for previous in existing {
            let outputs = plan
                .coverage
                .readings()
                .iter()
                .map(|reading| reading.reading().family().root())
                .chain(
                    plan.companions
                        .iter()
                        .map(|companion| companion.companion().parameters().node()),
                );
            for output in outputs {
                budget.work(1, AT)?;
                if previous == output {
                    return Err(Error::Identity);
                }
            }
        }
    }
    Ok(())
}

/// All fallible numerical and exact preparation precedes the existing atomic import suffix.
fn commit<P: Scalar + 'static>(
    heap: &mut Heap,
    plan: &Plan<P>,
    context: &ArithmeticContext,
    budget: &mut Budget,
) -> Result<PublishedLcFrequency<P>, Error> {
    let evidence = Rc::new(values::prepare(heap, plan, context, budget)?);
    let regions = values::entries(&evidence)
        .into_iter()
        .map(|value| {
            Ok(ReadingRegion {
                extent: Extent::new(1, 1)?,
                values: vec![value],
            })
        })
        .collect::<Result<Vec<_>, HeapError>>()?;
    let origin = Rc::new(Origin::new(plan.definition.clone(), &evidence));
    let provenance = crate::binding::live::import::ImportProvenance {
        definition: crate::binding::live::ResultDefinition::Frequency(Rc::clone(&origin)),
        sources: plan.sources.clone(),
        dependencies: DependencyPolicy::WholeBase,
        coverage: plan.evaluation.coverage(),
    };
    let readings = heap.import_readings_with_provenance(
        Rc::clone(&plan.graph),
        plan.coverage.clone(),
        plan.companions.clone(),
        &regions,
        DependencyPolicy::WholeBase,
        Some(provenance),
        budget,
    )?;
    Ok(PublishedLcFrequency {
        readings,
        origin,
        evidence,
    })
}

/// The original evaluation and constant remain shared; count all newly produced storage and owners.
fn produced<P: Scalar>(plan: &Plan<P>) -> Result<u64, Error> {
    let bytes = size_of::<Origin>()
        + size_of::<Evidence<P>>()
        + 4 * size_of::<usize>()
        + size_of::<PublishedLcFrequency<P>>()
        + size_of::<crate::binding::live::Group>()
        + 5 * size_of::<crate::ReadingBinding<P>>()
        + 12 * size_of::<crate::binding::live::RegionTicket>()
        + 12 * (size_of::<P>() + size_of::<u64>() + size_of::<ReadingRegion<P>>());
    plan.bytes
        .checked_add(u64::try_from(bytes).map_err(|_| HeapError::ExtentOverflow)?)
        .ok_or_else(|| HeapError::ExtentOverflow.into())
}
