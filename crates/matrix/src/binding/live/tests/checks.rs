// SPDX-License-Identifier: AGPL-3.0-only
//! Negative mutations and complete resource boundaries of the live declaration path.

use super::*;
use crate::binding::adapters::fixtures::{complex_case, map, meaning};
use nrz_bindings::readings::{
    CompanionParameters, CompanionReading, CompanionReadingKind, CoordinateFamily,
};
use nrz_bindings::{CompanionKind, ParameterEntry, ParameterSymbol, ParameterTarget};

/// Logical entry tickets in their original numerical order.
fn tickets(region: usize, indices: &[u64]) -> Vec<EntryTicket<usize>> {
    indices
        .iter()
        .map(|&index| EntryTicket { region, index })
        .collect()
}

/// Build a constant exact reading with no numerical occupation.
fn none_plan() -> TestResult<Plan> {
    let mut case = state_case()?;
    let fixed = case.fixture.define(
        case.whole.family(),
        "v",
        SelectionSpec::Column(0),
        map(2, 1, &[0, 0], &[7])?,
        case.component.entries().to_vec(),
    )?;
    let representation = Representation::prove(
        &mut case.fixture.cas,
        &fixed,
        RepresentationDefinition::Exact,
        &mut ledger()?,
    )?;
    let adapter = ReadingAdapter::new(representation, Vec::new(), &mut ledger()?)?;
    let coverage = ReadingCoverage::new(Vec::new(), vec![adapter], Vec::new(), &mut ledger()?)?;
    Ok((Rc::new(case.fixture.cas), coverage))
}

/// None is live without storage and cannot replace even one missing varying entry.
/// ´test:unit:live-none-never-covers-varying-storage´
#[test]
fn live_none_never_covers_varying_storage() -> TestResult {
    let mut heap = Heap::new(HeapLimits::default())?;
    let (graph, coverage) = none_plan()?;
    let imported = heap.import_readings::<f64>(
        graph,
        coverage,
        Vec::new(),
        &[],
        DependencyPolicy::Entries,
        &mut ledger()?,
    )?;
    let selected = imported.bindings().first().ok_or("none")?;
    assert_eq!(heap.reading_entries(selected)?, [] as [f64; 0]);
    assert!(heap.select_reading(selected, &mut ledger()?).is_err());
    assert!(
        heap.replace_reading_entries(selected, &[], &mut ledger()?)
            .is_err()
    );
    let (graph, coverage) = none_plan()?;
    let missing = ReadingRegion {
        extent: Extent::new(1, 1)?,
        values: vec![1.0_f64],
    };
    assert!(
        heap.import_readings(
            graph,
            coverage,
            Vec::new(),
            &[missing],
            DependencyPolicy::Entries,
            &mut ledger()?
        )
        .is_err()
    );
    assert_eq!(heap.descriptors.iter().count(), 0);
    Ok(())
}

/// One checked Complex block and its repeated real corner share an actual allocation.
fn complex_plan() -> TestResult<Plan> {
    let mut case = complex_case()?;
    let symbol = case.fixture.symbol("block")?;
    let whole = Representation::prove(
        &mut case.fixture.cas,
        &case.whole,
        RepresentationDefinition::ComplexBlock(symbol),
        &mut ledger()?,
    )?;
    let symbol = case.fixture.symbol("r")?;
    let component = Representation::prove(
        &mut case.fixture.cas,
        &case.component,
        RepresentationDefinition::Defined {
            symbol,
            expansion: AffineMap::identity(1, &mut ledger()?)?,
        },
        &mut ledger()?,
    )?;
    let left = case
        .whole
        .project_selected(&mut case.fixture.cas, &[3], &mut ledger()?)?;
    let right = case
        .component
        .project_selected(&mut case.fixture.cas, &[0], &mut ledger()?)?;
    let relation = ReadingRelation::prove(&mut case.fixture.cas, left, right, &mut ledger()?)?;
    let whole = ReadingAdapter::new(whole, tickets(0, &[0, 1, 2, 3]), &mut ledger()?)?;
    let component = ReadingAdapter::new(component, tickets(0, &[3]), &mut ledger()?)?;
    let coverage = ReadingCoverage::new(
        tickets(0, &[0, 1, 2, 3]),
        vec![whole, component],
        vec![relation],
        &mut ledger()?,
    )?;
    Ok((Rc::new(case.fixture.cas), coverage))
}

