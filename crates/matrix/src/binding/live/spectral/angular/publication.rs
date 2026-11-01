// SPDX-License-Identifier: AGPL-3.0-only
//! Native evaluation and all output parameters precede one atomic reading import.

use super::{
    AT, Error, Evidence, LcAngularFrequencyParameter, LcAngularFrequencyPublication, Origin,
    parameter, plan::Plan,
};
use crate::{
    DependencyPolicy, Extent, Heap, HeapError, LcSpectralEvaluation, MomentAssurance,
    ReadingImport, ReadingRegion, SpectralValue,
};
use nrz_bindings::{
    Budget,
    readings::{LcAngularFrequencyDefinition, algebra::limits::BudgetKind},
};
use nrz_scalar::{ArithmeticContext, Scalar};
use std::rc::Rc;

/// One atomically published angular-frequency value and its connected parameter bundle.
#[derive(Clone, Debug)]
pub struct PublishedLcAngularFrequency<P: Scalar> {
    /// Both declared live value readings; companions belong to the same immutable group.
    readings: ReadingImport<P>,
    /// Heap-owned original physical and numerical provenance shares this same allocation.
    origin: Rc<Origin>,
    /// Complete native proof, joint model and original-center derivative parameters.
    evidence: Rc<Evidence<P>>,
}

impl<P: Scalar + 'static> PublishedLcAngularFrequency<P> {
    /// Original physical recipe and ordered symbols; this historical evidence owns no mutable values.
    #[must_use]
    pub fn definition(&self) -> &LcAngularFrequencyDefinition {
        self.origin.definition()
    }
    /// Angular-frequency then covariance-error source readings, with all three output companions.
    #[must_use]
    pub const fn readings(&self) -> &ReadingImport<P> {
        &self.readings
    }
    /// Existing native evaluation retained without deriving another root theorem.
    #[must_use]
    pub fn evaluation(&self) -> &LcSpectralEvaluation<P> {
        &self.evidence.evaluation
    }
    /// Read the angular-frequency center and separate deterministic/numerical channels.
    /// # Errors
    /// Refuses any stale original source, companion, output or arithmetic revision.
    pub fn value(
        &self,
        heap: &Heap,
        context: &ArithmeticContext,
    ) -> Result<SpectralValue<P>, Error> {
        self.validate(heap, context)?;
        self.evidence
            .evaluation
            .imaginary_rates(heap, context)
            .map(|rates| rates[0])
    }
    /// Exact stored physical coordinates that define this local model's operating point.
    /// # Errors
    /// Refuses every stale dependency of the connected output bundle.
    pub fn operating_point(
        &self,
        heap: &Heap,
        context: &ArithmeticContext,
    ) -> Result<[P; 2], Error> {
        self.validate(heap, context)?;
        Ok(self.evidence.point)
    }
    /// Checked original-center partials in physical L/C order, including their numerical errors.
    /// # Errors
    /// Refuses every stale dependency of the connected output bundle.
    pub fn jacobian(
        &self,
        heap: &Heap,
        context: &ArithmeticContext,
    ) -> Result<[LcAngularFrequencyParameter<P>; 2], Error> {
        self.validate(heap, context)?;
        Ok(self.evidence.jacobian)
    }
    /// Local covariance parameter and its own numerical error in angular-frequency product units.
    /// # Errors
    /// Refuses stale source or output values, either companion side, parameter errors and revisions.
    pub fn covariance(
        &self,
        heap: &Heap,
        context: &ArithmeticContext,
    ) -> Result<LcAngularFrequencyParameter<P>, Error> {
        self.validate(heap, context)?;
        let values = heap.reading_companion_entries(&self.readings.bindings()[0], 2)?;
        let errors = heap.reading_entries(&self.readings.bindings()[1])?;
        let ([center], [error]) = (values.as_slice(), errors.as_slice()) else {
            return Err(Error::Identity);
        };
        Ok(LcAngularFrequencyParameter {
            center: *center,
            error: *error,
        })
    }
    /// Original joint model, ordered source maps and explicit known/zero/unknown block assumptions.
    /// # Errors
    /// Refuses a stale output, original statistical companion or source version.
    pub fn source_model<'h>(
        &self,
        heap: &'h Heap,
        context: &ArithmeticContext,
    ) -> Result<crate::StatisticalView<'h, P>, Error> {
        self.validate(heap, context)?;
        let (companion, _) = self
            .evidence
            .evaluation
            .plan()
            .statistical_selection()
            .ok_or(Error::Identity)?;
        Ok(heap.statistical(companion)?)
    }
    /// Inherited local linear assurance; no nonlinear output mean or distribution is asserted.
    /// # Errors
    /// Refuses every stale dependency before lending statistical assurance.
    pub fn assurance(
        &self,
        heap: &Heap,
        context: &ArithmeticContext,
    ) -> Result<MomentAssurance, Error> {
        self.validate(heap, context)?;
        Ok(MomentAssurance::LinearizedMoments)
    }
    /// One shared retirement boundary covers original inputs, every parameter and both output readings.
    fn validate(&self, heap: &Heap, context: &ArithmeticContext) -> Result<(), Error> {
        if !self.origin.owns(&self.evidence) {
            return Err(Error::Identity);
        }
        self.evidence.evaluation.validate(heap, context)?;
        for reading in self.readings.bindings() {
            heap.validate_reading(reading)?;
        }
        Ok(())
    }
}

