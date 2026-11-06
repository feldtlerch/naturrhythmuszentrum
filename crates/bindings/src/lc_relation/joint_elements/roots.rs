// SPDX-License-Identifier: AGPL-3.0-only
//! Checked inverse product-root endpoints retain complete original zero-epoch timing and element fibres.

mod domain;
mod witness;

use super::{AT, LcRelationError, calibration, storage};
use crate::{
    Budget, ExactConstant, LcCertainty, LcDeterminacy, LcPhaseGauge, LcRelation, LcWitness,
};
use domain::Domain;
use nrz_cas::{Rational, limits::BudgetUsage};
use std::{cmp::Ordering, rc::Rc};

/// One checked exact inverse product root with a full original endpoint witness.
#[derive(Clone, Debug)]
pub struct LcRootRateEndpoint {
    /// Positive rational original product; the rate is its exact inverse square root.
    product: Rational,
    /// Original equations, all observations and input membership checked at this endpoint.
    witness: LcWitness,
}

/// Complete simultaneous L/C domain with exact product-root endpoints and zero-epoch observations.
#[derive(Clone, Debug)]
pub struct LcJointRootCover(Rc<Data>);

/// Original source authority is retained beside complete product and conditional element domains.
#[derive(Debug)]
struct Data {
    /// Every original declaration and its named epoch.
    relation: LcRelation,
    /// Original rectangle and its exact product image.
    domain: Domain,
    /// Lower and upper rate endpoints, each with original physical reconstruction.
    endpoints: [LcRootRateEndpoint; 2],
    /// Cumulative complete construction snapshots.
    usage: [BudgetUsage; 2],
    /// Conservative complete newly retained metadata, excluding the shared original relation.
    bytes: u64,
}

/// Exact product coordinates select the algebraic rate and a compatible original element pair.
#[derive(Clone, Debug)]
pub struct LcJointRootParameters {
    /// Positive product in the complete original rectangle image.
    pub product: Rational,
    /// Capacitance in the original conditional fibre at that product.
    pub capacitance: Rational,
    /// Free positive voltage amplitude on the oscillating branch.
    pub voltage: Rational,
}

impl LcRelation {
    /// Admit exact nonrational product-root endpoints for a nonempty zero-epoch observation inventory.
    ///
    /// Every original declaration must have finite rational endpoints. Repeated L/C declarations
    /// must intersect to positive bounded intervals; initial phase and every original time/phase
    /// intersection must be exactly zero, fixing every cycle to zero. Extra quantities and general
    /// nonzero-time recovery remain separate adapters.
    /// # Errors
    /// Refuses incomplete/incompatible inventories, other scales or rate declarations, unsupported exact
    /// root reconstruction and exhausted caller resources, without claiming impossibility.
    pub fn joint_elements_root_cover(
        &self,
        budget: &mut Budget,
    ) -> Result<LcJointRootCover, LcRelationError> {
        let before = budget.usage();
        let domain = Domain::check(self, budget)?;
        let lower = witness::restore(
            self,
            &domain.product[1],
            &domain.elements[1][1],
            &Rational::one(),
            budget,
        )?;
        let retained = domain
            .bytes()
            .saturating_add(lower.retained_payload_bytes())
            .saturating_add(self.retained_payload_bytes());
        let upper = budget.with_retained_payload(retained, AT, |budget| {
            if domain.product[0].compare(&domain.product[1], budget)? == Ordering::Equal {
                storage::retained(lower.retained_payload_bytes(), budget)?;
                return Ok(lower.clone());
            }
            witness::restore(
                self,
                &domain.product[0],
                &domain.elements[1][0],
                &Rational::one(),
                budget,
            )
        })?;
        let endpoints = [
            LcRootRateEndpoint {
                product: domain.product[1].clone(),
                witness: lower,
            },
            LcRootRateEndpoint {
                product: domain.product[0].clone(),
                witness: upper,
            },
        ];
        let bytes = endpoints
            .iter()
            .fold(domain.bytes().saturating_add(1_024), |sum, endpoint| {
                sum.saturating_add(endpoint.witness.retained_payload_bytes())
                    .saturating_add(storage::rational(&endpoint.product))
            });
        storage::retained(bytes.saturating_add(self.retained_payload_bytes()), budget)?;
        Ok(LcJointRootCover(Rc::new(Data {
            relation: self.clone(),
            domain,
            endpoints,
            usage: [before, budget.usage()],
            bytes,
        })))
    }
}

