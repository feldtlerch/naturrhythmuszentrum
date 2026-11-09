// SPDX-License-Identifier: AGPL-3.0-only
//! Complete residual operations retain buffers and prior ledger spend at every boundary.

use crate::testing::common::{TestResult, default_budget};
use crate::testing::series_residual::{Fixture, Term, binary, member, rational};
use nrz_cas::limits::{Budget, BudgetKind};
use nrz_cas::syntax::Operator;
use nrz_cas::{Exhaustion, FaultKind, InvalidCode, Location};

/// Independent resource ceilings include solving dimensions and exponent preflight.
const KINDS: [BudgetKind; 18] = [
    BudgetKind::Work,
    BudgetKind::StagedPayloadBytes,
    BudgetKind::PayloadBytes,
    BudgetKind::WitnessBytes,
    BudgetKind::OutputBytes,
    BudgetKind::Depth,
    BudgetKind::ArithmeticSteps,
    BudgetKind::ConditionSteps,
    BudgetKind::ComponentBits,
    BudgetKind::JetOrder,
    BudgetKind::JetEntries,
    BudgetKind::ProofNodes,
    BudgetKind::ReplayWork,
    BudgetKind::IntermediateBits,
    BudgetKind::SolveIdentities,
    BudgetKind::LinearPivots,
    BudgetKind::MatrixEntries,
    BudgetKind::IntegerExponent,
];

/// Every attempt fixes the graph setup, allocation order, retained buffers and unrelated ceilings.
#[expect(
    clippy::too_many_lines,
    reason = "each complete attempt fixes setup, lifetime and rollback in one place"
)]
fn attempt(
    mode: usize,
    reversed: bool,
    dimension: BudgetKind,
    amount: u64,
) -> TestResult<(bool, u64)> {
    let mut limits = crate::testing::half_pi_limits::ceiling(dimension, amount);
    match dimension {
        BudgetKind::JetOrder => limits.jet_order = amount,
        BudgetKind::JetEntries => limits.jet_entries = amount,
        BudgetKind::SolveIdentities => limits.solve_identities = amount,
        BudgetKind::LinearPivots => limits.linear_pivots = amount,
        BudgetKind::MatrixEntries => limits.matrix_entries = amount,
        BudgetKind::IntegerExponent => limits.integer_exponent = amount,
        _ => {}
    }
    if matches!(
        dimension,
        BudgetKind::ComponentBits | BudgetKind::IntermediateBits
    ) && amount == 0
    {
        assert_eq!(
            Budget::new(limits).expect_err("zero bit ceiling").kind,
            FaultKind::Invalid(InvalidCode::Policy)
        );
        return Ok((false, 0));
    }
    let mut f = Fixture::new(&["1", "0"], "0", reversed, false)?;
    let subject = f.compose()?;
    let term = binary(
        Operator::Sub,
        Term::Power(Box::new(Term::Y(0)), 2),
        Term::Q(if mode == 2 { "2" } else { "1" }),
    );
    let (expression, rows) = f.term(&term, 2)?;
    let solving = if mode == 2 {
        f.linear(&[rational("-1")?, rational("0")?])?
    } else {
        member()
    };
    let source_bytes = f.cas.export_witness(&subject, &mut default_budget()?)?;
    let mut retained = 2 * u64::try_from(source_bytes.len())? + 113;
    let saved = if mode >= 3 {
        Some(f.cas.check_evaluated_residual(
            &subject,
            &expression,
            &f.y,
            &rows,
            if mode == 5 { None } else { Some(&solving) },
            &mut default_budget()?,
        )?)
    } else {
        None
    };
    let mut receiver = Fixture::new(&["1", "0"], "0", !reversed, false)?;
    let (expected_residual, expected_solve) =
        receiver.expectations(&term, &[rational("0")?, rational("0")?], 0)?;
    let mut bytes = Vec::new();
    if let Some((residual, solve)) = &saved {
        let witness = if mode == 5 {
            residual
        } else {
            solve.as_ref().ok_or("saved Member")?
        };
        bytes = f.cas.export_witness(witness, &mut default_budget()?)?;
        retained += u64::try_from(
            bytes.len()
                + residual.exported_byte_len()
                + solve
                    .as_ref()
                    .map_or(0, nrz_cas::proof::VerifiedWitness::exported_byte_len),
        )?;
    }
    let before = if mode >= 5 {
        receiver.cas.canonical_snapshot(&mut default_budget()?)?
    } else {
        f.cas.canonical_snapshot(&mut default_budget()?)?
    };
    let export_witness = saved.as_ref().and_then(|(_, solve)| solve.as_ref());
    let mut budget = Budget::new(limits)?;
    budget.work(37, Location::new("residual prior spend"))?;
    let result = budget.with_retained_payload(
        retained,
        Location::new("residual caller buffers"),
        |budget| match mode {
            0..=3 => f
                .cas
                .check_evaluated_residual(
                    &subject,
                    &expression,
                    &f.y,
                    &rows,
                    if mode == 0 { None } else { Some(&solving) },
                    budget,
                )
                .map(|_| ()),
            4 => f
                .cas
                .export_witness(
                    export_witness.ok_or_else(|| {
                        nrz_cas::Fault::invalid(
                            nrz_cas::InvalidCode::Rule,
                            Location::new("missing fixture witness"),
                        )
                    })?,
                    budget,
                )
                .map(|_| ()),
            5 => receiver
                .cas
                .replay(&bytes, &expected_residual, budget)
                .map(|_| ()),
            _ => receiver
                .cas
                .replay_evaluated_residual(&bytes, &expected_residual, &expected_solve, budget)
                .map(|_| ()),
        },
    );
    if let Err(fault) = &result {
        // Nested staging excludes retained outer buffers, as in the composition controls.
        let needed = if dimension == BudgetKind::StagedPayloadBytes {
            match mode {
                2 => 55_839,
                4 => 2088,
                _ => 55_836,
            }
        } else {
            amount + 1
        };
        assert_eq!(
            fault.kind,
            FaultKind::Exhausted {
                reason: if matches!(
                    dimension,
                    BudgetKind::ComponentBits | BudgetKind::IntermediateBits
                ) {
                    Exhaustion::ArithmeticRange
                } else {
                    Exhaustion::Limit
                },
                budget: dimension,
                limit: needed - 1,
                needed,
            },
            "{mode} {dimension:?} {amount}: {fault:?}"
        );
        let after = if mode >= 5 {
            receiver.cas.canonical_snapshot(&mut default_budget()?)?
        } else {
            f.cas.canonical_snapshot(&mut default_budget()?)?
        };
        assert_eq!(before, after);
        assert_eq!(
            source_bytes,
            f.cas.export_witness(&subject, &mut default_budget()?)?
        );
        assert!(f.cas.expression(&f.y).is_ok());
    }
    Ok((result.is_ok(), budget.usage().work))
}

