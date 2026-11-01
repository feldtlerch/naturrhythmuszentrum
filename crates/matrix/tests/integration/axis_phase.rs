// SPDX-License-Identifier: AGPL-3.0-only
//! Closed LC sign consumers retain original proof, certified pi and complete native versions.

#[path = "axis_phase/costs.rs"]
mod costs;
#[path = "../../../bindings/src/testing/axis_phase.rs"]
mod fixture;
#[path = "axis_phase/lifecycle.rs"]
mod lifecycle;
#[path = "axis_phase/phasor.rs"]
mod phasor;
use crate::kernel_oracle::Rational;
use fixture::{Fixture, TestResult, enclosure, fixture, ledger};
use nrz_bindings::{AxisPhaseOutcome, Budget};
use nrz_matrix::{
    AxisPhasePlan, Coverage, Heap, HeapLimits, LiveDecisionError, PreparedAxisPhase,
    PublishedAxisPhase, ValueBinding,
};
use nrz_scalar::{ArithmeticContext, Enclosure, Finite, LiteralLimits, Scalar};

/// Complete original native source, immutable case and one certified constant owner.
struct Case<P: Scalar> {
    /// Exact independent physical and CAS declarations.
    source: Fixture,
    /// Actual numerical owner.
    heap: Heap,
    /// Predeclared native normalized-reactance input.
    input: ValueBinding<P>,
    /// Original exact branches and shared certified pi owner.
    plan: AxisPhasePlan<P>,
    /// Admitted arithmetic environment.
    context: ArithmeticContext,
}

/// Exact test values are dyadic; no approximate parser supplies the circuit oracle.
fn scalar<P: Scalar>(text: &str) -> TestResult<P> {
    P::parse_nearest(text).ok_or_else(|| "fixture scalar".into())
}

impl<P: Scalar + 'static> Case<P> {
    /// Independently declare/import the original normalized-reactance source before attempting signs.
    fn new(frequency: i64, center: &str, radius: &str) -> TestResult<Self> {
        let source = fixture(frequency)?;
        let mut heap = Heap::new(HeapLimits::default())?;
        let input = heap.import_symbols(
            source.case.symbols(),
            &[scalar(center)?],
            Some(&[scalar(radius)?]),
            Some(&[P::ZERO]),
            Coverage::DeclaredBounds,
        )?;
        let context = ArithmeticContext::new()?;
        let pi = super::pi_conversion::pi::<P>(&context, &mut ledger()?)?;
        let plan = AxisPhasePlan::new(
            source.case.clone(),
            &pi,
            LiteralLimits::new(32, 32, 1024),
            &context,
            &mut ledger()?,
        )?;
        Ok(Self {
            source,
            heap,
            input,
            plan,
            context,
        })
    }
    /// Prepare on the caller's actual cumulative ledger and one finite attempt identity.
    fn prepare(
        &self,
        lower: &str,
        upper: &str,
        budget: &mut Budget,
    ) -> TestResult<PreparedAxisPhase<P>> {
        let frozen = self
            .heap
            .freeze_axis_phase_input(&self.source.case, &self.input, budget)?;
        Ok(self
            .heap
            .prepare_axis_phase(
                frozen,
                &mut self.source.case.attempts(1),
                &enclosure(&self.source, lower, upper)?,
                &self.plan,
                &self.context,
                budget,
            )
            .map_err(|cause| format!("axis preparation [{lower},{upper}]: {cause:?}"))?)
    }
    /// The final publication shares the same cumulative allowance as all earlier preparation.
    fn publish(
        &self,
        lower: &str,
        upper: &str,
        budget: &mut Budget,
    ) -> TestResult<PublishedAxisPhase<P>> {
        let prepared = self.prepare(lower, upper, budget)?;
        Ok(self.heap.publish_axis_phase(prepared, budget)?)
    }
}

