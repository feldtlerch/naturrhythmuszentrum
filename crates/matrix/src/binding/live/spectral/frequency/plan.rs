// SPDX-License-Identifier: AGPL-3.0-only
//! A complete fresh inventory and certified coefficient precede connected numerical publication.

use super::{AT, Error};
use crate::{
    CompanionAdapter, ConversionContract, EntryTicket, HeapError, LcSpectralEvaluation,
    OperationError, PiConstant, ReadingAdapter, ReadingCoverage,
};
use nrz_bindings::{
    Budget, LimitsOverride,
    readings::{
        LcFrequencyDefinition,
        algebra::{Cas, limits::BudgetKind},
    },
};
use nrz_scalar::{ArithmeticContext, Scalar};
use std::rc::Rc;

/// Prepared connected output consumes an existing native evaluation without evaluating its root again.
#[derive(Clone, Debug)]
pub struct LcFrequencyPublication<P: Scalar>(pub(super) Rc<Plan<P>>);

/// Shared immutable input proof and complete output layout, with no published output prefix.
#[derive(Debug)]
pub(super) struct Plan<P: Scalar> {
    /// Original evaluated root, checked derivative, joint model and operating-point policy.
    pub(super) evaluation: LcSpectralEvaluation<P>,
    /// Sealed angular/cyclic definitions and ordered covariance/error targets.
    pub(super) definition: LcFrequencyDefinition,
    /// Exact graph shared with the original input readings.
    pub(super) graph: Rc<Cas>,
    /// Angular value/error, cyclic value/error and cross-error source readings.
    pub(super) coverage: ReadingCoverage<usize>,
    /// Three angular, three cyclic and one ordered cross-covariance companion.
    pub(super) companions: Vec<CompanionAdapter<usize>>,
    /// Flattened closure includes original sources and their earlier producers.
    pub(super) sources: Vec<crate::binding::live::selected::SelectedReading>,
    /// Immutable exact-map-bound coefficient pair certified through the existing conversion route.
    pub(super) map: Rc<crate::coordinate_numeric::LoweredMap<P>>,
    /// Shared original constant region; no new stochastic source is introduced.
    pub(super) pi: PiConstant<P>,
    /// Conservative retained metadata allowance for the prepared publication.
    pub(super) bytes: u64,
}

impl<P: Scalar + 'static> LcFrequencyPublication<P> {
    /// Bind a retained original evaluation to fresh connected symbols and a certified full-cycle map.
    /// # Errors
    /// Refuses substituted laws/maps, absent statistics, reused inputs, unsupported constants or resources.
    pub fn new(
        evaluation: LcSpectralEvaluation<P>,
        definition: LcFrequencyDefinition,
        conversion: &ConversionContract,
        pi: &PiConstant<P>,
        exact: &LimitsOverride,
        context: &ArithmeticContext,
        budget: &mut Budget,
    ) -> Result<Self, Error> {
        let spectrum = evaluation.plan();
        if !spectrum.law().shares_allocation(definition.angular().law())
            || spectrum.root_jacobian().is_none()
            || conversion.map() != definition.map()
            || conversion.input.shape().extent().entries() != 1
        {
            return Err(Error::Identity);
        }
        let (coverage, companions) = inventory(&definition, budget)?;
        for input in spectrum.inputs() {
            for output in coverage.readings() {
                budget.work(1, AT)?;
                if input.reading().family().root() == output.reading().family().root() {
                    return Err(Error::Identity);
                }
            }
        }
        conversion
            .certify_pi(pi, exact, context, budget)
            .map_err(OperationError::from)?;
        let cache = conversion.cache.try_borrow().map_err(|_| Error::Identity)?;
        budget.work(cache.lookup_work().map_err(OperationError::from)?, AT)?;
        let certified = cache
            .find_pi(&conversion.map, pi, context)
            .ok_or(Error::Identity)?;
        let sources = super::super::publication::sources(spectrum.inputs(), budget)?;
        let bytes = storage::<P>(&coverage, &companions, sources.len(), &definition, budget)?;
        let graph = Rc::clone(&spectrum.inputs()[0].group.graph);
        budget.check(BudgetKind::OutputBytes, bytes, AT)?;
        Ok(Self(Rc::new(Plan {
            evaluation,
            definition,
            graph,
            coverage,
            companions,
            sources,
            map: Rc::clone(&certified.map),
            pi: certified.pi.clone(),
            bytes,
        })))
    }

    /// Complete exact meaning and ordered output inventory.
    #[must_use]
    pub fn definition(&self) -> &LcFrequencyDefinition {
        &self.0.definition
    }

    /// Original retained native evaluation, including the original caller receipts.
    #[must_use]
    pub fn evaluation(&self) -> &LcSpectralEvaluation<P> {
        &self.0.evaluation
    }

    /// Newly retained publication metadata allowance; original evidence owners remain shared.
    #[must_use]
    pub fn retained_payload_bytes(&self) -> u64 {
        self.0.bytes
    }
}

