// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Standard vocabulary IRIs shared by every entailment engine.
//!
//! These are spec-supplied `rdf:`/`rdfs:`/`owl:` IRIs from the RDF 1.1 Semantics /
//! OWL 2 calculus — PurRDF mints **none** of its own. Every engine (the RDFS/OWL-RL
//! chase, the OWL-Direct tableau, the RIF-Core evaluator) draws its constant IRIs
//! from this one table so there is a single source of truth.

/// `rdf:type`.
pub(crate) const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
/// `rdf:Property`.
pub(crate) const RDF_PROPERTY: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#Property";
/// `rdfs:subClassOf`.
pub(crate) const RDFS_SUBCLASSOF: &str = "http://www.w3.org/2000/01/rdf-schema#subClassOf";
/// `rdfs:subPropertyOf`.
pub(crate) const RDFS_SUBPROPERTYOF: &str = "http://www.w3.org/2000/01/rdf-schema#subPropertyOf";
/// `rdfs:domain`.
pub(crate) const RDFS_DOMAIN: &str = "http://www.w3.org/2000/01/rdf-schema#domain";
/// `rdfs:range`.
pub(crate) const RDFS_RANGE: &str = "http://www.w3.org/2000/01/rdf-schema#range";
/// `rdfs:Class`.
pub(crate) const RDFS_CLASS: &str = "http://www.w3.org/2000/01/rdf-schema#Class";
/// `rdfs:Resource`.
pub(crate) const RDFS_RESOURCE: &str = "http://www.w3.org/2000/01/rdf-schema#Resource";
/// `owl:equivalentClass`.
pub(crate) const OWL_EQUIVALENTCLASS: &str = "http://www.w3.org/2002/07/owl#equivalentClass";
/// `owl:equivalentProperty`.
pub(crate) const OWL_EQUIVALENTPROPERTY: &str = "http://www.w3.org/2002/07/owl#equivalentProperty";
/// `owl:inverseOf`.
pub(crate) const OWL_INVERSEOF: &str = "http://www.w3.org/2002/07/owl#inverseOf";
/// `owl:SymmetricProperty`.
pub(crate) const OWL_SYMMETRICPROPERTY: &str = "http://www.w3.org/2002/07/owl#SymmetricProperty";
/// `owl:TransitiveProperty`.
pub(crate) const OWL_TRANSITIVEPROPERTY: &str = "http://www.w3.org/2002/07/owl#TransitiveProperty";
