// SPDX-License-Identifier: AGPL-3.0-only
//! Statistical admission contains capture, contraction and complete replacement publication.

#[path = "statistical_addition.rs"]
mod addition;
#[path = "statistical_affine.rs"]
mod affine;
#[path = "statistical_cross.rs"]
mod cross;

use super::TestResult;
use crate::{
    Binding, CheckedOperation, ExecutionAccount, ExecutionLimits, Extent, Handle, Heap, HeapError,
    HeapLimits, OperationError, Value,
};
use nrz_bindings::{Budget, Descriptor, Limits};
use nrz_scalar::{Enclosure, Scalar};

/// Four independent source coordinates and one dense model term, with optional spare backing.
struct Case<P: Scalar> {
    /// Original owner with prepared spare descriptor, arena and execution tables.
    heap: Heap,
    /// Represented vector carrying the complete source relation.
    input: Handle<Value, P>,
    /// Exact four-entry copy law.
    operation: CheckedOperation,
}
impl<P: Scalar + 'static> Case<P> {
    /// Set up the review's physical boundary without charging unrelated resident bytes.
    fn new(spare: bool) -> TestResult<Self> {
        let mut heap = Heap::new(HeapLimits::default())?;
        let mut warm = Vec::new();
        for _ in 0..if spare { 32 } else { 0 } {
            warm.push(
                heap.import_represented(Extent::new(1, 1)?, &Descriptor::ratio(), &[P::ONE])?
                    .value(),
            );
        }
        for handle in warm {
            heap.retire(handle)?;
        }
        let input = heap
            .import_represented(Extent::new(4, 1)?, &Descriptor::ratio(), &[P::ONE; 4])?
            .value();
        let operation = crate::binding_testing::operation(4, 1)?;
        drop(
            Binding::new(operation.clone(), vec![input], ExecutionLimits::default())
                .validate(&mut heap)?,
        );
        let limits = crate::JointLimits::default();
        let sources = (1_u32..=4)
            .map(|index| crate::Source::new(&Descriptor::ratio(), u64::from(index)))
            .collect::<Result<Vec<_>, _>>()?;
        let covariance = crate::DenseCovariance::new(
            4,
            &[
                P::ONE,
                P::ZERO,
                P::ZERO,
                P::ZERO,
                P::ZERO,
                P::ONE,
                P::ZERO,
                P::ZERO,
                P::ZERO,
                P::ZERO,
                P::ONE,
                P::ZERO,
                P::ZERO,
                P::ZERO,
                P::ZERO,
                P::ONE,
            ],
            &nrz_scalar::ArithmeticContext::new()?,
            limits,
        )?;
        let model = crate::JointModel::from_dense(
            &sources,
            covariance,
            &std::array::from_fn::<_, 16, _>(|_| crate::CovarianceBlock::Known),
            limits,
        )?;
        let target = crate::TargetMap::capture(&heap, input)?;
        heap.publish_joint(
            crate::CompanionSetDraft::new(
                model,
                [target.clone(), target],
                [vec![0, 1, 2, 3], vec![0, 1, 2, 3]],
            ),
            limits,
        )?;
        // The joint declaration publishes separate dense covariance center and error regions.
        // Together with the source value these occupy three descriptor and arena slots.
        assert_eq!(heap.descriptors.iter().count(), 3);
        assert_eq!(
            heap.storage.elements()[crate::storage::Format::of::<P>().index()],
            4 + 2 * 16
        );
        Ok(Self {
            heap,
            input,
            operation,
        })
    }
    /// Fresh proposed execution over the same exact model and vector contract.
    fn binding(&self) -> Binding<P> {
        Binding::new(
            self.operation.clone(),
            vec![self.input; self.operation.inputs().len()],
            ExecutionLimits::default(),
        )
    }
}

/// The 2,240-work phase's 17,920-byte backend allowance alone exceeds 16,384 staged bytes.
/// ´test:unit:loan-statistical-account-refuses-before-numerical-capture´
#[test]
fn loan_statistical_account_refuses_before_numerical_capture() -> TestResult {
    for staged in [16_383, 16_384, 16_385] {
        let mut case = Case::<f64>::new(true)?;
        let mut account = ExecutionAccount::new(Budget::new(Limits {
            staged_payload_bytes: staged,
            ..Limits::default()
        })?)?;
        let injection = crate::loan::fault::Injection::after(0);
        let result = case
            .binding()
            .validate_with_account(&mut case.heap, &mut account);
        assert!(
            matches!(result, Err(OperationError::Heap(HeapError::ResourceBytes { staged: true, used, limit })) if used > 17_920 && limit == staged)
        );
        assert_eq!(crate::loan::fault::Injection::attempts(), 0);
        drop(injection);
    }
    Ok(())
}

