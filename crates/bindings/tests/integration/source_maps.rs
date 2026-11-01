// SPDX-License-Identifier: AGPL-3.0-only
//! Original source order survives repeated, signed and overlapping target selections.

use crate::testing::readings::{Fixture, TestResult, ledger, map, matrix, meaning, real};
use nrz_bindings::readings::{
    CoordinateFamily, PhysicalReading, ReadingSource, ReadingSourceMap, SourceError,
};
use nrz_cas::readings::{AffineMap, EntryIndex, SelectionSpec};

/// A single family supplies three declared sources and two overlapping target readings.
struct Case {
    /// Original exact graph and its declaration context.
    fixture: Fixture,
    /// Original scalar declarations in the deliberately permuted caller order.
    sources: Vec<ReadingSource>,
    /// Both overlapping signed target readings.
    targets: [PhysicalReading; 2],
}

/// Build the declared shared family and explicit paired maps.
fn case() -> TestResult<Case> {
    let mut fixture = Fixture::new(&[
        ("z", matrix(3, 1)),
        ("s0", real()),
        ("s1", real()),
        ("s2", real()),
        ("x", real()),
        ("y", real()),
        ("copy", real()),
        ("expanded", matrix(3, 1)),
    ])?;
    let entry = meaning("voltage", [2, 1, -3, -1, 0, 0, 0])?;
    let family = CoordinateFamily::new(
        &fixture.cas,
        &fixture.context,
        &fixture.symbol("z")?,
        vec![entry.clone(); 3],
        &mut ledger()?,
    )?;
    let mut sources = Vec::new();
    for (row, name) in [(1, "s1"), (2, "s2"), (0, "s0")] {
        let reading = fixture.define(
            &family,
            name,
            SelectionSpec::Entry(EntryIndex { row, column: 0 }),
            AffineMap::identity(1, &mut ledger()?)?,
            vec![entry.clone()],
        )?;
        sources.push(ReadingSource::new(reading, &mut ledger()?)?);
    }
    let first = fixture.define(
        &family,
        "x",
        SelectionSpec::Column(0),
        map(3, 1, &[-1, 0, 2], &[0])?,
        vec![entry.clone()],
    )?;
    let second = fixture.define(
        &family,
        "y",
        SelectionSpec::Column(0),
        map(3, 1, &[0, 1, -1], &[0])?,
        vec![entry],
    )?;
    Ok(Case {
        fixture,
        sources,
        targets: [first, second],
    })
}

/// Both maps expose the original caller order and preserve negative overlapping contributions.
/// ´test:integration:reading-source-map-keeps-both-targets-and-order´
#[test]
fn reading_source_map_keeps_both_targets_and_order() -> TestResult {
    let case = case()?;
    let exact = ReadingSourceMap::new(
        &case.fixture.cas,
        case.sources.clone(),
        case.targets.clone(),
        &mut ledger()?,
    )?;
    assert_eq!(exact.targets(), &case.targets);
    for (actual, expected) in exact.sources().iter().zip(&case.sources) {
        assert_eq!(actual.reading(), expected.reading());
    }
    assert_eq!(
        exact.maps(),
        &[map(3, 1, &[0, 2, -1], &[0])?, map(3, 1, &[1, -1, 0], &[0])?]
    );
    let reordered = vec![
        case.sources[2].clone(),
        case.sources[0].clone(),
        case.sources[1].clone(),
    ];
    let other = ReadingSourceMap::new(&case.fixture.cas, reordered, case.targets, &mut ledger()?)?;
    assert_eq!(
        other.maps(),
        &[map(3, 1, &[-1, 0, 2], &[0])?, map(3, 1, &[0, 1, -1], &[0])?]
    );
    Ok(())
}

/// Repeated and negated expansion copies compose once over their common original coordinate.
/// ´test:integration:reading-source-map-composes-repeated-negative-copies´
#[test]
fn reading_source_map_composes_repeated_negative_copies() -> TestResult {
    let mut case = case()?;
    let original = case.targets[0].family().clone();
    let symbol = case.fixture.symbol("expanded")?;
    let expanded = original.expand(
        &mut case.fixture.cas,
        &symbol,
        &map(3, 3, &[0, 0, 1, 0, 0, -1, 1, 0, 0], &[0, 0, 0])?,
        original.entries().to_vec(),
        &mut ledger()?,
    )?;
    let copied = case.fixture.define(
        &expanded,
        "copy",
        SelectionSpec::Column(0),
        map(3, 1, &[1, -1, -1], &[0])?,
        case.targets[0].entries().to_vec(),
    )?;
    let exact = ReadingSourceMap::new(
        &case.fixture.cas,
        case.sources,
        [case.targets[0].clone(), copied.clone()],
        &mut ledger()?,
    )?;
    assert_eq!(exact.maps()[0], exact.maps()[1]);
    assert_ne!(exact.targets()[0], exact.targets()[1]);
    assert_eq!(exact.targets()[1], copied);
    Ok(())
}

