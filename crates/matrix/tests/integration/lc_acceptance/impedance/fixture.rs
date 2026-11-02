// SPDX-License-Identifier: AGPL-3.0-only
//! All impedance and current declarations precede the first numerical driven output.

use crate::phasor_solve_testing::{declarations, inputs};
use crate::reading_testing::TestResult;
use nrz_bindings::{
    Budget,
    readings::{
        CoordinateNormalization, PhasorBlockCorrespondence, PhasorConvention, PhasorResidualNorm,
        PhasorResidualTransfer, PhasorSolveDefinition, ReadingRelation,
    },
};
use nrz_matrix::{
    CompanionAdapter, DependencyPolicy, EntryTicket, Extent, Heap, PhasorOperand, PhasorSolve,
    PhasorSolvePublication, ReadingAdapter, ReadingBinding, ReadingCoverage, ReadingRegion,
};
use nrz_scalar::{LiteralLimits, Scalar};
use std::rc::Rc;

/// Complete fresh impedance group and future current slot, with an independently imported voltage.
pub(super) struct Row<P: Scalar> {
    /// Caller Work for original correspondence, frames, group planning and voltage import.
    pub(super) declaration_work: u64,
    /// One graph owns the original equation and all output symbols.
    pub(super) graph: Rc<nrz_bindings::readings::algebra::Cas>,
    /// Original complete equation; its impedance has no numerical allocation yet.
    pub(super) definition: PhasorSolveDefinition,
    /// Whole impedance and all its exact scalar copies.
    pub(super) impedance: ReadingCoverage<usize>,
    /// Own-target total and numerical radii, grouped in separate regions.
    pub(super) radii: Vec<CompanionAdapter<usize>>,
    /// Fresh original current group, declared before numerical impedance production.
    current: ReadingCoverage<usize>,
    /// Every current output radius remains tied to its actual scalar-copy target.
    current_radii: Vec<CompanionAdapter<usize>>,
    /// Original voltage then current residual metrics.
    frames: [CoordinateNormalization; 3],
    /// Actual complete original voltage source; no impedance is imported by this fixture.
    voltage: Vec<ReadingBinding<P>>,
}

impl<P: Scalar + 'static> Row<P> {
    /// Reuse the exact physical fixtures while reserving the impedance for the actual producer.
    pub(super) fn new(heap: &mut Heap, reactance: i64, budget: &mut Budget) -> TestResult<Self> {
        let before = budget.usage().work;
        let mut original =
            declarations::fixture().map_err(|cause| format!("phasor declarations: {cause:?}"))?;
        let (adapters, relations, current, coverage, companions) =
            declarations::readings_fixed(&mut original, Some(reactance))
                .map_err(|cause| format!("original fixed readings: {cause:?}"))?;
        let registry = declarations::registry(&adapters, &current)?;
        let correspondence = PhasorBlockCorrespondence::prove(
            &mut original.cas,
            [adapters[0].representation().clone(), current],
            PhasorConvention::PositiveTimePassive,
            budget,
        )
        .map_err(|cause| format!("original phasor correspondence fixture: {cause:?}"))?;
        let definition = PhasorSolveDefinition::new(
            &original.cas,
            correspondence,
            adapters[5].representation().clone(),
            &registry,
            budget,
        )
        .map_err(|cause| format!("original fixed current equation: {cause:?}"))?;
        let frames = declarations::frames(&mut original, &definition, budget)
            .map_err(|cause| format!("original residual frames: {cause:?}"))?;
        let parameters = inputs::companions(&mut original, &adapters)
            .map_err(|cause| format!("original source companions: {cause:?}"))?;
        let (impedance, radii) = group(&adapters[..5], &relations[..4], &parameters[..8], budget)
            .map_err(|cause| format!("fresh impedance group: {cause:?}"))?;
        let (voltage, voltage_radii) =
            group(&adapters[5..], &relations[4..], &parameters[8..], budget)
                .map_err(|cause| format!("original voltage group: {cause:?}"))?;
        let graph = Rc::new(original.cas);
        let regions = [
            ReadingRegion {
                extent: Extent::new(2, 2)?,
                values: vec![P::from_u8(3), P::ZERO, P::ZERO, P::from_u8(3)],
            },
            ReadingRegion {
                extent: Extent::new(4, 1)?,
                values: vec![P::ZERO; 4],
            },
            ReadingRegion {
                extent: Extent::new(4, 1)?,
                values: vec![P::ZERO; 4],
            },
        ];
        let voltage = heap.import_readings(
            Rc::clone(&graph),
            voltage,
            voltage_radii,
            &regions,
            DependencyPolicy::Entries,
            budget,
        )?;
        Ok(Self {
            declaration_work: budget.usage().work - before,
            graph,
            definition,
            impedance,
            radii,
            current: coverage,
            current_radii: companions,
            frames,
            voltage: voltage.bindings().to_vec(),
        })
    }

    /// Bind predeclared current slots only to the actually produced original impedance copies.
    pub(super) fn current(
        &self,
        impedance: &[ReadingBinding<P>],
        budget: &mut Budget,
    ) -> TestResult<PhasorSolvePublication<P>> {
        let operands = [operand(impedance, budget)?, operand(&self.voltage, budget)?];
        let transfer = PhasorResidualTransfer::new(
            self.definition.clone(),
            [self.frames[0].clone(), self.frames[1].clone()],
            PhasorResidualNorm::ComponentEuclidean,
            budget,
        )?;
        let solve = PhasorSolve::new(transfer, operands, LiteralLimits::default(), budget)?;
        Ok(PhasorSolvePublication::new(
            solve,
            self.current.clone(),
            self.current_radii.clone(),
            budget,
        )?)
    }

    /// Actual original voltage versions are available for the RHS-only mutation twin.
    pub(super) fn voltage(&self) -> &ReadingBinding<P> {
        &self.voltage[0]
    }
}

/// Preserve exact selected identities while assigning each independent group its own local regions.
fn group(
    adapters: &[ReadingAdapter<usize>],
    relations: &[ReadingRelation],
    parameters: &[CompanionAdapter<usize>],
    budget: &mut Budget,
) -> TestResult<(ReadingCoverage<usize>, Vec<CompanionAdapter<usize>>)> {
    let readings = adapters
        .iter()
        .map(|adapter| {
            let entries = adapter
                .entries()
                .iter()
                .map(|entry| EntryTicket {
                    region: 0,
                    index: entry.index,
                })
                .collect();
            ReadingAdapter::new(adapter.representation().clone(), entries, budget)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut companions = Vec::new();
    for (index, parameter) in parameters.iter().enumerate() {
        let component = index / 2;
        companions.push(CompanionAdapter::new(
            parameter.companion().clone(),
            vec![EntryTicket {
                region: index % 2 + 1,
                index: u64::try_from(component)?,
            }],
            vec![readings[component + 1].clone()],
            budget,
        )?);
    }
    let entries = (0..4)
        .map(|index| EntryTicket { region: 0, index })
        .collect();
    Ok((
        ReadingCoverage::new(entries, readings, relations.to_vec(), budget)?,
        companions,
    ))
}

/// Every actual whole block and each of its four stored copies retain their complete radii.
pub(super) fn operand<P: Scalar>(
    readings: &[ReadingBinding<P>],
    budget: &mut Budget,
) -> TestResult<PhasorOperand<P>> {
    Ok(PhasorOperand::new(
        readings[0].clone(),
        readings[1..5]
            .to_vec()
            .try_into()
            .map_err(|_| "four impedance copies")?,
        budget,
    )?)
}
