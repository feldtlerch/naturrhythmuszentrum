// SPDX-License-Identifier: AGPL-3.0-only
//! Exact selected-copy identities cover every stored source and both signed expansions.

use super::super::{TestResult, source, symbol};
use nrz_bindings::readings::algebra::{
    Handle, Rational, Reference,
    readings::{AffineMap, EntryIndex, SelectionSpec},
    syntax::{AtomicSort, Draft, ExpressionDraft, Extent, Field, Operator, Sort},
};
use nrz_bindings::{
    Budget,
    readings::{
        CoordinateFamily, LcSpectralLaw, PhysicalReading, ReadingDefinition, ReadingRelation,
        Representation, RepresentationDefinition,
    },
};
use nrz_matrix::{EntryTicket, ReadingAdapter, ReadingCoverage};

/// Cover original sources and each selected signed copy, with exact overlap evidence.
pub(super) fn coverage(
    source: &mut source::Fixture,
    context: &Handle,
    law: &LcSpectralLaw,
    budget: &mut Budget,
) -> TestResult<ReadingCoverage<usize>> {
    let mut adapters = Vec::new();
    let mut required = Vec::new();
    for (side, prefix) in ["L", "C"].into_iter().enumerate() {
        let stored = symbol(
            source,
            if side == 0 {
                "inductance_stored"
            } else {
                "capacitance_stored"
            },
            true,
            budget,
        )?;
        let meaning = law.operator().frame().entries()[side].clone();
        let family =
            CoordinateFamily::new(&source.cas, context, &stored, vec![meaning.clone()], budget)?;
        let key = format!("mapped_{prefix}_source");
        let original = symbol(source, &key, false, budget)?;
        adapters.push(adapter(source, &family, original, 0, side, budget)?);
        required.push(EntryTicket {
            region: side,
            index: 0,
        });
        let key = format!("mapped_{prefix}_expanded");
        let expanded_symbol = matrix_symbol(source, &key, budget)?;
        let factor = if side == 0 { 2 } else { -1 };
        let coefficients = [factor, -factor, factor]
            .into_iter()
            .map(|value| Rational::from_ratio(value, 1, budget))
            .collect::<Result<Vec<_>, _>>()?;
        let map = AffineMap::new(1, 3, coefficients, vec![Rational::zero(); 3], budget)?;
        let expanded = family.expand(
            &mut source.cas,
            &expanded_symbol,
            &map,
            vec![meaning; 3],
            budget,
        )?;
        for row in 0..3 {
            let symbol = if row == 1 {
                super::super::symbol(source, &format!("mapped_{prefix}_negative"), false, budget)?
            } else {
                law.operator().inputs()[side].clone()
            };
            adapters.push(adapter(source, &expanded, symbol, row, side + 2, budget)?);
            required.push(EntryTicket {
                region: side + 2,
                index: u64::from(row),
            });
        }
    }
    // Additional aliases select the first positive copies through the same physical reading.
    let mut overlaps = Vec::new();
    for index in [1, 5] {
        let original = adapters[index].clone();
        let left = original
            .reading()
            .project_selected(&mut source.cas, &[0], budget)?;
        let right = original
            .reading()
            .project_selected(&mut source.cas, &[0], budget)?;
        overlaps.push(ReadingRelation::prove(
            &mut source.cas,
            left,
            right,
            budget,
        )?);
        adapters.push(original);
    }
    Ok(ReadingCoverage::new(required, adapters, overlaps, budget)?)
}

/// An identity physical reading retains its exact selected family coordinate.
fn adapter(
    source: &mut source::Fixture,
    family: &CoordinateFamily,
    symbol: Handle,
    row: u32,
    region: usize,
    budget: &mut Budget,
) -> TestResult<ReadingAdapter<usize>> {
    let reading = PhysicalReading::define(
        &mut source.cas,
        family,
        ReadingDefinition {
            symbol: symbol.clone(),
            selection: SelectionSpec::Entry(EntryIndex { row, column: 0 }),
            map: AffineMap::identity(1, budget)?,
            entries: vec![family.entries()[usize::try_from(row)?].clone()],
        },
        budget,
    )?;
    let representation = Representation::prove(
        &mut source.cas,
        &reading,
        RepresentationDefinition::Defined {
            symbol,
            expansion: AffineMap::identity(1, budget)?,
        },
        budget,
    )?;
    Ok(ReadingAdapter::new(
        representation,
        vec![EntryTicket {
            region,
            index: u64::from(row),
        }],
        budget,
    )?)
}

/// Admit the predeclared complete three-entry expansion symbol.
fn matrix_symbol(
    source: &mut source::Fixture,
    key: &str,
    budget: &mut Budget,
) -> TestResult<Handle> {
    Ok(source
        .cas
        .admit(
            &[Draft::Expression(ExpressionDraft {
                operator: Operator::Free(key.into()),
                sort: Sort::Value(AtomicSort::Matrix {
                    field: Field::Real,
                    rows: Extent::Literal(3),
                    columns: Extent::Literal(1),
                }),
                header: Reference::Existing(source.header.clone()),
                scope: Vec::new(),
                binding: None,
                space: None,
                operands: Vec::new(),
            })],
            budget,
        )?
        .pop()
        .ok_or("mapped expansion symbol")?)
}