/// Missing and duplicate original coordinates cannot become fixed or independent directions.
/// ´test:integration:reading-source-map-refuses-incomplete-and-substituted-sources´
#[test]
fn reading_source_map_refuses_incomplete_and_substituted_sources() -> TestResult {
    let case = case()?;
    assert!(matches!(
        ReadingSourceMap::new(
            &case.fixture.cas,
            case.sources[..2].to_vec(),
            case.targets.clone(),
            &mut ledger()?,
        ),
        Err(SourceError::Missing)
    ));
    assert!(matches!(
        ReadingSourceMap::new(
            &case.fixture.cas,
            vec![case.sources[0].clone(); 3],
            case.targets.clone(),
            &mut ledger()?,
        ),
        Err(SourceError::Conflict)
    ));
    let foreign = crate::testing::readings::Fixture::new(&[("z", matrix(3, 1))])?;
    assert!(
        ReadingSourceMap::new(&foreign.cas, case.sources, case.targets, &mut ledger()?,).is_err()
    );
    Ok(())
}

/// Literal owner ceilings distinguish complete success from one-below and consumed-prefix refusal.
/// ´test:integration:reading-source-map-literal-owner-boundaries´
#[test]
fn reading_source_map_literal_owner_boundaries() -> TestResult {
    let case = case()?;
    let mut budget = ledger()?;
    let exact = ReadingSourceMap::new(
        &case.fixture.cas,
        case.sources.clone(),
        case.targets.clone(),
        &mut budget,
    )?;
    assert_eq!(
        [budget.usage().work, exact.retained_payload_bytes()],
        [44_158, 3_520]
    );
    for (kind, required) in [(0, 44_158), (1, 5_848), (2, 3_520)] {
        for (available, prior, success) in [
            (required, 0, true),
            (required - 1, 0, false),
            (required, 1, kind != 0),
        ] {
            let mut limits = nrz_bindings::Limits::default();
            match kind {
                0 => limits.work = available,
                1 => limits.staged_payload_bytes = available,
                _ => limits.output_bytes = available,
            }
            let mut budget = nrz_bindings::Budget::new(limits)?;
            budget.work(prior, nrz_cas::Location::new("earlier source composition"))?;
            let result = ReadingSourceMap::new(
                &case.fixture.cas,
                case.sources.clone(),
                case.targets.clone(),
                &mut budget,
            );
            assert_eq!(
                result.is_ok(),
                success,
                "kind {kind}, limit {available}, prior {prior}: {result:?}"
            );
            assert!(budget.usage().work >= prior);
        }
    }
    Ok(())
}

/// Equal magnitudes do not replace the original physical reference definition of a source family.
/// ´test:integration:reading-source-map-refuses-equal-magnitude-reference-substitution´
#[test]
fn reading_source_map_refuses_equal_magnitude_reference_substitution() -> TestResult {
    let mut case = case()?;
    let original = case.targets[0].family().clone();
    let descriptor = original.entries()[0].descriptor().clone();
    let reference = nrz_bindings::ReferenceDefinition::unit("different-reference", &descriptor)?;
    let meaning =
        nrz_bindings::readings::EntryMeaning::new(descriptor, reference, None, &mut ledger()?)?;
    assert_eq!(meaning.magnitude(), original.entries()[0].magnitude());
    let family = CoordinateFamily::new(
        &case.fixture.cas,
        &case.fixture.context,
        original.root(),
        vec![meaning.clone(); 3],
        &mut ledger()?,
    )?;
    let changed = case.fixture.define(
        &family,
        "copy",
        SelectionSpec::Column(0),
        map(3, 1, &[-1, 0, 2], &[0])?,
        vec![meaning],
    )?;
    assert!(matches!(
        ReadingSourceMap::new(
            &case.fixture.cas,
            case.sources,
            [changed, case.targets[1].clone()],
            &mut ledger()?
        ),
        Err(SourceError::Conflict)
    ));
    Ok(())
}
