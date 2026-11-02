// SPDX-License-Identifier: AGPL-3.0-only
//! Exact original expression evaluation precedes one complete impedance publication.

use super::{Error, LcImpedancePublication, Plan, PublishedLcImpedance};
use crate::{
    Coverage, DependencyPolicy, Extent, Heap, LcSpectralEvaluation, ReadingRegion, SpectralError,
};
use nrz_bindings::{
    Budget,
    readings::algebra::{Rational, limits::BudgetKind},
};
use nrz_scalar::{ArithmeticContext, DecodedLiteral, Enclosure, Finite, Scalar};
use std::rc::Rc;

impl Heap {
    /// Produce the original deterministic driven impedance after one retained invariant result.
    ///
    /// This explicitly declared exact point must be covered by each actual L/C numerical-error
    /// radius. Physical L/C uncertainty refuses. Exact arithmetic on the original expression
    /// establishes resonance; a floating cancellation is never promoted to zero authority.
    /// # Errors
    /// Refuses another invariant owner, stale or uncovered inputs, uncertain L/C, occupied
    /// output roots, failed outward conversion and cumulative resources without a prefix.
    pub fn publish_lc_impedance_reusing<P: Scalar + 'static>(
        &mut self,
        publication: &LcImpedancePublication<P>,
        invariant: &LcSpectralEvaluation<P>,
        context: &ArithmeticContext,
        budget: &mut Budget,
    ) -> Result<PublishedLcImpedance<P>, Error> {
        let plan = &publication.0;
        let before = budget.usage();
        budget.work(1, super::super::AT)?;
        if !Rc::ptr_eq(&plan.spectrum.0, &invariant.plan().0) {
            return Err(Error::Identity);
        }
        invariant.validate(self, context)?;
        fresh(self, plan, budget)?;
        let readings = budget.with_retained_payload(plan.bytes, super::super::AT, |budget| {
            inputs(self, plan, context, budget)?;
            let exact = plan.definition.evaluate(budget).map_err(super::exact)?;
            let value = decode::<P>(&exact, plan, context, budget)?;
            let center = value.center();
            let radius = value
                .enclosure()
                .displacement_from(center, context)
                .map_err(crate::PhasorError::from)?
                .get();
            let regions = [
                ReadingRegion {
                    extent: Extent::new(2, 2)?,
                    values: vec![P::ZERO, center.get(), -center.get(), P::ZERO],
                },
                ReadingRegion {
                    extent: Extent::new(4, 1)?,
                    values: vec![P::ZERO, radius, radius, P::ZERO],
                },
                ReadingRegion {
                    extent: Extent::new(4, 1)?,
                    values: vec![P::ZERO, radius, radius, P::ZERO],
                },
            ];
            let sources = super::super::publication::sources(plan.spectrum.inputs(), budget)?;
            budget.check(BudgetKind::OutputBytes, plan.bytes, super::super::AT)?;
            let provenance = crate::binding::live::import::ImportProvenance {
                definition: crate::binding::live::ResultDefinition::LcImpedance(
                    plan.definition.clone(),
                ),
                sources,
                dependencies: DependencyPolicy::WholeBase,
                coverage: [Coverage::DeclaredBounds; 2],
            };
            invariant.validate(self, context)?;
            self.import_readings_with_provenance(
                Rc::clone(&plan.graph),
                plan.coverage.clone(),
                plan.companions.clone(),
                &regions,
                DependencyPolicy::WholeBase,
                Some(provenance),
                budget,
            )
            .map_err(SpectralError::from)
            .map_err(Error::from)
        })?;
        Ok(PublishedLcImpedance {
            readings,
            definition: plan.definition.clone(),
            usage: [before, budget.usage()],
        })
    }
}

/// A previously bound root cannot be overwritten, including after its old numerical evidence retires.
fn fresh<P: Scalar>(heap: &Heap, plan: &Plan<P>, budget: &mut Budget) -> Result<(), Error> {
    let root = plan
        .definition
        .impedance()
        .equation()
        .impedance()
        .reading()
        .family()
        .root();
    for group in heap.symbols.groups() {
        for reading in group.coverage.readings() {
            budget.work(1, super::super::AT)?;
            if reading.reading().family().root() == root {
                return Err(Error::Identity);
            }
        }
    }
    Ok(())
}

/// The exact source declaration is a qualified input, never inferred from approximate centers.
fn inputs<P: Scalar + 'static>(
    heap: &Heap,
    plan: &Plan<P>,
    context: &ArithmeticContext,
    budget: &mut Budget,
) -> Result<(), Error> {
    let (values, coverage) = super::super::input::capture(heap, &plan.spectrum, budget)?;
    if coverage != [Coverage::DeclaredBounds; 2] {
        return Err(Error::Identity);
    }
    for (value, exact) in values.iter().zip(plan.definition.input_coordinates()) {
        if value.total != value.error {
            return Err(Error::Identity);
        }
        let expected = decode::<P>(exact, plan, context, budget)?.enclosure();
        let actual =
            Enclosure::singleton(Finite::new(value.center).map_err(crate::PhasorError::from)?)
                .add(
                    Enclosure::new(-value.error, value.error).map_err(crate::PhasorError::from)?,
                    context,
                )
                .map_err(crate::PhasorError::from)?;
        if actual.lower() > expected.lower() || actual.upper() < expected.upper() {
            return Err(Error::Identity);
        }
    }
    Ok(())
}

/// Original rationals enter the existing bounded scalar decoder on the same caller ledger.
fn decode<P: Scalar>(
    value: &Rational,
    plan: &Plan<P>,
    context: &ArithmeticContext,
    budget: &mut Budget,
) -> Result<DecodedLiteral<P>, Error> {
    let text = value.exact_string(budget)?;
    budget.work(plan.literals.max_work(), super::super::AT)?;
    nrz_scalar::NumericLiteral::new(&text, plan.literals)
        .map_err(crate::PhasorError::from)?
        .decode(context)
        .map_err(crate::PhasorError::from)
        .map_err(Error::from)
}