/// Actual repeated and negative centers are checked independently of the symbolic reconstruction proof.
/// ´test:unit:live-complex-block-checks-every-stored-copy´
#[test]
fn live_complex_block_checks_every_stored_copy() -> TestResult {
    for values in [
        vec![2.0_f64, 3.0, -3.0, 2.0],
        vec![2.0, 3.0, 3.0, 2.0],
        vec![2.0, 3.0, -3.0, 4.0],
    ] {
        let negated = values[1].to_bits() == (-values[2]).to_bits();
        let repeated = values[0].to_bits() == values[3].to_bits();
        let valid = negated && repeated;
        let (graph, coverage) = complex_plan()?;
        let mut heap = Heap::new(HeapLimits::default())?;
        let region = ReadingRegion {
            extent: Extent::new(2, 2)?,
            values,
        };
        let result = heap.import_readings(
            graph,
            coverage,
            Vec::new(),
            &[region],
            DependencyPolicy::Entries,
            &mut ledger()?,
        );
        assert_eq!(result.is_ok(), valid);
        if let Ok(imported) = result {
            let whole = &imported.bindings()[0];
            let corner = &imported.bindings()[1];
            assert_eq!(heap.reading_entries(corner)?, vec![2.0]);
            assert!(
                heap.replace_reading_entries(corner, &[4.0], &mut ledger()?)
                    .is_err()
            );
            heap.replace_reading_entries(whole, &[4.0, 5.0, -5.0, 4.0], &mut ledger()?)?;
            assert_eq!(whole.state(), SymbolState::Unavailable);
            assert_eq!(corner.state(), SymbolState::Unavailable);
        } else {
            assert_eq!(heap.descriptors.iter().count(), 0);
        }
    }
    Ok(())
}

/// Bound and numerical error are distinct exact declarations on one selected value map.
fn companion_plan() -> TestResult<(Plan, Vec<crate::CompanionAdapter<usize>>)> {
    let mut fixture = Fixture::new(&[
        ("z", matrix(2, 1)),
        ("x", matrix(2, 1)),
        ("p", matrix(2, 1)),
        ("r", matrix(2, 1)),
        ("e", matrix(2, 1)),
        ("q", matrix(2, 1)),
    ])?;
    let entry = meaning("voltage", [2, 1, -3, -1, 0, 0, 0])?;
    let family = CoordinateFamily::new(
        &fixture.cas,
        &fixture.context,
        &fixture.symbol("z")?,
        vec![entry.clone(); 2],
        &mut ledger()?,
    )?;
    let reading = fixture.define(
        &family,
        "x",
        SelectionSpec::Column(0),
        AffineMap::identity(2, &mut ledger()?)?,
        vec![entry; 2],
    )?;
    let symbol = fixture.symbol("z")?;
    let representation = Representation::prove(
        &mut fixture.cas,
        &reading,
        RepresentationDefinition::Defined {
            symbol,
            expansion: AffineMap::identity(2, &mut ledger()?)?,
        },
        &mut ledger()?,
    )?;
    let target = ReadingAdapter::new(representation, tickets(0, &[0, 1]), &mut ledger()?)?;
    let mut companions = Vec::new();
    for (region, parameter, result, kind) in [
        (1, "p", "r", CompanionKind::DeterministicBound),
        (2, "e", "q", CompanionKind::NumericalError),
    ] {
        let parameters = CompanionParameters::new(
            &fixture.cas,
            &fixture.context,
            &fixture.symbol(parameter)?,
            CompanionReadingKind::Radius(kind),
            std::slice::from_ref(&reading),
            &mut ledger()?,
        )?;
        let symbol = fixture.symbol(result)?;
        let companion = CompanionReading::define(
            &mut fixture.cas,
            &symbol,
            &parameters,
            std::slice::from_ref(&reading),
            &mut ledger()?,
        )?;
        companions.push(crate::CompanionAdapter::new(
            companion,
            tickets(region, &[0, 1]),
            vec![target.clone()],
            &mut ledger()?,
        )?);
    }
    let coverage = ReadingCoverage::new(
        tickets(0, &[0, 1]),
        vec![target],
        Vec::new(),
        &mut ledger()?,
    )?;
    Ok(((Rc::new(fixture.cas), coverage), companions))
}

