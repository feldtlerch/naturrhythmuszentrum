// SPDX-License-Identifier: AGPL-3.0-only
//! One sealed full-cycle interpretation owns both frequency channels and their ordered cross block.

mod check;
use super::{AT, LcAngularFrequencyDefinition, ReadingError, SpectralLawError as Error};
use crate::readings::{CompanionReading, EntryMeaning, ParameterErrorReading, Representation};
use crate::{CheckedDeclarations, CheckedMap, Descriptor, FrequencyRelation, Kind, Measure, Role};
use nrz_cas::{
    Cas,
    limits::{Budget, BudgetKind},
};
use std::rc::Rc;

/// Proposed cyclic channel and the complete ordered angular/cyclic covariance interpretation.
#[derive(Clone, Debug)]
pub struct LcFrequencySymbols {
    /// Fresh coherent scalar cyclic-frequency value.
    pub cyclic: Representation,
    /// Own-target total radius, numerical radius and cyclic self covariance, in that order.
    pub companions: [CompanionReading; 3],
    /// Numerical error targeting the cyclic covariance parameter.
    pub covariance_error: ParameterErrorReading,
    /// Identity reading of the cyclic covariance-error source coordinate.
    pub covariance_error_value: Representation,
    /// Cross covariance with angular frequency on the left and cyclic frequency on the right.
    pub cross_covariance: CompanionReading,
    /// Numerical error targeting that exact ordered cross-covariance parameter.
    pub cross_error: ParameterErrorReading,
    /// Identity reading of the cross-covariance-error source coordinate.
    pub cross_error_value: Representation,
}

/// Original angular law and an explicit full-cycle map into a complete fresh cyclic channel.
#[derive(Clone, Debug)]
pub struct LcFrequencyDefinition(Rc<Definition>);

/// Both exact meanings and all parameter targets survive independently of numerical values.
#[derive(Debug)]
struct Definition {
    /// Original physical-center angular law and its complete channel declarations.
    angular: LcAngularFrequencyDefinition,
    /// Explicit source-free angle corresponding to one complete cycle.
    cycle: EntryMeaning,
    /// Registered relation retains distinct kinds and the exact full-cycle descriptor.
    relation: FrequencyRelation,
    /// Frozen declaration identity and exact coefficient consumed by certified conversion.
    map: CheckedMap,
    /// Cyclic channel and angular/cyclic cross-block declarations.
    symbols: LcFrequencySymbols,
    /// Conservative newly owned metadata allowance, excluding shared original owners.
    bytes: u64,
}

impl LcFrequencyDefinition {
    /// Seal both channels and all ordered parameters against the original full-cycle relation.
    /// # Errors
    /// Refuses changed cycle, kind, role, identity map, context, target, alias or exact resources.
    pub fn new(
        cas: &Cas,
        angular: LcAngularFrequencyDefinition,
        cycle: EntryMeaning,
        symbols: LcFrequencySymbols,
        declarations: &CheckedDeclarations,
        budget: &mut Budget,
    ) -> Result<Self, Error> {
        let target = Descriptor::new(
            Kind::frequency().map_err(ReadingError::from)?,
            Measure::coherent(),
            Role::Difference,
        )
        .map_err(ReadingError::from)?;
        check::symbols(cas, &angular, &symbols, &target, budget)?;
        let relation = declarations
            .registry()
            .frequency_relation(angular.descriptor(), &target)
            .map_err(ReadingError::from)?;
        budget.work(2, AT)?;
        if cycle.descriptor() != &relation.cycle().coherent()
            || cycle.magnitude() != relation.cycle().measure().scale()
        {
            return Err(ReadingError::Meaning.into());
        }
        let map = declarations
            .frequency_map(angular.descriptor(), &target)
            .map_err(ReadingError::from)?;
        let bytes = check::storage(&symbols, budget)?;
        budget.check(BudgetKind::OutputBytes, bytes, AT)?;
        Ok(Self(Rc::new(Definition {
            angular,
            cycle,
            relation,
            map,
            symbols,
            bytes,
        })))
    }

    /// Original angular definition, including the retained positive LC law and exact radian.
    #[must_use]
    pub fn angular(&self) -> &LcAngularFrequencyDefinition {
        &self.0.angular
    }

    /// Explicit exact full-cycle angle; its evaluation error is never a statistical source.
    #[must_use]
    pub fn cycle(&self) -> &EntryMeaning {
        &self.0.cycle
    }

    /// Registered relation retains both exact frequency kinds and the original cycle descriptor.
    #[must_use]
    pub fn relation(&self) -> &FrequencyRelation {
        &self.0.relation
    }

    /// Exact checked map supplied to the existing certified conversion route.
    #[must_use]
    pub fn map(&self) -> &CheckedMap {
        &self.0.map
    }

    /// Complete cyclic channel and ordered cross-covariance declarations.
    #[must_use]
    pub fn symbols(&self) -> &LcFrequencySymbols {
        &self.0.symbols
    }

    /// Distinct coherent cyclic-frequency descriptor.
    #[must_use]
    pub fn descriptor(&self) -> &Descriptor {
        self.0.symbols.cyclic.reading().entries()[0].descriptor()
    }

    /// New metadata allowance; original law, exact map and declarations share their owners.
    #[must_use]
    pub fn retained_payload_bytes(&self) -> u64 {
        self.0.bytes
    }
}