/// Measured complete-operation minima, in KINDS order. Work includes the prior 37 units.
const CEILINGS: [[u64; 18]; 7] = [
    [
        184_947, 80_257, 14_411, 1_931, 3_641, 3, 783, 2, 1, 1, 30, 2, 0, 2, 0, 0, 0, 2,
    ],
    [
        322_511, 80_321, 15_942, 2_088, 3_641, 3, 1_160, 4, 1, 1, 30, 3, 0, 2, 1, 0, 0, 2,
    ],
    [
        346_289, 80_360, 17_704, 2_311, 3_644, 3, 1_502, 4, 2, 1, 30, 3, 0, 3, 1, 1, 2, 2,
    ],
    [
        322_409, 86_684, 533, 2_088, 5_240, 3, 1_160, 4, 1, 1, 30, 3, 0, 2, 1, 0, 0, 2,
    ],
    [
        2_125, 10_544, 0, 2_088, 2_088, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0,
    ],
    [
        356_678, 65_139, 14_812, 1_931, 3_641, 3, 1_878, 3, 1, 1, 64, 2, 356_641, 2, 0, 0, 0, 2,
    ],
    [
        735_603, 67_384, 15_942, 2_088, 3_641, 3, 2_588, 5, 1, 1, 64, 3, 735_566, 2, 1, 0, 0, 2,
    ],
];