/// Companion numerical coverage, enclosure and captured entry revisions are checked together.
/// ´test:unit:live-companions-recheck-maps-and-versions´
#[test]
fn live_companions_recheck_maps_and_versions() -> TestResult {
    for error in [0.25, 2.0, -1.0] {
        let ((graph, coverage), companions) = companion_plan()?;
        let extent = Extent::new(2, 1)?;
        let regions = [
            ReadingRegion {
                extent,
                values: vec![3.0_f64; 2],
            },
            ReadingRegion {
                extent,
                values: vec![1.0; 2],
            },
            ReadingRegion {
                extent,
                values: vec![error; 2],
            },
        ];
        let mut heap = Heap::new(HeapLimits::default())?;
        let result = heap.import_readings(
            graph,
            coverage,
            companions,
            &regions,
            DependencyPolicy::Entries,
            &mut ledger()?,
        );
        assert_eq!(result.is_ok(), error.to_bits() == 0.25_f64.to_bits());
        if let Ok(imported) = result {
            let reading = &imported.bindings()[0];
            assert_eq!(reading.companions().len(), 2);
            assert_eq!(heap.reading_entries(reading)?, vec![3.0; 2]);
            assert_eq!(heap.reading_companion_entries(reading, 0)?, vec![1.0; 2]);
            assert_eq!(heap.reading_companion_entries(reading, 1)?, vec![0.25; 2]);
            assert!(heap.reading_companion_entries(reading, 2).is_err());
            let companion = &reading.group.tickets[1];
            heap.symbols.cover(companion.key)?.versions[0].set(2);
            assert!(heap.reading_entries(reading).is_err());
            assert!(heap.reading_companion_entries(reading, 0).is_err());
        } else {
            assert_eq!(heap.descriptors.iter().count(), 0);
        }
    }
    Ok(())
}

/// Fresh parameter symbols and homogeneous selection admission obey independent exact budget boundaries.
/// ´test:unit:live-declaration-resource-boundaries´
#[test]
fn live_declaration_resource_boundaries() -> TestResult {
    for parameter in [false, true] {
        boundaries(|limits, precharge| {
            let mut budget = Budget::new(limits)?;
            budget.work(precharge, super::super::admission::AT)?;
            if parameter {
                let symbols = crate::binding::symbols::declare(
                    &nrz_bindings::Descriptor::ratio(),
                    Extent::new(1, 1)?,
                    [false, false],
                    &mut ledger()?,
                )?;
                let target = ParameterTarget::new(symbols.value().clone(), 0)?;
                ParameterSymbol::declare([1, 1], vec![ParameterEntry::value(target)], &mut budget)?;
            } else {
                let (graph, coverage) = result_plan()?;
                nrz_bindings::BindingSymbols::from_reading(
                    graph,
                    coverage.readings()[1].reading(),
                    &mut budget,
                )?;
            }
            Ok(budget.usage().work)
        })?;
    }
    Ok(())
}

/// Prepare one actual solve and its separately declared whole and component readings.
fn solved() -> TestResult<(Heap, Vec<ReadingBinding<f64>>)> {
    let mut heap = Heap::new(HeapLimits::default())?;
    let quantity = nrz_bindings::Descriptor::ratio();
    let operator =
        heap.import_represented(Extent::new(2, 2)?, &quantity, &[2.0_f64, 0.0, 0.0, 4.0])?;
    let rhs = heap.import_represented(Extent::new(2, 1)?, &quantity, &[2.0_f64, 8.0])?;
    let limits = crate::ExecutionLimits::default();
    let mut bound = crate::Binding::new(
        crate::solve_testing::draft(2, 1)?.check(limits)?,
        vec![operator.value(), rhs.value()],
        limits,
    )
    .validate(&mut heap)?;
    let (graph, coverage) = result_plan()?;
    let readings = bound.predeclare_result_readings(
        graph,
        coverage,
        DependencyPolicy::Entries,
        &mut ledger()?,
    )?;
    let candidate = bound.evaluate(bound.normalized_inputs()?)?;
    bound.publish(candidate)?;
    Ok((heap, readings))
}