impl LcRootRateEndpoint {
    /// Exact positive algebraic rate, without a rational outer replacement.
    #[must_use]
    pub const fn exact(&self) -> &ExactConstant {
        &self.witness.values().rate
    }

    /// Positive original product whose inverse square root is the endpoint.
    #[must_use]
    pub const fn product(&self) -> &Rational {
        &self.product
    }

    /// Full original endpoint tuple, including every original zero-epoch observation.
    #[must_use]
    pub const fn witness(&self) -> &LcWitness {
        &self.witness
    }

    /// Compare the exact root with any rational using positive-domain squaring on arbitrary precision.
    /// # Errors
    /// Refuses exhausted caller resources; no approximate root determines the order.
    pub fn compare_rational(
        &self,
        value: &Rational,
        budget: &mut Budget,
    ) -> Result<Ordering, LcRelationError> {
        budget.work(1, AT)?;
        value.check(budget, AT)?;
        if value.is_zero() || value.is_negative() {
            return Ok(Ordering::Greater);
        }
        Ok(Rational::one().compare(&self.product.mul(&value.pow(2, budget)?, budget)?, budget)?)
    }
}

impl LcJointRootCover {
    /// Complete original physical question with its unchanged source allocation.
    #[must_use]
    pub fn relation(&self) -> &LcRelation {
        &self.0.relation
    }

    /// Checked lower and upper exact rate endpoints, each retaining its original extremizer.
    #[must_use]
    pub fn rate_endpoints(&self) -> &[LcRootRateEndpoint; 2] {
        &self.0.endpoints
    }

    /// Complete rational product image; its inverse square root reverses endpoint order.
    #[must_use]
    pub fn product_bounds(&self) -> &[Rational; 2] {
        &self.0.domain.product
    }

    /// Every repeated original L/C declaration remains retained beside these intersections.
    #[must_use]
    pub fn original_elements(&self) -> &[[Rational; 2]; 2] {
        &self.0.domain.elements
    }

    /// Exact bounded rate image with checked endpoint attainment.
    #[must_use]
    pub const fn determinacy(&self) -> LcDeterminacy {
        LcDeterminacy::DeterminedFinite
    }

    /// Complete original product, timing and cycle coverage is certified independently of rendering.
    #[must_use]
    pub const fn certainty(&self) -> LcCertainty {
        LcCertainty::Certified
    }

    /// Known zero initial phase fixes the common gauge and every original cycle to zero.
    #[must_use]
    pub const fn phase_gauge(&self) -> LcPhaseGauge {
        LcPhaseGauge::Fixed
    }

    /// Exact original fibre `C_original intersect [P/L_max,P/L_min]` at a selected product.
    /// # Errors
    /// Refuses products outside the complete image and exhausted resources.
    pub fn capacitance_at_product(
        &self,
        product: &Rational,
        budget: &mut Budget,
    ) -> Result<[Rational; 2], LcRelationError> {
        self.0.domain.capacitance(product, budget)
    }

    /// Reconstruct the algebraic rate and recheck every original equation and observation.
    /// # Errors
    /// Refuses crossed conditional parameters, nonpositive voltage and exhausted resources.
    pub fn witness(
        &self,
        parameters: &LcJointRootParameters,
        budget: &mut Budget,
    ) -> Result<LcWitness, LcRelationError> {
        budget.with_retained_payload(
            self.0
                .bytes
                .saturating_add(self.relation().retained_payload_bytes()),
            AT,
            |budget| {
                let fibre = self.0.domain.capacitance(&parameters.product, budget)?;
                calibration::contains(&fibre, &parameters.capacitance, budget)?;
                witness::restore(
                    self.relation(),
                    &parameters.product,
                    &parameters.capacitance,
                    &parameters.voltage,
                    budget,
                )
            },
        )
    }

    /// Construction work remains on the original cumulative caller ledger.
    #[must_use]
    pub fn usage(&self) -> [BudgetUsage; 2] {
        self.0.usage
    }

    /// Conservative newly retained ownership, excluding the original shared relation.
    #[must_use]
    pub fn retained_payload_bytes(&self) -> u64 {
        self.0.bytes
    }
}