/// Work consumed when the corresponding ceiling is one less, including the prior spend.
const PREFIXES: [[u64; 18]; 7] = [
    [
        184_943, 47_142, 59_698, 184_943, 58_992, 296, 159_446, 56_186, 0, 293, 55_961, 59_698, 0,
        47_868, 0, 0, 0, 295,
    ],
    [
        322_507, 47_206, 192_302, 322_507, 59_056, 296, 301_310, 191_956, 0, 293, 56_025, 192_302,
        0, 47_932, 190_940, 0, 0, 295,
    ],
    [
        346_285, 47_623, 208_702, 346_285, 59_477, 296, 334_840, 208_264, 297, 293, 56_442,
        208_702, 0, 55_095, 199_054, 201_632, 193_060, 295,
    ],
    [
        322_405, 47_104, 1_836, 322_405, 58_954, 296, 301_208, 191_854, 0, 293, 55_923, 59_660, 0,
        47_830, 190_838, 0, 0, 295,
    ],
    [37, 37, 0, 37, 37, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [
        356_579, 212_122, 224_678, 775, 223_972, 41, 330_975, 221_166, 0, 38, 220_941, 787,
        356_579, 89_348, 0, 0, 0, 40,
    ],
    [
        735_599, 218_567, 371_947, 835, 230_417, 41, 723_481, 371_601, 0, 38, 227_386, 847,
        735_599, 95_243, 370_585, 0, 0, 40,
    ],
];

/// Construction, Member, contradiction, retained hit, export, fresh and paired replay boundaries.
/// ´test:integration:residual-resource-measurements´
#[test]
fn residual_resource_measurements() -> TestResult {
    for (mode, ceilings) in CEILINGS.iter().enumerate() {
        for (index, kind) in KINDS.into_iter().enumerate() {
            let ceiling = ceilings[index];
            for reversed in [false, true] {
                assert!(
                    attempt(mode, reversed, kind, ceiling)?.0,
                    "mode {mode} {kind:?}"
                );
                if ceiling > 0 {
                    assert_eq!(
                        attempt(mode, reversed, kind, ceiling - 1)?,
                        (false, PREFIXES[mode][index]),
                        "mode {mode} {kind:?}"
                    );
                }
            }
        }
    }
    Ok(())
}

/// Growth varies exactly one original syntax dimension, keeping unrelated ceilings fixed.
fn growth(kind: usize, level: usize, reversed: bool, work: u64) -> TestResult<(bool, u64, u64)> {
    let n = if kind == 0 { level + 1 } else { 5 };
    let mut f = Fixture::new(&vec!["1"; n], "0", reversed, false)?;
    let subject = f.compose()?;
    let mut term = Term::Y(if kind == 1 { u32::try_from(level)? } else { 0 });
    if kind == 2 {
        for _ in 0..level {
            term = binary(Operator::Add, term, Term::Y(0));
        }
    }
    if kind == 3 {
        term = Term::Power(Box::new(term), u32::try_from(level)?);
    }
    let order = n - if kind == 1 { level } else { 0 };
    let (expression, rows) = f.term(&term, order)?;
    let before = f.cas.canonical_snapshot(&mut default_budget()?)?;
    let old_bytes = f.cas.export_witness(&subject, &mut default_budget()?)?;
    let mut budget = Budget::new(nrz_cas::limits::Limits {
        work,
        ..nrz_cas::limits::Limits::default()
    })?;
    let result =
        f.cas
            .check_evaluated_residual(&subject, &expression, &f.y, &rows, None, &mut budget);
    if let Err(fault) = &result {
        assert!(matches!(
            fault.kind,
            FaultKind::Exhausted {
                budget: BudgetKind::Work,
                ..
            }
        ));
        assert_eq!(before, f.cas.canonical_snapshot(&mut default_budget()?)?);
        assert_eq!(
            old_bytes,
            f.cas.export_witness(&subject, &mut default_budget()?)?
        );
        assert!(f.cas.expression(&f.y).is_ok());
    }
    Ok((
        result.is_ok(),
        budget.usage().work,
        budget.usage().jet_entries,
    ))
}

/// Original order, derivative depth, repeated occurrences and exponent each change complete costs.
/// ´test:integration:residual-resource-growth´
#[test]
fn residual_resource_growth() -> TestResult {
    let measured = [
        [(91_038, 5), (140_334, 10), (239_151, 15), (409_150, 20)],
        [(669_795, 25), (693_062, 32), (696_235, 33), (690_663, 28)],
        [(669_795, 25), (710_233, 50), (745_832, 75), (781_512, 100)],
        [(691_869, 35), (715_710, 45), (760_439, 55), (789_016, 65)],
    ];
    for (kind, row) in measured.into_iter().enumerate() {
        for (level, (work, entries)) in row.into_iter().enumerate() {
            for reversed in [false, true] {
                assert_eq!(growth(kind, level, reversed, work)?, (true, work, entries));
                assert!(!growth(kind, level, reversed, work - 1)?.0);
            }
        }
    }
    Ok(())
}