/// Native companion edges cannot be left at an earlier target revision by a value-only write.
/// ´test:unit:live-bounded-value-only-write-refuses´
#[test]
fn live_bounded_value_only_write_refuses() -> TestResult {
    let (mut heap, readings) = solved()?;
    let reading = &readings[0];
    assert_eq!(reading.group.native.len(), 2);
    assert!(matches!(
        heap.select_reading(reading, &mut ledger()?),
        Err(LiveBindingError::Heap(HeapError::MissingCoverage))
    ));
    assert_eq!(heap.reading_entries(reading)?, vec![1.0, 2.0]);
    assert!(matches!(
        heap.replace_reading_entries(reading, &[3.0, 4.0], &mut ledger()?),
        Err(LiveBindingError::Heap(HeapError::MissingCoverage))
    ));
    assert_eq!(heap.reading_entries(reading)?, vec![1.0, 2.0]);
    assert_eq!(reading.state(), SymbolState::Published);
    Ok(())
}

/// A native companion's target and version are part of every captured result reading.
/// ´test:unit:live-result-companion-edge-mismatch-refuses´
#[test]
fn live_result_companion_edge_mismatch_refuses() -> TestResult {
    let (mut heap, readings) = solved()?;
    let reading = &readings[0];
    assert!(heap.reading_entries(reading).is_ok());
    let key = reading.group.native[0].key;
    for (actual, record) in heap.descriptors.iter_mut() {
        if actual == key
            && let crate::heap::records::Meaning::Bound { stamp, .. } = &mut record.meaning
        {
            stamp.version = crate::ValueVersion(2);
        }
    }
    assert!(matches!(
        heap.reading_entries(reading),
        Err(LiveBindingError::Heap(HeapError::StaleEvidence))
    ));
    Ok(())
}

/// A write queries other reading groups on the same exact graph and preserves independent graphs.
/// ´test:unit:live-write-invalidates-proofs-across-reading-groups´
#[test]
fn live_write_invalidates_proofs_across_reading_groups() -> TestResult {
    let (graph, coverage) = state()?;
    let mut heap = Heap::new(HeapLimits::default())?;
    let regions = [ReadingRegion {
        extent: Extent::new(2, 1)?,
        values: vec![1.0_f64; 2],
    }];
    let first = heap.import_readings(
        Rc::clone(&graph),
        coverage.clone(),
        Vec::new(),
        &regions,
        DependencyPolicy::Entries,
        &mut ledger()?,
    )?;
    let second = heap.import_readings(
        graph,
        coverage,
        Vec::new(),
        &regions,
        DependencyPolicy::Entries,
        &mut ledger()?,
    )?;
    let (independent_graph, independent_coverage) = state()?;
    let independent = heap.import_readings(
        independent_graph,
        independent_coverage,
        Vec::new(),
        &regions,
        DependencyPolicy::Entries,
        &mut ledger()?,
    )?;
    heap.replace_reading_entries(&first.bindings()[2], &[0.0], &mut ledger()?)?;
    assert_eq!(second.bindings()[0].state(), SymbolState::Unavailable);
    assert_eq!(second.bindings()[2].state(), SymbolState::Unavailable);
    assert_eq!(heap.reading_entries(&second.bindings()[1])?, vec![1.0]);
    assert_eq!(
        heap.reading_entries(&independent.bindings()[0])?,
        vec![1.0; 2]
    );
    Ok(())
}