/// Twelve distinct regions are covered by five readings and seven exact companion adapters.
fn inventory(
    definition: &LcFrequencyDefinition,
    budget: &mut Budget,
) -> Result<(ReadingCoverage<usize>, Vec<CompanionAdapter<usize>>), Error> {
    let angular = definition.angular().symbols();
    let cyclic = definition.symbols();
    let entries: Vec<_> = [0, 4, 5, 9, 11]
        .into_iter()
        .map(|region| EntryTicket { region, index: 0 })
        .collect();
    let readings = [
        &angular.value,
        &angular.covariance_error_value,
        &cyclic.cyclic,
        &cyclic.covariance_error_value,
        &cyclic.cross_error_value,
    ]
    .into_iter()
    .zip(&entries)
    .map(|(value, entry)| ReadingAdapter::new(value.clone(), vec![entry.clone()], budget))
    .collect::<Result<Vec<_>, _>>()
    .map_err(crate::LiveBindingError::from)?;
    let mut companions = Vec::new();
    for (channel, symbols) in [&angular.companions, &cyclic.companions]
        .into_iter()
        .enumerate()
    {
        for (index, symbol) in symbols.iter().enumerate() {
            companions.push(
                CompanionAdapter::new(
                    symbol.clone(),
                    vec![EntryTicket {
                        region: channel * 5 + index + 1,
                        index: 0,
                    }],
                    vec![readings[channel * 2].clone(); if index == 2 { 2 } else { 1 }],
                    budget,
                )
                .map_err(crate::LiveBindingError::from)?,
            );
        }
    }
    companions.push(
        CompanionAdapter::new(
            cyclic.cross_covariance.clone(),
            vec![EntryTicket {
                region: 10,
                index: 0,
            }],
            vec![readings[0].clone(), readings[2].clone()],
            budget,
        )
        .map_err(crate::LiveBindingError::from)?,
    );
    let coverage = ReadingCoverage::new(entries, readings, Vec::new(), budget)
        .map_err(crate::LiveBindingError::from)?;
    Ok((coverage, companions))
}

/// Shared original owners are not copied; new adapters and declaration metadata receive allowances.
fn storage<P: Scalar>(
    coverage: &ReadingCoverage<usize>,
    companions: &[CompanionAdapter<usize>],
    sources: usize,
    definition: &LcFrequencyDefinition,
    budget: &mut Budget,
) -> Result<u64, Error> {
    let base =
        super::super::publication::adapter_storage::<P>(coverage, companions, sources, budget)?;
    let bytes = base
        .checked_add(definition.retained_payload_bytes())
        .and_then(|value| {
            value.checked_add(u64::try_from(size_of::<Plan<P>>() + 2 * size_of::<usize>()).ok()?)
        })
        .ok_or(HeapError::ExtentOverflow)?;
    budget.check(BudgetKind::StagedPayloadBytes, bytes, AT)?;
    Ok(bytes)
}