/// The fixture owns ten center/enclosure pairs, three backend contractions and six parameter copies.
/// Each contraction packs three 4-by-4 matrices with 64-byte rows, in either precision.
fn expected_bytes<P: Scalar>(spare: bool) -> [usize; 2] {
    use crate::heap::{descriptors::DescriptorRecord, identity::Key};
    let width = size_of::<P>();
    let metadata = crate::statistics::inventory::copy_fixture_metadata_bytes();
    let paired_entries = 16 + 4 + 16 + 16 + 16 + 4 + 16 + 16 + 16 + 16;
    let vectors = paired_entries * (width + size_of::<Enclosure<P>>()) + 6 * 16 * width;
    let packed = 3 * 3 * 4 * 64;
    let backend = 2240 * width;
    // One four-scalar ordinary destination and six sixteen-scalar parameter destinations.
    let slabs = 64 + 6 * (16 * width).div_ceil(64) * 64;
    // Six execution fields (identity plus five counters), and both reference-count words.
    let trace = size_of::<crate::ExecutionId>() + 7 * size_of::<usize>();
    let mut retained = slabs + metadata + trace;
    // Original one-slab reservation survives until the seven-slab replacement is assigned.
    // Both Mat owner vectors and both descriptor/arena key sets coexist with the candidate.
    let mut scratch = 64
        + header_bytes::<P>(1)
        + (1 + 7) * size_of::<crate::heap::records::Record>()
        + size_of::<crate::publication::Payload<P>>()
        + (1 + 7) * size_of::<faer::Mat<P>>()
        + (1 + 7) * 2 * size_of::<Key>()
        + 4 * width
        + 8 * width
        + vectors
        + packed
        + backend
        + trace;
    if !spare {
        // The input and two model regions occupy three slots in capacity four. One ordinary
        // destination fits; the seven-destination replacement needs ten slots, beyond doubling.
        // Resident model payloads stay excluded, but their occupations determine this growth.
        let slots =
            size_of::<(u64, Option<DescriptorRecord>)>() + size_of::<(u64, Option<faer::Mat<P>>)>();
        retained += (3 + 7 - 4) * slots;
        scratch += 2 * 4 * slots;
    }
    [retained, retained + scratch]
}

/// Captured inputs, operating points, one moment and six center/error region headers coexist.
fn header_bytes<P: Scalar>(inputs: usize) -> usize {
    let [input, point, moment, region] = crate::statistics::fixture_header_sizes::<P>();
    inputs * (input + point) + moment + 6 * region
}

/// Independent N and N-1 ceilings cover both spare tables and a four-to-ten slot replacement.
fn byte_neighbours<P: Scalar + 'static>() -> TestResult {
    for spare in [true, false] {
        let boundaries = expected_bytes::<P>(spare);
        for (staged, needed) in [false, true].into_iter().zip(boundaries) {
            for (limit, succeeds) in [(needed, true), (needed - 1, false)] {
                let mut case = Case::<P>::new(spare)?;
                let mut limits = Limits::default();
                if staged {
                    limits.staged_payload_bytes = u64::try_from(limit)?;
                } else {
                    limits.output_bytes = u64::try_from(limit)?;
                }
                let mut account = ExecutionAccount::new(Budget::new(limits)?)?;
                let injection =
                    crate::loan::fault::Injection::after(if succeeds { usize::MAX } else { 0 });
                let result = case
                    .binding()
                    .validate_with_account(&mut case.heap, &mut account);
                match result {
                    Ok(bound) => {
                        assert!(succeeds);
                        assert_eq!(bound.statistical_work(), 2240);
                        assert_eq!(bound.resource_permit.retained, boundaries[0]);
                        assert_eq!(bound.resource_permit.peak, boundaries[1]);
                        let candidate = bound.evaluate(bound.normalized_inputs()?)?;
                        let outcome = bound.publish(candidate)?;
                        assert_eq!(outcome.output().len(), 1);
                        assert_eq!(
                            crate::loan::fault::Injection::statistical_headers(),
                            [4, header_bytes::<P>(1)]
                        );
                    }
                    Err(error) => {
                        assert!(
                            !succeeds,
                            "{error:?}; precision {:?}, spare {spare}",
                            P::PRECISION
                        );
                        assert!(matches!(error,
                            OperationError::Heap(HeapError::ResourceBytes { staged: actual, used, limit: ceiling })
                            if actual == staged && used == u64::try_from(needed)?
                                && ceiling == u64::try_from(limit)?
                        ));
                        assert_eq!(crate::loan::fault::Injection::attempts(), 0);
                        assert_eq!(crate::loan::fault::Injection::statistical_headers(), [0; 2]);
                    }
                }
                drop(injection);
            }
        }
    }
    Ok(())
}