/// General companion maps survive structurally through writes, while their value evidence becomes unavailable.
/// ´test:unit:live-companion-map-retained-through-write´
#[test]
fn live_companion_map_retained_through_write() -> TestResult {
    let ((graph, coverage), companions) = companion_plan()?;
    let original = companions[0].clone();
    let extent = Extent::new(2, 1)?;
    let regions = [
        ReadingRegion {
            extent,
            values: vec![3.0_f64; 2],
        },
        ReadingRegion {
            extent,
            values: vec![1.0; 2],
        },
        ReadingRegion {
            extent,
            values: vec![0.25; 2],
        },
    ];
    let mut heap = Heap::new(HeapLimits::default())?;
    let imported = heap.import_readings(
        graph,
        coverage,
        companions,
        &regions,
        DependencyPolicy::Entries,
        &mut ledger()?,
    )?;
    let reading = &imported.bindings()[0];
    assert_eq!(heap.reading_entries(reading)?, vec![3.0; 2]);
    heap.replace_reading_entries(reading, &[4.0, 5.0], &mut ledger()?)?;
    assert!(heap.reading_entries(reading).is_err());
    assert_eq!(
        reading.companions()[0].companion().node(),
        original.companion().node()
    );
    assert_eq!(
        reading.companions()[0].companion().map(),
        original.companion().map()
    );
    assert_eq!(reading.companions()[0].entries(), original.entries());
    assert_eq!(
        reading.companions()[0].targets()[0].entries(),
        original.targets()[0].entries()
    );
    Ok(())
}

/// Identity radius maps preserve their exact symbols and separate channels through selection.
/// ´test:unit:live-homogeneous-facade-selects-general-companions´
#[test]
fn live_homogeneous_facade_selects_general_companions() -> TestResult {
    let ((graph, coverage), companions) = companion_plan()?;
    let extent = Extent::new(2, 1)?;
    let regions = [
        ReadingRegion {
            extent,
            values: vec![3.0_f64; 2],
        },
        ReadingRegion {
            extent,
            values: vec![1.0; 2],
        },
        ReadingRegion {
            extent,
            values: vec![0.25; 2],
        },
    ];
    let mut heap = Heap::new(HeapLimits::default())?;
    let imported = heap.import_readings(
        graph,
        coverage,
        companions,
        &regions,
        DependencyPolicy::Entries,
        &mut ledger()?,
    )?;
    let selected = heap
        .select_reading(&imported.bindings()[0], &mut ledger()?)
        .map_err(|e| format!("selection: {e:?}"))?;
    let original = imported.bindings()[0].companions();
    assert_eq!(
        selected.symbols().bound().ok_or("total symbol")?.node(),
        original[0].companion().node()
    );
    assert_eq!(
        selected.symbols().error().ok_or("error symbol")?.node(),
        original[1].companion().node()
    );
    let total = heap
        .selected_bound(selected.handle())
        .map_err(|e| format!("total view: {e:?}"))?
        .ok_or("total view missing")?;
    assert_eq!(total.region().get(0, 0)?.to_bits(), 1.0_f64.to_bits());
    let error = heap
        .selected_evaluation(selected.handle())
        .map_err(|e| format!("error view: {e:?}"))?
        .ok_or("error view missing")?;
    assert_eq!(error.region().get(0, 0)?.to_bits(), 0.25_f64.to_bits());
    assert_eq!(heap.reading_entries(&imported.bindings()[0])?, vec![3.0; 2]);
    Ok(())
}

/// Result reservation charges exact dependency extraction before any reading becomes available.
/// ´test:unit:live-result-declaration-resource-boundaries´
#[test]
fn live_result_declaration_resource_boundaries() -> TestResult {
    boundaries(|limits, precharge| {
        let mut budget = Budget::new(limits)?;
        budget.work(precharge, super::super::admission::AT)?;
        let mut heap = Heap::new(HeapLimits::default())?;
        let source = heap.import_represented(
            Extent::new(2, 1)?,
            &nrz_bindings::Descriptor::ratio(),
            &[1.0_f64; 2],
        )?;
        let limits = crate::ExecutionLimits::default();
        let mut bound = crate::Binding::new(
            crate::binding_testing::operation(2, 1)?,
            vec![source.value()],
            limits,
        )
        .validate(&mut heap)?;
        let (graph, coverage) = result_plan()?;
        bound.predeclare_result_readings(
            graph,
            coverage,
            DependencyPolicy::Entries,
            &mut budget,
        )?;
        Ok(budget.usage().work)
    })
}

