// SPDX-License-Identifier: AGPL-3.0-only
//! An isolated mapped-source fixture leaves every native declaration and pin unchanged.

mod companions;
mod readings;
use super::{Fixture, TestResult, exact, ledger, original, source};
use nrz_bindings::readings::algebra::syntax::{AtomicSort, Declaration, Extent, Field, Sort};
use nrz_bindings::{
    Limits,
    readings::{CoordinateNormalization, LcSpectralLaw},
};
use std::rc::Rc;

impl Fixture {
    /// Original L/C inputs read signed/scaled copies of two separately declared original sources.
    pub(crate) fn with_mapped_sources(limits: Limits) -> TestResult<Self> {
        Self::mapped(limits, false)
    }

    /// The same exact copies can declare original and copied radii for conservative refusal twins.
    pub(crate) fn with_mapped_copy_companions(limits: Limits) -> TestResult<Self> {
        Self::mapped(limits, true)
    }

    /// Only the refusal fixture adds radius declarations; existing owner pins retain their graph.
    fn mapped(limits: Limits, radii: bool) -> TestResult<Self> {
        let (mut source, coverage, parameters) = original(limits)?;
        let context = source::real::context_with_declarations(
            &mut source,
            false,
            inventory(radii),
            &mut ledger()?,
        )?;
        let entries = [
            exact::meaning("voltage", [2, 1, -3, -1, 0, 0, 0], "5", &mut ledger()?)?,
            exact::meaning("current", [0, 0, 0, 1, 0, 0, 0], "1/10", &mut ledger()?)?,
        ];
        let state =
            CoordinateNormalization::new(&mut source.cas, &context, &entries, &mut ledger()?)?;
        let time = exact::meaning("duration", [0, 0, 1, 0, 0, 0, 0], "1/2000", &mut ledger()?)?;
        let recipe = exact::recipe(&mut source, &context, &state, &time, &mut ledger()?)?;
        let law = LcSpectralLaw::check(
            &mut source.cas,
            coverage,
            parameters,
            recipe,
            state,
            time,
            &mut ledger()?,
        )?;
        let mut budget = ledger()?;
        let coverage = readings::coverage(&mut source, &context, &law, &mut budget)?;
        let companions = if radii {
            companions::define(&mut source, &context, &coverage, &mut budget)?
        } else {
            Vec::new()
        };
        Ok(Self {
            graph: Rc::new(source.cas),
            law,
            input_construction_work: budget.usage().work,
            physical_construction_work: 0,
            coverage,
            companions,
            outputs: None,
        })
    }
}

/// Only this fixture adds the source and copied-family declarations.
fn inventory(radii: bool) -> Vec<Declaration> {
    let mut result = Vec::new();
    for prefix in ["L", "C"] {
        for (suffix, rows) in [("source", 0), ("negative", 0), ("expanded", 3)] {
            let key = format!("mapped_{prefix}_{suffix}");
            let sort = if rows == 0 {
                Sort::Value(AtomicSort::Scalar(Field::Real))
            } else {
                Sort::Value(AtomicSort::Matrix {
                    field: Field::Real,
                    rows: Extent::Literal(rows),
                    columns: Extent::Literal(1),
                })
            };
            result.push(Declaration {
                display_name: key.clone(),
                key,
                sort,
            });
        }
    }
    if radii {
        result.extend(companions::inventory());
    }
    result
}
