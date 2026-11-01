// SPDX-License-Identifier: AGPL-3.0-only
//! Existing LC equations provide the original fixed impedance family without executing a solve.

use super::{AxisPhaseCase, DecisionError, storage};
use crate::readings::{PhasorSolveDefinition, Representation};
use crate::{Budget, Dimension, ExactConstant, Measure, Role};
use std::rc::Rc;

/// Predeclared association between a closed sign question and its actual original LC impedance.
#[derive(Clone, Debug)]
pub struct AxisPhaseImpedance(Rc<Data>);

/// Both original declarations remain shared; no new reactance source or solve result is created.
#[derive(Debug)]
struct Data {
    /// Original physical ratio, CAS question and all exact continuations.
    case: AxisPhaseCase,
    /// Existing complete equation owns the zero-real-part proof and fixed imaginary component.
    equation: PhasorSolveDefinition,
    /// New association bytes exclude its already retained original declarations.
    bytes: u64,
}

impl AxisPhaseImpedance {
    /// Associate the original fixed coherent impedance with its closed normalized sign target.
    ///
    /// Both physical component meanings must use the same one-ohm reference. The existing
    /// equation must prove identically zero real part and the exact original reactance;
    /// no numerical center, root evaluation or current solve participates in this check.
    /// # Errors
    /// Refuses varying reactance, incompatible physical meanings, changed targets and limits.
    pub fn new(
        case: AxisPhaseCase,
        equation: PhasorSolveDefinition,
        budget: &mut Budget,
    ) -> Result<Self, DecisionError> {
        let entries = equation.impedance().reading().entries();
        let [real, imaginary] = entries else {
            return Err(DecisionError::Association);
        };
        real.check(imaginary)?;
        let descriptor = imaginary.descriptor();
        budget.work(8, storage::AT)?;
        if descriptor.dimension() != &Dimension::integers([2, 1, -3, -2, 0, 0, 0])?
            || descriptor.role() != Role::Difference
            || descriptor.measure() != &Measure::coherent()
            || !imaginary
                .magnitude()
                .structurally_equal(&ExactConstant::one(), budget)?
        {
            return Err(DecisionError::Association);
        }
        let [Some(zero), Some(reactance)] = &equation.fixed_inputs()[0] else {
            return Err(DecisionError::Source);
        };
        if !zero.is_zero() {
            return Err(DecisionError::Source);
        }
        let target = case.source().exact().ok_or(DecisionError::Source)?;
        if !target.structurally_equal(&ExactConstant::from(reactance.clone()), budget)? {
            return Err(DecisionError::Target);
        }
        let bytes = u64::try_from(size_of::<Data>() + 2 * size_of::<usize>())
            .map_err(|_| DecisionError::Capacity)?;
        storage::retained(bytes, budget)?;
        Ok(Self(Rc::new(Data {
            case,
            equation,
            bytes,
        })))
    }
    /// Match the complete original representation and reconstruction, never equal center bytes.
    /// # Errors
    /// Refuses a substituted family, map, reconstruction or caller resource limit.
    pub fn authenticate(
        &self,
        impedance: &Representation,
        budget: &mut Budget,
    ) -> Result<(), DecisionError> {
        self.0.equation.correspondence().check_operands(
            &[impedance.clone(), self.0.equation.output().clone()],
            budget,
        )?;
        Ok(())
    }
    /// Original sign question and predeclared consumers.
    #[must_use]
    pub fn case(&self) -> &AxisPhaseCase {
        &self.0.case
    }
    /// Existing equation retains all physical meanings and whole-family evidence.
    #[must_use]
    pub fn equation(&self) -> &PhasorSolveDefinition {
        &self.0.equation
    }
    /// New association allowance, excluding shared original case and equation owners.
    #[must_use]
    pub fn retained_payload_bytes(&self) -> u64 {
        self.0.bytes
    }
}