/// Exact full-plan ceilings pass publication; one fewer byte refuses before numerical capture.
/// ´test:unit:loan-statistical-physical-plan-byte-neighbours´
#[test]
fn loan_statistical_physical_plan_byte_neighbours() -> TestResult {
    // Matrix is Extent plus two Buffer headers. Input adds one lineage pointer;
    // Moment adds a 112-byte lineage and derivative pointer to four Matrix headers.
    // RegionData adds a 736-byte optional descriptor and symbol pointer to Extent/Buffer.
    assert_eq!(
        crate::statistics::fixture_header_sizes::<f32>(),
        [280, 136, 664, 824]
    );
    assert_eq!(
        crate::statistics::fixture_header_sizes::<f64>(),
        [280, 136, 664, 824]
    );
    byte_neighbours::<f32>()?;
    byte_neighbours::<f64>()
}

/// Compatibility metadata ceilings remain independent of their default physical policy.
/// ´test:unit:loan-statistical-compatibility-keeps-metadata-scope´
#[test]
fn loan_statistical_compatibility_keeps_metadata_scope() -> TestResult {
    let mut case = Case::<f64>::new(true)?;
    let budget = Budget::new(Limits {
        staged_payload_bytes: 16_384,
        output_bytes: 16_384,
        ..Limits::default()
    })?;
    let bound = case
        .binding()
        .validate_with_resource_budget(&mut case.heap, &budget)?;
    assert_eq!(bound.statistical_work(), 2240);
    let candidate = bound.evaluate(bound.normalized_inputs()?)?;
    bound.publish(candidate)?;
    Ok(())
}

/// Account admission checks the complete final destination set before numerical capture.
/// ´test:unit:loan-statistical-account-checks-complete-destination-set´
#[test]
fn loan_statistical_account_checks_complete_destination_set() -> TestResult {
    for succeeds in [false, true] {
        let mut case = Case::<f64>::new(true)?;
        // Four copied coordinates and three 4-by-4 parameter center/error pairs.
        let required = case.heap.storage.elements()[1] + 4 + 3 * 2 * 16;
        case.heap.limits = HeapLimits::new(4096, required - usize::from(!succeeds), 4096);
        let mut account = ExecutionAccount::new(Budget::new(Limits::default())?)?;
        let injection = crate::loan::fault::Injection::after(if succeeds { usize::MAX } else { 0 });
        let result = case
            .binding()
            .validate_with_account(&mut case.heap, &mut account);
        match result {
            Ok(bound) => {
                assert!(succeeds);
                let candidate = bound.evaluate(bound.normalized_inputs()?)?;
                bound.publish(candidate)?;
            }
            Err(error) => {
                assert!(!succeeds);
                assert!(matches!(error, OperationError::Heap(HeapError::Capacity)));
                assert_eq!(crate::loan::fault::Injection::attempts(), 0);
            }
        }
        drop(injection);
    }
    Ok(())
}

/// Escaped statistical candidates retain both physical outer backings until their destruction.
/// ´test:unit:loan-residence-retains-escaped-statistical-candidate´
#[test]
fn loan_residence_retains_escaped_statistical_candidate() -> TestResult {
    statistical_residence::<f32>()?;
    statistical_residence::<f64>()
}

/// Drop, publication and refusal all release each physical candidate owner exactly once.
fn statistical_residence<P: Scalar + 'static>() -> TestResult {
    for ending in 0..3 {
        let mut case = Case::<P>::new(true)?;
        let bound = case.binding().validate(&mut case.heap)?;
        let trace = bound.resources.trace.downgrade();
        let mut candidate = bound.evaluate(bound.normalized_inputs()?)?;
        assert_eq!(candidate.outputs.capacity(), 1);
        assert_eq!(candidate.statistics.capacity(), 1);
        if ending == 0 {
            drop(bound);
            case.heap.retire(case.input)?;
            let elements = case.heap.storage.elements();
            let mut account = ExecutionAccount::new(Budget::new(Limits::default())?)?;
            assert!(matches!(case.heap.residence(&mut account, 10_000)?,
                crate::Residence::Pending { elements: Some(actual), .. } if actual == elements));
            let live = case.heap.resource_bytes()?;
            drop(candidate);
            // Four ordinary scalars plus three 4x4 center/enclosure pairs, and one of
            // each outer header: Payload 280 and MomentPayload 664 bytes.
            let numerical = (4 + 3 * 16 * 3) * size_of::<P>();
            assert_eq!(live - case.heap.resource_bytes()?, numerical + 280 + 664);
            assert_eq!(case.heap.storage.elements(), elements);
        } else if ending == 1 {
            drop(bound.publish(candidate)?);
        } else {
            // Clearing entries destroys nested arrays but retains the outer capacity/debit.
            candidate.statistics.clear();
            assert_eq!(candidate.statistics.capacity(), 1);
            let failure = bound
                .publish(candidate)
                .err()
                .ok_or("incomplete moments published")?;
            assert!(matches!(
                failure.cause(),
                OperationError::Statistics(crate::StatisticsError::Contract)
            ));
        }
        assert_eq!(
            trace.owned_bytes()?,
            crate::execution::Trace::metadata_bytes()
        );
    }
    Ok(())
}