/// Reading imports combine descriptor and arena growth before either table changes.
/// ´test:unit:loan-reading-import-combined-ceiling´
#[test]
fn loan_reading_import_combined_ceiling() -> TestResult {
    let mut heap = Heap::new(HeapLimits::new(8192, 1_048_576, 4096))?;
    let extent = Extent::new(1, 1)?;
    let quantity = nrz_bindings::Descriptor::ratio();
    let first = heap
        .import_represented(extent, &quantity, &[1.0_f64])?
        .value();
    for _ in 1..4096 {
        heap.import_represented(extent, &quantity, &[1.0_f64])?;
    }
    let (graph, coverage) = state()?;
    let region = ReadingRegion {
        extent: Extent::new(2, 1)?,
        values: vec![1.0_f64; 2],
    };
    let before = [
        heap.descriptors.slots.owned_bytes()?,
        heap.storage.owned_bytes()?,
    ];
    let injection = crate::loan::fault::Injection::after(0);
    let result = heap.import_readings(
        graph,
        coverage,
        Vec::new(),
        std::slice::from_ref(&region),
        DependencyPolicy::Entries,
        &mut ledger()?,
    );
    assert!(
        matches!(
            result,
            Err(LiveBindingError::Heap(HeapError::ResourceBytes {
                staged: false,
                used: 1_310_784,
                limit: 1_048_576
            }))
        ),
        "{result:?}"
    );
    assert_eq!(crate::loan::fault::Injection::attempts(), 0);
    drop(injection);
    assert_eq!(
        [
            heap.descriptors.slots.owned_bytes()?,
            heap.storage.owned_bytes()?
        ],
        before
    );
    heap.retire(first)?;
    let (graph, coverage) = state()?;
    let imported = heap.import_readings(
        graph,
        coverage,
        Vec::new(),
        &[region],
        DependencyPolicy::Entries,
        &mut ledger()?,
    )?;
    assert_eq!(imported.bindings().len(), 3);
    Ok(())
}

/// A represented reading's full raw base copy joins its replacement slab before export.
fn raw_copy_boundaries<P: Scalar + 'static>() -> TestResult {
    for succeeds in [false, true] {
        let (mut heap, imported) = imported::<P>(DependencyPolicy::Entries)?;
        let binding = imported.bindings().first().ok_or("reading")?;
        let staged = 2 * 64
            + size_of::<faer::Mat<P>>()
            + 3 * size_of::<crate::heap::identity::Key>()
            // Replacement owns key, old region, raw-copy/indices headers and successor version.
            + size_of::<crate::heap::identity::Key>()
            + 2 * size_of::<crate::storage::Region>()
            + 2 * size_of::<Vec<usize>>()
            + size_of::<crate::ValueVersion>()
            + 2 * size_of::<P>();
        let limit = staged - usize::from(!succeeds);
        let injection = crate::loan::fault::Injection::after(if succeeds { usize::MAX } else { 0 });
        let result = crate::binding::live::write::replace_with_policy(
            &mut heap,
            &binding.group,
            binding.adapter().entries(),
            &[P::from_u8(3), P::from_u8(4)],
            &mut ledger()?,
            crate::heap::resources::ResourcePolicy {
                output: 64,
                staged: u64::try_from(limit)?,
            },
        );
        match result {
            Ok(()) => {
                assert!(succeeds);
                assert_eq!(crate::loan::fault::Injection::raw_copies(), 1);
                assert_eq!(crate::loan::fault::Injection::write_buffers(), [2, 168]);
            }
            Err(error) => {
                assert!(!succeeds, "{error:?}");
                assert!(
                    matches!(error, LiveBindingError::Heap(HeapError::ResourceBytes { staged: true, used, limit: ceiling })
                    if used == u64::try_from(staged)? && ceiling == u64::try_from(limit)?),
                    "{error:?}"
                );
                assert_eq!(crate::loan::fault::Injection::raw_copies(), 0);
                assert_eq!(crate::loan::fault::Injection::write_buffers(), [0; 2]);
                assert_eq!(crate::loan::fault::Injection::attempts(), 0);
            }
        }
        drop(injection);
    }
    Ok(())
}

/// Both formats refuse before the actual raw exporter and succeed at its complete staging boundary.
/// ´test:unit:loan-reading-write-raw-copy-byte-neighbours´
#[test]
fn loan_reading_write_raw_copy_byte_neighbours() -> TestResult {
    raw_copy_boundaries::<f32>()?;
    raw_copy_boundaries::<f64>()
}