impl Heap {
    /// Publish the angular-frequency value, deterministic channels and local covariance together.
    /// # Errors
    /// Refuses stale sources, reused outputs, nonpositive full boxes, incomplete models and limits atomically.
    pub fn publish_lc_angular_frequency<P: Scalar + 'static>(
        &mut self,
        publication: &LcAngularFrequencyPublication<P>,
        context: &ArithmeticContext,
        budget: &mut Budget,
    ) -> Result<PublishedLcAngularFrequency<P>, Error> {
        let plan = &publication.0;
        for group in self.symbols.groups() {
            for existing in group.coverage.readings() {
                for output in plan.coverage.readings() {
                    budget.work(1, AT)?;
                    if existing.reading().family().root() == output.reading().family().root() {
                        return Err(Error::Identity);
                    }
                }
            }
        }
        let owned = produced(plan)?;
        budget.check(BudgetKind::OutputBytes, owned, AT)?;
        budget.with_retained_payload(plan.bytes, AT, |budget| {
            let evaluation = self.evaluate_lc_spectrum(&plan.spectrum, context, budget)?;
            budget.with_retained_payload(owned, AT, |budget| {
                commit(self, plan, evaluation, context, budget)
            })
        })
    }
}

/// All fallible parameter packing and graph preparation precede the existing atomic import.
fn commit<P: Scalar + 'static>(
    heap: &mut Heap,
    plan: &Plan<P>,
    evaluation: LcSpectralEvaluation<P>,
    context: &ArithmeticContext,
    budget: &mut Budget,
) -> Result<PublishedLcAngularFrequency<P>, Error> {
    let (evidence, rate, covariance) = pack(heap, evaluation, context, budget)?;
    let regions = [
        rate.center,
        rate.total,
        rate.error,
        covariance.center,
        covariance.error,
    ]
    .into_iter()
    .map(|value| {
        Ok(ReadingRegion {
            extent: Extent::new(1, 1)?,
            values: vec![value],
        })
    })
    .collect::<Result<Vec<_>, HeapError>>()?;
    let coverage = evidence.evaluation.coverage();
    let evidence = Rc::new(evidence);
    let origin = Rc::new(Origin::new(plan.definition.clone(), &evidence));
    let provenance = crate::binding::live::import::ImportProvenance {
        definition: crate::binding::live::ResultDefinition::AngularFrequency(Rc::clone(&origin)),
        sources: plan.sources.clone(),
        dependencies: DependencyPolicy::WholeBase,
        coverage,
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
    Ok(PublishedLcAngularFrequency {
        readings,
        origin,
        evidence,
    })
}

/// New result, five scalar regions, native statistical extension and all live tickets coexist.
fn produced<P: Scalar>(plan: &Plan<P>) -> Result<u64, Error> {
    let bytes = size_of::<Origin>()
        + size_of::<Evidence<P>>()
        + 4 * size_of::<usize>()
        + size_of::<PublishedLcAngularFrequency<P>>()
        + size_of::<crate::binding::live::Group>()
        + 2 * size_of::<crate::ReadingBinding<P>>()
        + 5 * size_of::<crate::binding::live::RegionTicket>()
        + 5 * (size_of::<P>() + size_of::<u64>() + size_of::<ReadingRegion<P>>());
    plan.bytes
        .checked_add(u64::try_from(bytes).map_err(|_| HeapError::ExtentOverflow)?)
        .and_then(|value| {
            value.checked_add(super::super::statistics::output_bytes(&plan.spectrum).ok()?)
        })
        .ok_or_else(|| HeapError::ExtentOverflow.into())
}

/// Existing angular evidence and its value/covariance parameters are packed without publication.
type Packed<P> = (
    Evidence<P>,
    SpectralValue<P>,
    LcAngularFrequencyParameter<P>,
);

/// Reuse the existing angular packing schedule with an already evaluated native root.
pub(in crate::binding::live::spectral) fn pack<P: Scalar + 'static>(
    heap: &Heap,
    evaluation: LcSpectralEvaluation<P>,
    context: &ArithmeticContext,
    budget: &mut Budget,
) -> Result<Packed<P>, Error> {
    budget.work(9, AT)?;
    let rate = evaluation.imaginary_rates(heap, context)?[0];
    let partials = evaluation
        .root_partials(heap, context)?
        .ok_or(Error::Identity)?[1];
    let covariance = parameter(
        evaluation
            .root_covariance(heap, context)?
            .ok_or(Error::Identity)?[1][1],
        context,
    )?;
    let jacobian = [
        parameter(partials[0], context)?,
        parameter(partials[1], context)?,
    ];
    let point = evaluation
        .plan()
        .statistical_operating_point(heap, context, budget)?;
    let point = <[P; 2]>::try_from(point).map_err(|_| Error::Identity)?;
    Ok((
        Evidence {
            evaluation,
            point,
            jacobian,
        },
        rate,
        covariance,
    ))
}
