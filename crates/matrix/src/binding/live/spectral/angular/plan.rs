// SPDX-License-Identifier: AGPL-3.0-only
//! Every output and companion binding is checked before the existing native evaluator is called.

use super::{AT, Error};
use crate::{
    CompanionAdapter, EntryTicket, HeapError, LcSpectrum, ReadingAdapter, ReadingCoverage,
};
use nrz_bindings::{
    Budget,
    readings::{
        LcAngularFrequencyDefinition,
        algebra::{Cas, limits::BudgetKind},
    },
};
use nrz_scalar::Scalar;
use std::rc::Rc;

/// Original statistical LC plan and a sealed, complete angular-frequency output inventory.
#[derive(Clone, Debug)]
pub struct LcAngularFrequencyPublication<P: Scalar>(pub(super) Rc<Plan<P>>);

/// Immutable exact and numerical source owners are retained beside the complete output layout.
#[derive(Debug)]
pub(super) struct Plan<P: Scalar> {
    /// Native original-center law, source Jacobian, joint model and live input closure.
    pub(super) spectrum: LcSpectrum<P>,
    /// Sealed angular-frequency interpretation and complete companion definitions.
    pub(super) definition: LcAngularFrequencyDefinition,
    /// Original exact graph, including the predeclared fresh output header.
    pub(super) graph: Rc<Cas>,
    /// Angular-frequency value and covariance-parameter-error source reading.
    pub(super) coverage: ReadingCoverage<usize>,
    /// Total radius, numerical radius and self covariance in the fixed declared order.
    pub(super) companions: Vec<CompanionAdapter<usize>>,
    /// Every original source and its earlier producer ancestors.
    pub(super) sources: Vec<crate::binding::live::selected::SelectedReading>,
    /// Newly retained publication metadata allowance.
    pub(super) bytes: u64,
}

impl<P: Scalar + 'static> LcAngularFrequencyPublication<P> {
    /// Bind the complete fresh angular-frequency inventory to an original statistical LC plan.
    /// # Errors
    /// Refuses absent statistics, substituted laws, reused input roots and exhausted resources.
    pub fn new(
        spectrum: LcSpectrum<P>,
        definition: LcAngularFrequencyDefinition,
        budget: &mut Budget,
    ) -> Result<Self, Error> {
        if !spectrum.law().shares_allocation(definition.law()) || spectrum.root_jacobian().is_none()
        {
            return Err(Error::Identity);
        }
        let symbols = definition.symbols();
        for input in spectrum.inputs() {
            for output in [&symbols.value, &symbols.covariance_error_value] {
                budget.work(1, AT)?;
                if input.reading().family().root() == output.reading().family().root() {
                    return Err(Error::Identity);
                }
            }
        }
        let readings = [&symbols.value, &symbols.covariance_error_value]
            .into_iter()
            .zip([0, 4])
            .map(|(output, region)| {
                ReadingAdapter::new(
                    output.clone(),
                    vec![EntryTicket { region, index: 0 }],
                    budget,
                )
            })
            .collect::<Result<Vec<_>, _>>()
            .map_err(crate::LiveBindingError::from)?;
        let mut companions = Vec::new();
        for (index, companion) in symbols.companions.iter().enumerate() {
            companions.push(
                CompanionAdapter::new(
                    companion.clone(),
                    vec![EntryTicket {
                        region: index + 1,
                        index: 0,
                    }],
                    vec![readings[0].clone(); if index == 2 { 2 } else { 1 }],
                    budget,
                )
                .map_err(crate::LiveBindingError::from)?,
            );
        }
        let coverage = ReadingCoverage::new(
            vec![
                EntryTicket {
                    region: 0,
                    index: 0,
                },
                EntryTicket {
                    region: 4,
                    index: 0,
                },
            ],
            readings,
            Vec::new(),
            budget,
        )
        .map_err(crate::LiveBindingError::from)?;
        let sources = super::super::publication::sources(spectrum.inputs(), budget)?;
        let bytes = storage::<P>(
            &coverage,
            &companions,
            sources.len(),
            definition.retained_payload_bytes(),
            budget,
        )?;
        let graph = Rc::clone(&spectrum.inputs()[0].group.graph);
        budget.check(BudgetKind::OutputBytes, bytes, AT)?;
        Ok(Self(Rc::new(Plan {
            spectrum,
            definition,
            graph,
            coverage,
            companions,
            sources,
            bytes,
        })))
    }
    /// Original statistical plan, including its exact Jacobian and source selection identities.
    #[must_use]
    pub fn spectrum(&self) -> &LcSpectrum<P> {
        &self.0.spectrum
    }
    /// Sealed original physical output definition and every predeclared parameter symbol.
    #[must_use]
    pub fn definition(&self) -> &LcAngularFrequencyDefinition {
        &self.0.definition
    }
    /// Newly retained publication allowance; original source/proof owners are shared.
    #[must_use]
    pub fn retained_payload_bytes(&self) -> u64 {
        self.0.bytes
    }
}

/// Reuse the existing adapter inventory allowance and add the new exact output owner.
fn storage<P: Scalar>(
    coverage: &ReadingCoverage<usize>,
    companions: &[CompanionAdapter<usize>],
    sources: usize,
    exact: u64,
    budget: &mut Budget,
) -> Result<u64, Error> {
    let base =
        super::super::publication::adapter_storage::<P>(coverage, companions, sources, budget)?;
    let bytes = base
        .checked_add(exact)
        .and_then(|value| value.checked_add(u64::try_from(size_of::<Plan<P>>()).ok()?))
        .ok_or(HeapError::ExtentOverflow)?;
    budget.check(BudgetKind::StagedPayloadBytes, bytes, AT)?;
    Ok(bytes)
}