/// Independent pi and reciprocal endpoint bounds validate both floating formats.
fn twins<P: Scalar + 'static>() -> TestResult {
    for (frequency, center, lower, upper, expected) in [
        (1000, "-75", "-76", "-74", AxisPhaseOutcome::Negative),
        (4000, "75", "74", "76", AxisPhaseOutcome::Positive),
    ] {
        let case = Case::<P>::new(frequency, center, "0.25")?;
        let usage = case.heap.provenance_usage();
        let published = case.publish(lower, upper, &mut ledger()?)?;
        assert_eq!(case.heap.provenance_usage(), usage);
        let attempt = published.current(&case.heap, &case.context, &mut ledger()?)?;
        assert_eq!(attempt.outcome(), expected);
        assert!(
            attempt
                .reciprocal()
                .ok_or("discharged original")?
                .is_discharged()
        );
        assert!(
            published
                .plan()
                .constant()
                .same_region(case.plan.constant())
        );
        let phase = published.phase(&case.heap, &case.context, &mut ledger()?)?;
        let positive = if expected == AxisPhaseOutcome::Negative {
            phase.negated()
        } else {
            phase
        };
        let oracle = super::pi_conversion::oracle::factor(1, 2, false, 56)?;
        contains(positive, oracle)?;
        let reciprocal = published.reciprocal(&case.heap, &case.context, &mut ledger()?)?;
        let bounds = if expected == AxisPhaseOutcome::Negative {
            [Rational::new(-4, 299)?, Rational::new(-4, 301)?]
        } else {
            [Rational::new(4, 301)?, Rational::new(4, 299)?]
        };
        contains(reciprocal, bounds)?;
    }
    Ok(())
}

/// Compare represented endpoints to independent exact fractions without production enclosure logic.
fn contains<P: Scalar>(actual: Enclosure<P>, expected: [Rational; 2]) -> TestResult {
    assert!(Rational::binary(actual.lower().get().to_f64())?.at_most(expected[0])?);
    assert!(expected[1].at_most(Rational::binary(actual.upper().get().to_f64())?)?);
    Ok(())
}

/// Both-format below/above LC twins select exact half-pi branches with original nonzero discharge.
/// ´test:integration:lc-axis-phase-below-above-twins´
#[test]
fn lc_axis_phase_below_above_twins() -> TestResult {
    twins::<f32>()?;
    twins::<f64>()
}

/// At resonance and across zero there is an explicit outcome and no fabricated numerical phase.
/// ´test:integration:lc-axis-phase-zero-undecided-twins´
#[test]
fn lc_axis_phase_zero_undecided_twins() -> TestResult {
    undefined::<f32>()?;
    undefined::<f64>()
}

/// Closed original zero and strict-sign nominal targets retain different unresolved boxes.
fn undefined<P: Scalar + 'static>() -> TestResult {
    for (frequency, center, radius, lower, upper, expected) in [
        (2000, "0", "0", "0", "0", AxisPhaseOutcome::Undefined),
        (
            2000,
            "0",
            "0.25",
            "-1/4",
            "1/4",
            AxisPhaseOutcome::Undecided,
        ),
        (1000, "-75", "76", "-152", "2", AxisPhaseOutcome::Undecided),
        (4000, "75", "76", "-2", "152", AxisPhaseOutcome::Undecided),
    ] {
        let case = Case::<P>::new(frequency, center, radius)?;
        let published = case.publish(lower, upper, &mut ledger()?)?;
        let attempt = published.current(&case.heap, &case.context, &mut ledger()?)?;
        assert_eq!(attempt.outcome(), expected);
        assert!(attempt.phase().is_none());
        assert!(attempt.reciprocal().is_none());
        let phase = published.phase(&case.heap, &case.context, &mut ledger()?);
        let reciprocal = published.reciprocal(&case.heap, &case.context, &mut ledger()?);
        if expected == AxisPhaseOutcome::Undefined {
            assert!(matches!(phase, Err(LiveDecisionError::UndefinedPhase)));
            assert!(matches!(reciprocal, Err(LiveDecisionError::UndefinedPhase)));
        } else {
            assert!(matches!(phase, Err(LiveDecisionError::UndecidedPhase)));
            assert!(matches!(reciprocal, Err(LiveDecisionError::UndecidedPhase)));
        }
        assert_eq!(
            case.heap
                .value_by_symbol(case.input.symbol(), &case.input)?
                .region()
                .get(0, 0)?,
            scalar(center)?
        );
        let radius = scalar::<P>(radius)?;
        let unsafe_box = Enclosure::singleton(Finite::new(scalar(center)?)?)
            .add(Enclosure::new(-radius, radius)?, &case.context)?;
        assert!(
            Enclosure::singleton(Finite::ONE)
                .div(unsafe_box, &case.context)
                .is_err()
        );
    }
    Ok(())
}
