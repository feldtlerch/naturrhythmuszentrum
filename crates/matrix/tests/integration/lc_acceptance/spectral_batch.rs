// SPDX-License-Identifier: AGPL-3.0-only
//! One original exact schema owns bounded autonomous schedules for several actual LC instances.

mod boundaries;
mod lifecycle;
mod limits;

use crate::reading_testing::{TestResult, ledger};
use crate::reciprocal_testing::oracle::Rational;
use crate::spectral_testing::Fixture;
use nrz_bindings::Budget;
use nrz_matrix::{
    Heap, LcSpectralBatch, LcSpectralBatchLimits, LcSpectralSchema, LcSpectralStatus, LcSpectrum,
    ReadingBinding,
};
use nrz_scalar::{ArithmeticContext, LiteralLimits, Scalar};

/// Actual floating inputs differ; the common checked theorem and exact exports do not.
/// ´test:integration:lc-spectral-batch-rebinds-one-schema´
#[test]
fn lc_spectral_batch_rebinds_one_schema() -> TestResult {
    let fixture = Fixture::new(["5", "1/10", "1/2000"])?;
    let schema = schema(&fixture)?;
    successful::<f32>(&fixture, &schema)?;
    successful::<f64>(&fixture, &schema)
}

/// The same exact schema crosses evaluation scopes, while numerical keys and results cannot.
fn successful<P: Scalar + 'static>(fixture: &Fixture, schema: &LcSpectralSchema) -> TestResult {
    let context = ArithmeticContext::new()?;
    for exponent in [0, 1] {
        let case = case::<P>(fixture, schema, exponent)?;
        let mut budget = ledger()?;
        let mut batch = prepare(schema, &case, &context, &mut budget)?;
        assert_eq!(batch.point_keys().collect::<Vec<_>>(), [41, 9, 73, 5]);
        assert_eq!(batch.bindings(), 2);
        assert_eq!(batch.binding_index(0), batch.binding_index(2));
        assert_eq!(batch.binding_index(0), batch.binding_index(1));
        let before = budget.usage();
        batch.execute(&case.heap, &context, &mut budget)?;
        assert_eq!(batch.usage(), Some([before, budget.usage()]));
        for (row, expected) in [8 >> exponent, 8 >> exponent, 8 >> exponent, 4 >> exponent]
            .into_iter()
            .enumerate()
        {
            assert_eq!(batch.status(row), Some(LcSpectralStatus::Successful));
            let value = batch.result(row, &case.heap, &context, &mut budget)?;
            let roots = value.imaginary_rates(&case.heap, &context)?;
            for (root, sign) in roots.into_iter().zip([1, -1]) {
                assert!(
                    Rational::new(sign * expected, 1)?
                        .covered(root.center.to_f64(), root.total.to_f64())?
                );
            }
        }
        assert!(std::ptr::eq(
            batch.result(0, &case.heap, &context, &mut budget)?,
            batch.result(2, &case.heap, &context, &mut budget)?,
        ));
        assert_eq!(
            batch
                .result(2, &case.heap, &context, &mut budget)?
                .plan()
                .usage(),
            case.plans[0].usage()
        );
        assert!(batch.execute(&case.heap, &context, &mut budget).is_err());
        assert_eq!(batch.status(0), Some(LcSpectralStatus::Successful));
    }
    Ok(())
}

/// A pair of valid actual instances shares the original declarations but not numerical identity.
struct Case<P: Scalar> {
    heap: Heap,
    inputs: [[ReadingBinding<P>; 2]; 2],
    plans: [LcSpectrum<P>; 3],
}

fn schema(fixture: &Fixture) -> TestResult<LcSpectralSchema> {
    Ok(LcSpectralSchema::new(
        &fixture.graph,
        fixture.law.clone(),
        LiteralLimits::default(),
        &mut ledger()?,
    )?)
}

fn case<P: Scalar + 'static>(
    fixture: &Fixture,
    schema: &LcSpectralSchema,
    exponent: i32,
) -> TestResult<Case<P>> {
    let quarter = if exponent == 0 {
        P::ONE / P::from_u8(4)
    } else {
        P::ONE
    };
    let sixteenth = P::ONE / P::from_u8(16);
    let (mut heap, first) =
        fixture.import([[quarter, P::ZERO, P::ZERO], [sixteenth, P::ZERO, P::ZERO]])?;
    let second = fixture.import_into(
        &mut heap,
        [
            [quarter * P::from_u8(4), P::ZERO, P::ZERO],
            [sixteenth, P::ZERO, P::ZERO],
        ],
        &mut ledger()?,
    )?;
    let plans = [
        schema.bind(&heap, first.clone(), &mut ledger()?)?,
        schema.bind(&heap, first.clone(), &mut ledger()?)?,
        schema.bind(&heap, second.clone(), &mut ledger()?)?,
    ];
    Ok(Case {
        heap,
        inputs: [first, second],
        plans,
    })
}

const fn ceilings() -> LcSpectralBatchLimits {
    LcSpectralBatchLimits {
        points: 4,
        bindings: 2,
        key_bytes: 4096,
    }
}

fn points<P: Scalar>(case: &Case<P>) -> [(u64, LcSpectrum<P>); 4] {
    [
        (41, case.plans[0].clone()),
        (9, case.plans[0].clone()),
        (73, case.plans[1].clone()),
        (5, case.plans[2].clone()),
    ]
}

fn prepare<P: Scalar + 'static>(
    schema: &LcSpectralSchema,
    case: &Case<P>,
    context: &ArithmeticContext,
    budget: &mut Budget,
) -> TestResult<LcSpectralBatch<P>> {
    Ok(LcSpectralBatch::prepare(
        schema,
        &case.heap,
        &points(case),
        ceilings(),
        context,
        budget,
    )?)
}
