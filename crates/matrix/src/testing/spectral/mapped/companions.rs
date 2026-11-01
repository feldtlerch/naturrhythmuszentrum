// SPDX-License-Identifier: AGPL-3.0-only
//! Original and copied L radii are separate declarations; center equality cannot validate them.

use super::super::{TestResult, source, symbol};
use nrz_bindings::{
    Budget, CompanionKind,
    readings::{
        CompanionParameters, CompanionReading, CompanionReadingKind,
        algebra::{
            Handle,
            syntax::{AtomicSort, Declaration, Extent, Field, Sort},
        },
    },
};
use nrz_matrix::{CompanionAdapter, EntryTicket, ReadingCoverage};

/// Only the companion-refusal fixture adds these independent parameter symbols.
pub(super) fn inventory() -> Vec<Declaration> {
    let mut result = Vec::new();
    for target in ["original", "copy"] {
        for channel in ["total", "error"] {
            for suffix in ["source", "output"] {
                let key = format!("mapped_{target}_{channel}_{suffix}");
                result.push(Declaration {
                    display_name: key.clone(),
                    key,
                    sort: Sort::Value(AtomicSort::Matrix {
                        field: Field::Real,
                        rows: Extent::Literal(1),
                        columns: Extent::Literal(1),
                    }),
                });
            }
        }
    }
    result
}

/// Four independent parameter coordinates belong to one additional numerical region.
pub(super) fn define(
    source: &mut source::Fixture,
    context: &Handle,
    coverage: &ReadingCoverage<usize>,
    budget: &mut Budget,
) -> TestResult<Vec<CompanionAdapter<usize>>> {
    let mut result = Vec::new();
    for (index, prefix) in [(0, "original"), (1, "copy")] {
        let target = &coverage.readings()[index];
        for (channel, kind) in [
            ("total", CompanionKind::DeterministicBound),
            ("error", CompanionKind::NumericalError),
        ] {
            let parameter = symbol(
                source,
                &format!("mapped_{prefix}_{channel}_source"),
                true,
                budget,
            )?;
            let parameters = CompanionParameters::new(
                &source.cas,
                context,
                &parameter,
                CompanionReadingKind::Radius(kind),
                std::slice::from_ref(target.reading()),
                budget,
            )?;
            let output = symbol(
                source,
                &format!("mapped_{prefix}_{channel}_output"),
                true,
                budget,
            )?;
            let reading = CompanionReading::define(
                &mut source.cas,
                &output,
                &parameters,
                std::slice::from_ref(target.reading()),
                budget,
            )?;
            result.push(CompanionAdapter::new(
                reading,
                vec![EntryTicket {
                    region: 4,
                    index: u64::try_from(result.len())?,
                }],
                vec![target.clone()],
                budget,
            )?);
        }
    }
    Ok(result)
}
