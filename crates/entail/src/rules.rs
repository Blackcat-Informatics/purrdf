// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The machine-readable entailment rule inventory.
//!
//! [`RuleId`] names every rule of the two calculi this crate speaks about — the 78
//! OWL 2 RL/RDF rules of the OWL 2 Profiles specification, §4.3, Tables 4–9, and the
//! 18 RDF / RDFS entailment patterns of RDF 1.2 Semantics, §8.1.1 and §9.2.1 — as data
//! rather than as prose — plus the rules this crate fires that NO specification table
//! states. Three functions turn a [`Regime`] into a rule list:
//!
//! * [`rules`] — the rule table the regime is *defined by*: what the specification
//!   says a complete implementation must fire.
//! * [`implemented`] — the subset this crate's chase actually fires today.
//! * [`extensions`] — what the chase fires BEYOND that table, if anything.
//!
//! `rules(r)` minus `implemented(r)` is therefore the regime's gap, expressed as an
//! executable artifact instead of an assertion in a README, and `extensions(r)` is the
//! overshoot in the other direction, expressed the same way. All three return `&'static`
//! slices in specification table order, so the inventory is deterministic and free of
//! map iteration. For `RDF`, `RDFS`, `OWL-RL` and `D` the gap is now EMPTY — which is a
//! claim about the RULE TABLE and not about the closure; see
//! [`Completeness::ExactWithinBoundaries`](crate::Completeness::ExactWithinBoundaries) for
//! the difference and why a report states both.
//!
//! # The two directions are never mixed
//!
//! An extension is not an implemented rule with a footnote: it is in a fourth category
//! that [`RuleId::is_extension`] decides, and neither [`rules`] nor [`implemented`] ever
//! names one. That is what keeps `OWL-RL 78 / 78` a true statement about OWL 2
//! Profiles §4.3 Tables 4–9 while the lane's closure is strictly larger than those tables
//! license, and it is why a report renders the extension list beside the fired list rather
//! than folding one into the other.
//!
//! PurRDF mints no vocabulary here: [`RuleId`] carries specification rule *names*
//! (`"eq-ref"`, `"cls-svf1"`, `"rdfs4"`), not IRIs.
//!
//! # Canonical spellings, and the legacy names accepted as input
//!
//! [`RuleId::as_str`] returns the spelling used by the *current* specification, which
//! for the RDF and RDFS patterns is RDF 1.2 Semantics. RDF 1.2 differs from the RDF 1.0
//! (2004) names still in wide circulation in three ways, all of them real changes to
//! the rule set rather than cosmetic renames:
//!
//! * The RDF patterns are `rdfD1` (datatyped-literal typing) and `rdfD2` (predicate
//!   typing). The numbering is *crossed* relative to RDF 1.0: `rdf1` is `rdfD2` and
//!   `rdf2` is `rdfD1`. RDF 1.2 also adds a third, `rdfD1a`.
//! * RDF 1.0's `rdfs4a` and `rdfs4b` are merged into a single `rdfs4` that ranges over
//!   every position of a triple, not just subject and object.
//! * RDF 1.2 adds `rdfs14` and `rdfs14a` for triple terms and `rdfs:Proposition`.
//!
//! [`FromStr`] additionally accepts the superseded spellings `rdf1`, `rdf2`, `rdfs4a`,
//! and `rdfs4b` as input aliases, so a rule list written against RDF 1.0/1.1 still
//! parses. Note that `rdfs4a` and `rdfs4b` both widen to `rdfs4`: recognizing them
//! loses the subject/object distinction the RDF 1.2 rule no longer draws, so a caller
//! reconstructing a coverage claim from legacy ids will *over*-report unless it accounts
//! for that. Nothing is ever emitted under a superseded name.

use core::fmt;
use core::str::FromStr;

use crate::Regime;

/// Declare [`RuleId`] plus its specification spellings from one table, so a variant,
/// its documentation, and its canonical name cannot drift apart.
///
/// Each entry is `Variant = "canonical"`, optionally followed by `| "alias"` for each
/// superseded spelling that [`FromStr`] should still accept. Aliases are input-only:
/// [`RuleId::as_str`] always answers with the canonical name.
macro_rules! rule_ids {
    ($( $(#[$attr:meta])* $variant:ident = $spelling:literal $( | $alias:literal )* ),+ $(,)?) => {
        /// A single entailment rule, by its canonical specification name.
        ///
        /// Variants are declared in specification table order — OWL 2 RL Tables 4, 5,
        /// 6, 7, 8, 9, then the RDF patterns, then the RDFS patterns, then the
        /// EXTENSIONS this crate declares beyond every specification table — and the
        /// derived [`Ord`] follows that declaration order, so any ordered collection of
        /// `RuleId` reads in the order the specifications present the rules and the
        /// non-normative rules sort last, where they cannot be mistaken for one.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum RuleId {
            $( $(#[$attr])* $variant, )+
        }

        impl RuleId {
            /// Every rule id this crate knows, in specification table order: the 78
            /// OWL 2 RL rules of Tables 4–9, then the RDF patterns of RDF 1.2
            /// Semantics §8.1.1, then the RDFS patterns of §9.2.1, then the
            /// extensions — which are LAST, and are the only entries
            /// [`RuleId::is_extension`] answers `true` for.
            pub const ALL: &'static [Self] = &[ $( Self::$variant, )+ ];

            /// The canonical spelling used by the current specification, exactly as the
            /// rule tables write it (`"eq-ref"`, `"cls-svf1"`, `"rdfs4"`).
            ///
            /// Never a superseded spelling, even for the variants whose superseded
            /// names [`FromStr`] still accepts.
            #[must_use]
            pub const fn as_str(self) -> &'static str {
                match self {
                    $( Self::$variant => $spelling, )+
                }
            }
        }

        impl RuleId {
            /// Whether this rule is an EXTENSION — a rule this crate's chase fires that
            /// no specification table states.
            ///
            /// Decided by the canonical spelling's `ext-` prefix, which no rule of
            /// OWL 2 Profiles §4.3 Tables 4–9 or of RDF 1.2 Semantics §8.1.1 / §9.2.1
            /// carries — `the_extension_prefix_is_disjoint_from_the_specification_names`
            /// asserts that over the whole enum, so the predicate cannot start
            /// answering `true` for a normative rule because someone added a variant.
            ///
            /// This is the line between "PurRDF implements the table" and "PurRDF's
            /// closure is the table's". [`rules`] and [`implemented`] describe the
            /// first and never name an extension; [`extensions`] names exactly the
            /// second, and a caller that wants strictly normative behaviour compares a
            /// report's fired rules against it.
            ///
            /// ```
            /// use purrdf_entail::{Regime, RuleId, extensions, implemented, rules};
            ///
            /// assert!(RuleId::ExtEqDiffSym.is_extension());
            /// assert!(!RuleId::PrpSymp.is_extension());
            /// // An extension is in no specification table, by construction.
            /// for &rule in extensions(Regime::OwlRl) {
            ///     assert!(rule.is_extension());
            ///     assert!(!rules(Regime::OwlRl).contains(&rule));
            ///     assert!(!implemented(Regime::OwlRl).contains(&rule));
            /// }
            /// ```
            #[must_use]
            pub const fn is_extension(self) -> bool {
                matches!(self.as_str().as_bytes(), [b'e', b'x', b't', b'-', ..])
            }
        }

        impl FromStr for RuleId {
            type Err = ParseRuleIdError;

            /// Left inverse of [`RuleId::as_str`], widened to also accept the
            /// superseded spellings listed in the module documentation. Matching is
            /// case-sensitive, as the specifications write the names.
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s {
                    $( $spelling $( | $alias )* => Ok(Self::$variant), )+
                    other => Err(ParseRuleIdError(other.to_owned())),
                }
            }
        }
    };
}

rule_ids! {
    // --- OWL 2 Profiles §4.3 Table 4: The Semantics of Equality (9 rules). ---
    /// `eq-ref`: `T(?s, ?p, ?o)` ⇒ `?s owl:sameAs ?s`, `?p owl:sameAs ?p`,
    /// `?o owl:sameAs ?o`.
    EqRef = "eq-ref",
    /// `eq-sym`: `?x owl:sameAs ?y` ⇒ `?y owl:sameAs ?x`.
    EqSym = "eq-sym",
    /// `eq-trans`: `?x owl:sameAs ?y`, `?y owl:sameAs ?z` ⇒ `?x owl:sameAs ?z`.
    EqTrans = "eq-trans",
    /// `eq-rep-s`: `?s owl:sameAs ?s'`, `T(?s, ?p, ?o)` ⇒ `T(?s', ?p, ?o)`.
    EqRepS = "eq-rep-s",
    /// `eq-rep-p`: `?p owl:sameAs ?p'`, `T(?s, ?p, ?o)` ⇒ `T(?s, ?p', ?o)`.
    EqRepP = "eq-rep-p",
    /// `eq-rep-o`: `?o owl:sameAs ?o'`, `T(?s, ?p, ?o)` ⇒ `T(?s, ?p, ?o')`.
    EqRepO = "eq-rep-o",
    /// `eq-diff1`: `?x owl:sameAs ?y`, `?x owl:differentFrom ?y` ⇒ inconsistency.
    EqDiff1 = "eq-diff1",
    /// `eq-diff2`: an `owl:AllDifferent` whose `owl:members` list holds two members
    /// asserted `owl:sameAs` ⇒ inconsistency.
    EqDiff2 = "eq-diff2",
    /// `eq-diff3`: an `owl:AllDifferent` whose `owl:distinctMembers` list holds two
    /// members asserted `owl:sameAs` ⇒ inconsistency.
    EqDiff3 = "eq-diff3",

    // --- Table 5: The Semantics of Axioms about Properties (20 rules). ---
    /// `prp-ap`: assert `ap rdf:type owl:AnnotationProperty` for each built-in
    /// annotation property of OWL 2 RL. Axiomatic; it has no premise.
    PrpAp = "prp-ap",
    /// `prp-dom`: `?p rdfs:domain ?c`, `T(?x, ?p, ?y)` ⇒ `?x rdf:type ?c`.
    PrpDom = "prp-dom",
    /// `prp-rng`: `?p rdfs:range ?c`, `T(?x, ?p, ?y)` ⇒ `?y rdf:type ?c`.
    PrpRng = "prp-rng",
    /// `prp-fp`: `?p rdf:type owl:FunctionalProperty`, `T(?x, ?p, ?y1)`,
    /// `T(?x, ?p, ?y2)` ⇒ `?y1 owl:sameAs ?y2`.
    PrpFp = "prp-fp",
    /// `prp-ifp`: `?p rdf:type owl:InverseFunctionalProperty`, `T(?x1, ?p, ?y)`,
    /// `T(?x2, ?p, ?y)` ⇒ `?x1 owl:sameAs ?x2`.
    PrpIfp = "prp-ifp",
    /// `prp-irp`: `?p rdf:type owl:IrreflexiveProperty`, `T(?x, ?p, ?x)` ⇒
    /// inconsistency.
    PrpIrp = "prp-irp",
    /// `prp-symp`: `?p rdf:type owl:SymmetricProperty`, `T(?x, ?p, ?y)` ⇒
    /// `T(?y, ?p, ?x)`.
    PrpSymp = "prp-symp",
    /// `prp-asyp`: `?p rdf:type owl:AsymmetricProperty`, `T(?x, ?p, ?y)`,
    /// `T(?y, ?p, ?x)` ⇒ inconsistency.
    PrpAsyp = "prp-asyp",
    /// `prp-trp`: `?p rdf:type owl:TransitiveProperty`, `T(?x, ?p, ?y)`,
    /// `T(?y, ?p, ?z)` ⇒ `T(?x, ?p, ?z)`.
    PrpTrp = "prp-trp",
    /// `prp-spo1`: `?p1 rdfs:subPropertyOf ?p2`, `T(?x, ?p1, ?y)` ⇒ `T(?x, ?p2, ?y)`.
    PrpSpo1 = "prp-spo1",
    /// `prp-spo2`: `?p owl:propertyChainAxiom ?x` over the list `?p1 … ?pn`, matched
    /// by a chain `T(?u1, ?p1, ?u2) … T(?un, ?pn, ?un+1)` ⇒ `T(?u1, ?p, ?un+1)`.
    PrpSpo2 = "prp-spo2",
    /// `prp-eqp1`: `?p1 owl:equivalentProperty ?p2`, `T(?x, ?p1, ?y)` ⇒
    /// `T(?x, ?p2, ?y)`.
    PrpEqp1 = "prp-eqp1",
    /// `prp-eqp2`: `?p1 owl:equivalentProperty ?p2`, `T(?x, ?p2, ?y)` ⇒
    /// `T(?x, ?p1, ?y)`.
    PrpEqp2 = "prp-eqp2",
    /// `prp-pdw`: `?p1 owl:propertyDisjointWith ?p2`, `T(?x, ?p1, ?y)`,
    /// `T(?x, ?p2, ?y)` ⇒ inconsistency.
    PrpPdw = "prp-pdw",
    /// `prp-adp`: an `owl:AllDisjointProperties` whose `owl:members` list holds two
    /// properties sharing a subject/object pair ⇒ inconsistency.
    PrpAdp = "prp-adp",
    /// `prp-inv1`: `?p1 owl:inverseOf ?p2`, `T(?x, ?p1, ?y)` ⇒ `T(?y, ?p2, ?x)`.
    PrpInv1 = "prp-inv1",
    /// `prp-inv2`: `?p1 owl:inverseOf ?p2`, `T(?x, ?p2, ?y)` ⇒ `T(?y, ?p1, ?x)`.
    PrpInv2 = "prp-inv2",
    /// `prp-key`: `?c owl:hasKey ?u` over the list `?p1 … ?pn`, with two instances of
    /// `?c` agreeing on every key property ⇒ `?x owl:sameAs ?y`.
    PrpKey = "prp-key",
    /// `prp-npa1`: a negative object-property assertion
    /// (`owl:sourceIndividual`/`owl:assertionProperty`/`owl:targetIndividual`) whose
    /// triple is asserted ⇒ inconsistency.
    PrpNpa1 = "prp-npa1",
    /// `prp-npa2`: a negative data-property assertion
    /// (`owl:sourceIndividual`/`owl:assertionProperty`/`owl:targetValue`) whose triple
    /// is asserted ⇒ inconsistency.
    PrpNpa2 = "prp-npa2",

    // --- Table 6: The Semantics of Classes (19 rules). ---
    /// `cls-thing`: assert `owl:Thing rdf:type owl:Class`. Axiomatic; no premise.
    ClsThing = "cls-thing",
    /// `cls-nothing1`: assert `owl:Nothing rdf:type owl:Class`. Axiomatic; no premise.
    ClsNothing1 = "cls-nothing1",
    /// `cls-nothing2`: `?x rdf:type owl:Nothing` ⇒ inconsistency.
    ClsNothing2 = "cls-nothing2",
    /// `cls-int1`: `?c owl:intersectionOf ?x` over the list `?c1 … ?cn`, with `?y` in
    /// every `?ci` ⇒ `?y rdf:type ?c`.
    ClsInt1 = "cls-int1",
    /// `cls-int2`: `?c owl:intersectionOf ?x` over the list `?c1 … ?cn`, with
    /// `?y rdf:type ?c` ⇒ `?y rdf:type ?ci` for every `?ci`.
    ClsInt2 = "cls-int2",
    /// `cls-uni`: `?c owl:unionOf ?x` over the list `?c1 … ?cn`, with `?y rdf:type ?ci`
    /// for some `?ci` ⇒ `?y rdf:type ?c`.
    ClsUni = "cls-uni",
    /// `cls-com`: `?c1 owl:complementOf ?c2` with `?x` typed both ⇒ inconsistency.
    ClsCom = "cls-com",
    /// `cls-svf1`: `?x owl:someValuesFrom ?y`, `?x owl:onProperty ?p`, `T(?u, ?p, ?v)`,
    /// `?v rdf:type ?y` ⇒ `?u rdf:type ?x`.
    ClsSvf1 = "cls-svf1",
    /// `cls-svf2`: `?x owl:someValuesFrom owl:Thing`, `?x owl:onProperty ?p`,
    /// `T(?u, ?p, ?v)` ⇒ `?u rdf:type ?x`.
    ClsSvf2 = "cls-svf2",
    /// `cls-avf`: `?x owl:allValuesFrom ?y`, `?x owl:onProperty ?p`, `?u rdf:type ?x`,
    /// `T(?u, ?p, ?v)` ⇒ `?v rdf:type ?y`.
    ClsAvf = "cls-avf",
    /// `cls-hv1`: `?x owl:hasValue ?y`, `?x owl:onProperty ?p`, `?u rdf:type ?x` ⇒
    /// `T(?u, ?p, ?y)`.
    ClsHv1 = "cls-hv1",
    /// `cls-hv2`: `?x owl:hasValue ?y`, `?x owl:onProperty ?p`, `T(?u, ?p, ?y)` ⇒
    /// `?u rdf:type ?x`.
    ClsHv2 = "cls-hv2",
    /// `cls-maxc1`: an `owl:maxCardinality 0` restriction with a matching property
    /// assertion on one of its instances ⇒ inconsistency.
    ClsMaxc1 = "cls-maxc1",
    /// `cls-maxc2`: an `owl:maxCardinality 1` restriction with two property values on
    /// one of its instances ⇒ `?y1 owl:sameAs ?y2`.
    ClsMaxc2 = "cls-maxc2",
    /// `cls-maxqc1`: an `owl:maxQualifiedCardinality 0` restriction on `?c` with a
    /// matching value typed `?c` ⇒ inconsistency.
    ClsMaxqc1 = "cls-maxqc1",
    /// `cls-maxqc2`: an `owl:maxQualifiedCardinality 0` restriction on `owl:Thing`
    /// with any matching value ⇒ inconsistency.
    ClsMaxqc2 = "cls-maxqc2",
    /// `cls-maxqc3`: an `owl:maxQualifiedCardinality 1` restriction on `?c` with two
    /// values typed `?c` ⇒ `?y1 owl:sameAs ?y2`.
    ClsMaxqc3 = "cls-maxqc3",
    /// `cls-maxqc4`: an `owl:maxQualifiedCardinality 1` restriction on `owl:Thing`
    /// with two values ⇒ `?y1 owl:sameAs ?y2`.
    ClsMaxqc4 = "cls-maxqc4",
    /// `cls-oo`: `?c owl:oneOf ?x` over the list `?y1 … ?yn` ⇒ `?yi rdf:type ?c` for
    /// every `?yi`.
    ClsOo = "cls-oo",

    // --- Table 7: The Semantics of Class Axioms (5 rules). ---
    /// `cax-sco`: `?c1 rdfs:subClassOf ?c2`, `?x rdf:type ?c1` ⇒ `?x rdf:type ?c2`.
    CaxSco = "cax-sco",
    /// `cax-eqc1`: `?c1 owl:equivalentClass ?c2`, `?x rdf:type ?c1` ⇒
    /// `?x rdf:type ?c2`.
    CaxEqc1 = "cax-eqc1",
    /// `cax-eqc2`: `?c1 owl:equivalentClass ?c2`, `?x rdf:type ?c2` ⇒
    /// `?x rdf:type ?c1`.
    CaxEqc2 = "cax-eqc2",
    /// `cax-dw`: `?c1 owl:disjointWith ?c2` with `?x` typed both ⇒ inconsistency.
    CaxDw = "cax-dw",
    /// `cax-adc`: an `owl:AllDisjointClasses` whose `owl:members` list holds two
    /// classes sharing an instance ⇒ inconsistency.
    CaxAdc = "cax-adc",

    // --- Table 8: The Semantics of Datatypes (5 rules). ---
    /// `dt-type1`: assert `dt rdf:type rdfs:Datatype` for each datatype supported in
    /// OWL 2 RL. Axiomatic; no premise.
    DtType1 = "dt-type1",
    /// `dt-type2`: assert `lt rdf:type dt` for each literal whose data value lies in
    /// the value space of a supported datatype `dt`.
    DtType2 = "dt-type2",
    /// `dt-eq`: assert `lt1 owl:sameAs lt2` for all literals with the same data value.
    DtEq = "dt-eq",
    /// `dt-diff`: assert `lt1 owl:differentFrom lt2` for all literals with different
    /// data values.
    DtDiff = "dt-diff",
    /// `dt-not-type`: `lt rdf:type dt` where the data value of `lt` is outside the
    /// value space of `dt` ⇒ inconsistency.
    DtNotType = "dt-not-type",

    // --- Table 9: The Semantics of Schema Vocabulary (20 rules). ---
    /// `scm-cls`: `?c rdf:type owl:Class` ⇒ `?c rdfs:subClassOf ?c`,
    /// `?c owl:equivalentClass ?c`, `?c rdfs:subClassOf owl:Thing`,
    /// `owl:Nothing rdfs:subClassOf ?c`.
    ScmCls = "scm-cls",
    /// `scm-sco`: `?c1 rdfs:subClassOf ?c2`, `?c2 rdfs:subClassOf ?c3` ⇒
    /// `?c1 rdfs:subClassOf ?c3`.
    ScmSco = "scm-sco",
    /// `scm-eqc1`: `?c1 owl:equivalentClass ?c2` ⇒ `?c1 rdfs:subClassOf ?c2`,
    /// `?c2 rdfs:subClassOf ?c1`.
    ScmEqc1 = "scm-eqc1",
    /// `scm-eqc2`: `?c1 rdfs:subClassOf ?c2`, `?c2 rdfs:subClassOf ?c1` ⇒
    /// `?c1 owl:equivalentClass ?c2`.
    ScmEqc2 = "scm-eqc2",
    /// `scm-op`: `?p rdf:type owl:ObjectProperty` ⇒ `?p rdfs:subPropertyOf ?p`,
    /// `?p owl:equivalentProperty ?p`.
    ScmOp = "scm-op",
    /// `scm-dp`: `?p rdf:type owl:DatatypeProperty` ⇒ `?p rdfs:subPropertyOf ?p`,
    /// `?p owl:equivalentProperty ?p`.
    ScmDp = "scm-dp",
    /// `scm-spo`: `?p1 rdfs:subPropertyOf ?p2`, `?p2 rdfs:subPropertyOf ?p3` ⇒
    /// `?p1 rdfs:subPropertyOf ?p3`.
    ScmSpo = "scm-spo",
    /// `scm-eqp1`: `?p1 owl:equivalentProperty ?p2` ⇒ `?p1 rdfs:subPropertyOf ?p2`,
    /// `?p2 rdfs:subPropertyOf ?p1`.
    ScmEqp1 = "scm-eqp1",
    /// `scm-eqp2`: `?p1 rdfs:subPropertyOf ?p2`, `?p2 rdfs:subPropertyOf ?p1` ⇒
    /// `?p1 owl:equivalentProperty ?p2`.
    ScmEqp2 = "scm-eqp2",
    /// `scm-dom1`: `?p rdfs:domain ?c1`, `?c1 rdfs:subClassOf ?c2` ⇒
    /// `?p rdfs:domain ?c2`.
    ScmDom1 = "scm-dom1",
    /// `scm-dom2`: `?p2 rdfs:domain ?c`, `?p1 rdfs:subPropertyOf ?p2` ⇒
    /// `?p1 rdfs:domain ?c`.
    ScmDom2 = "scm-dom2",
    /// `scm-rng1`: `?p rdfs:range ?c1`, `?c1 rdfs:subClassOf ?c2` ⇒
    /// `?p rdfs:range ?c2`.
    ScmRng1 = "scm-rng1",
    /// `scm-rng2`: `?p2 rdfs:range ?c`, `?p1 rdfs:subPropertyOf ?p2` ⇒
    /// `?p1 rdfs:range ?c`.
    ScmRng2 = "scm-rng2",
    /// `scm-hv`: two `owl:hasValue` restrictions on the same value whose properties
    /// are related by `rdfs:subPropertyOf` ⇒ `?c1 rdfs:subClassOf ?c2`.
    ScmHv = "scm-hv",
    /// `scm-svf1`: two `owl:someValuesFrom` restrictions on the same property whose
    /// fillers are related by `rdfs:subClassOf` ⇒ `?c1 rdfs:subClassOf ?c2`.
    ScmSvf1 = "scm-svf1",
    /// `scm-svf2`: two `owl:someValuesFrom` restrictions on the same filler whose
    /// properties are related by `rdfs:subPropertyOf` ⇒ `?c1 rdfs:subClassOf ?c2`.
    ScmSvf2 = "scm-svf2",
    /// `scm-avf1`: two `owl:allValuesFrom` restrictions on the same property whose
    /// fillers are related by `rdfs:subClassOf` ⇒ `?c1 rdfs:subClassOf ?c2`.
    ScmAvf1 = "scm-avf1",
    /// `scm-avf2`: two `owl:allValuesFrom` restrictions on the same filler whose
    /// properties are related by `rdfs:subPropertyOf` ⇒ `?c2 rdfs:subClassOf ?c1`
    /// (the conclusion is contravariant, unlike `scm-svf2`).
    ScmAvf2 = "scm-avf2",
    /// `scm-int`: `?c owl:intersectionOf ?x` over the list `?c1 … ?cn` ⇒
    /// `?c rdfs:subClassOf ?ci` for every `?ci`.
    ScmInt = "scm-int",
    /// `scm-uni`: `?c owl:unionOf ?x` over the list `?c1 … ?cn` ⇒
    /// `?ci rdfs:subClassOf ?c` for every `?ci`.
    ScmUni = "scm-uni",

    // --- RDF 1.2 Semantics §8.1.1: patterns of RDF entailment (3 rules). ---
    /// `rdfD1`: any triple in which a datatyped literal `"sss"^^ddd` appears, for a
    /// recognized `ddd` ⇒ that triple with the literal replaced by a fresh `_:nnn`,
    /// plus `_:nnn rdf:type ddd`. Valid even when the literal is ill-typed.
    ///
    /// Spelled `rdf2` in RDF 1.0, where it covered `rdf:XMLLiteral` only — note the
    /// crossed numbering. `rdf2` is accepted as an input alias.
    RdfD1 = "rdfD1" | "rdf2",
    /// `rdfD1a`: for any graph, even the empty one, `_:nnn rdf:type ddd` holds for each
    /// recognized `ddd` with a non-empty value space. Axiomatic; no premise. Added in
    /// RDF 1.2; it has no RDF 1.0 spelling.
    RdfD1a = "rdfD1a",
    /// `rdfD2`: `T(xxx, aaa, yyy)` ⇒ `aaa rdf:type rdf:Property`.
    ///
    /// Spelled `rdf1` in RDF 1.0 — note the crossed numbering. `rdf1` is accepted as an
    /// input alias.
    RdfD2 = "rdfD2" | "rdf1",

    // --- RDF 1.2 Semantics §9.2.1: patterns of RDFS entailment (15 rules). ---
    /// `rdfs1`: any IRI `aaa` among the recognized datatypes ⇒
    /// `aaa rdf:type rdfs:Datatype`.
    Rdfs1 = "rdfs1",
    /// `rdfs2`: `aaa rdfs:domain xxx`, `T(yyy, aaa, zzz)` ⇒ `yyy rdf:type xxx`.
    Rdfs2 = "rdfs2",
    /// `rdfs3`: `aaa rdfs:range xxx`, `T(yyy, aaa, zzz)` ⇒ `zzz rdf:type xxx`.
    Rdfs3 = "rdfs3",
    /// `rdfs4`: any triple in which `xxx` appears, in *any* position ⇒
    /// `xxx rdf:type rdfs:Resource`.
    ///
    /// RDF 1.2 merges RDF 1.0's subject-only `rdfs4a` and object-only `rdfs4b` into
    /// this one rule and widens it to every position. Both legacy spellings are
    /// accepted as input aliases and both widen to this variant, so neither can be
    /// round-tripped back to its RDF 1.0 name.
    Rdfs4 = "rdfs4" | "rdfs4a" | "rdfs4b",
    /// `rdfs5`: `xxx rdfs:subPropertyOf yyy`, `yyy rdfs:subPropertyOf zzz` ⇒
    /// `xxx rdfs:subPropertyOf zzz`.
    Rdfs5 = "rdfs5",
    /// `rdfs6`: `xxx rdf:type rdf:Property` ⇒ `xxx rdfs:subPropertyOf xxx`.
    Rdfs6 = "rdfs6",
    /// `rdfs7`: `aaa rdfs:subPropertyOf bbb`, `T(xxx, aaa, yyy)` ⇒
    /// `T(xxx, bbb, yyy)`.
    Rdfs7 = "rdfs7",
    /// `rdfs8`: `xxx rdf:type rdfs:Class` ⇒ `xxx rdfs:subClassOf rdfs:Resource`.
    Rdfs8 = "rdfs8",
    /// `rdfs9`: `xxx rdfs:subClassOf yyy`, `zzz rdf:type xxx` ⇒ `zzz rdf:type yyy`.
    Rdfs9 = "rdfs9",
    /// `rdfs10`: `xxx rdf:type rdfs:Class` ⇒ `xxx rdfs:subClassOf xxx`.
    Rdfs10 = "rdfs10",
    /// `rdfs11`: `xxx rdfs:subClassOf yyy`, `yyy rdfs:subClassOf zzz` ⇒
    /// `xxx rdfs:subClassOf zzz`.
    Rdfs11 = "rdfs11",
    /// `rdfs12`: `xxx rdf:type rdfs:ContainerMembershipProperty` ⇒
    /// `xxx rdfs:subPropertyOf rdfs:member`.
    Rdfs12 = "rdfs12",
    /// `rdfs13`: `xxx rdf:type rdfs:Datatype` ⇒ `xxx rdfs:subClassOf rdfs:Literal`.
    Rdfs13 = "rdfs13",
    /// `rdfs14`: any triple in which a triple term appears ⇒ that triple with the
    /// triple term replaced by a fresh `_:nnn`, plus `_:nnn rdf:type rdfs:Proposition`.
    /// Added in RDF 1.2 for triple terms.
    Rdfs14 = "rdfs14",
    /// `rdfs14a`: for any graph, even the empty one,
    /// `_:nnn rdf:type rdfs:Proposition` holds. Axiomatic; no premise. Added in
    /// RDF 1.2.
    Rdfs14a = "rdfs14a",

    // --- EXTENSIONS. Not a specification rule; see `RuleId::is_extension`. ---
    /// `ext-eq-diff-sym`: `?x owl:differentFrom ?y` ⇒ `?y owl:differentFrom ?x`.
    ///
    /// **Not one of the 78.** No rule of OWL 2 Profiles §4.3 Tables 4–9 has an
    /// `owl:differentFrom` head at all: the table's three `owl:differentFrom`
    /// rules (`eq-diff1`, `eq-diff2`, `eq-diff3`) read the property in a BODY and
    /// conclude `false`. This one concludes a triple, so it belongs to no table
    /// and is declared as an extension — [`is_extension`](Self::is_extension)
    /// answers `true` for it, [`extensions`] lists it, and neither [`rules`] nor
    /// [`implemented`] does.
    ///
    /// It is SOUND. `owl:differentFrom` denotes inequality of interpretations and
    /// `≠` is symmetric, so the symmetric triple holds in every model of the
    /// premise. And it reaches no `false` the normative table did not already
    /// reach: the only rules with `owl:differentFrom` in a body are `eq-diff1..3`,
    /// each of which pairs it with `owl:sameAs`, and `owl:sameAs` is already closed
    /// under `eq-sym` — so a clash on `(x, y)` was already a clash on `(y, x)`.
    ExtEqDiffSym = "ext-eq-diff-sym",
}

impl fmt::Display for RuleId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A string was not the canonical spelling of any known entailment rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseRuleIdError(String);

impl ParseRuleIdError {
    /// The unrecognized spelling, echoed back for diagnostics.
    #[must_use]
    pub fn unrecognized(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ParseRuleIdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown entailment rule id: {}", self.0)
    }
}

impl std::error::Error for ParseRuleIdError {}

// --- The specification rule tables, verbatim, in specification order. ---

/// OWL 2 Profiles §4.3 Table 4 — The Semantics of Equality.
const OWL_RL_EQUALITY: [RuleId; 9] = [
    RuleId::EqRef,
    RuleId::EqSym,
    RuleId::EqTrans,
    RuleId::EqRepS,
    RuleId::EqRepP,
    RuleId::EqRepO,
    RuleId::EqDiff1,
    RuleId::EqDiff2,
    RuleId::EqDiff3,
];

/// OWL 2 Profiles §4.3 Table 5 — The Semantics of Axioms about Properties.
const OWL_RL_PROPERTY_AXIOMS: [RuleId; 20] = [
    RuleId::PrpAp,
    RuleId::PrpDom,
    RuleId::PrpRng,
    RuleId::PrpFp,
    RuleId::PrpIfp,
    RuleId::PrpIrp,
    RuleId::PrpSymp,
    RuleId::PrpAsyp,
    RuleId::PrpTrp,
    RuleId::PrpSpo1,
    RuleId::PrpSpo2,
    RuleId::PrpEqp1,
    RuleId::PrpEqp2,
    RuleId::PrpPdw,
    RuleId::PrpAdp,
    RuleId::PrpInv1,
    RuleId::PrpInv2,
    RuleId::PrpKey,
    RuleId::PrpNpa1,
    RuleId::PrpNpa2,
];

/// OWL 2 Profiles §4.3 Table 6 — The Semantics of Classes.
const OWL_RL_CLASS_EXPRESSIONS: [RuleId; 19] = [
    RuleId::ClsThing,
    RuleId::ClsNothing1,
    RuleId::ClsNothing2,
    RuleId::ClsInt1,
    RuleId::ClsInt2,
    RuleId::ClsUni,
    RuleId::ClsCom,
    RuleId::ClsSvf1,
    RuleId::ClsSvf2,
    RuleId::ClsAvf,
    RuleId::ClsHv1,
    RuleId::ClsHv2,
    RuleId::ClsMaxc1,
    RuleId::ClsMaxc2,
    RuleId::ClsMaxqc1,
    RuleId::ClsMaxqc2,
    RuleId::ClsMaxqc3,
    RuleId::ClsMaxqc4,
    RuleId::ClsOo,
];

/// OWL 2 Profiles §4.3 Table 7 — The Semantics of Class Axioms.
const OWL_RL_CLASS_AXIOMS: [RuleId; 5] = [
    RuleId::CaxSco,
    RuleId::CaxEqc1,
    RuleId::CaxEqc2,
    RuleId::CaxDw,
    RuleId::CaxAdc,
];

/// OWL 2 Profiles §4.3 Table 8 — The Semantics of Datatypes.
const OWL_RL_DATATYPES: [RuleId; 5] = [
    RuleId::DtType1,
    RuleId::DtType2,
    RuleId::DtEq,
    RuleId::DtDiff,
    RuleId::DtNotType,
];

/// OWL 2 Profiles §4.3 Table 9 — The Semantics of Schema Vocabulary.
const OWL_RL_SCHEMA_VOCABULARY: [RuleId; 20] = [
    RuleId::ScmCls,
    RuleId::ScmSco,
    RuleId::ScmEqc1,
    RuleId::ScmEqc2,
    RuleId::ScmOp,
    RuleId::ScmDp,
    RuleId::ScmSpo,
    RuleId::ScmEqp1,
    RuleId::ScmEqp2,
    RuleId::ScmDom1,
    RuleId::ScmDom2,
    RuleId::ScmRng1,
    RuleId::ScmRng2,
    RuleId::ScmHv,
    RuleId::ScmSvf1,
    RuleId::ScmSvf2,
    RuleId::ScmAvf1,
    RuleId::ScmAvf2,
    RuleId::ScmInt,
    RuleId::ScmUni,
];

/// RDF 1.2 Semantics §8.1.1 — patterns of RDF entailment.
const RDF_PATTERNS: [RuleId; 3] = [RuleId::RdfD1, RuleId::RdfD1a, RuleId::RdfD2];

/// RDF 1.2 Semantics §9.2.1 — patterns of RDFS entailment.
const RDFS_PATTERNS: [RuleId; 15] = [
    RuleId::Rdfs1,
    RuleId::Rdfs2,
    RuleId::Rdfs3,
    RuleId::Rdfs4,
    RuleId::Rdfs5,
    RuleId::Rdfs6,
    RuleId::Rdfs7,
    RuleId::Rdfs8,
    RuleId::Rdfs9,
    RuleId::Rdfs10,
    RuleId::Rdfs11,
    RuleId::Rdfs12,
    RuleId::Rdfs13,
    RuleId::Rdfs14,
    RuleId::Rdfs14a,
];

/// Splice the six OWL 2 RL tables into one array, preserving table order.
///
/// The per-table array lengths (9 + 20 + 19 + 5 + 5 + 20) are part of their types, so
/// a dropped or duplicated rule fails to compile here rather than silently shrinking
/// the inventory.
const fn splice_owl_rl() -> [RuleId; 78] {
    let tables: [&[RuleId]; 6] = [
        &OWL_RL_EQUALITY,
        &OWL_RL_PROPERTY_AXIOMS,
        &OWL_RL_CLASS_EXPRESSIONS,
        &OWL_RL_CLASS_AXIOMS,
        &OWL_RL_DATATYPES,
        &OWL_RL_SCHEMA_VOCABULARY,
    ];
    let mut out = [RuleId::EqRef; 78];
    let mut written = 0;
    let mut t = 0;
    while t < tables.len() {
        let table = tables[t];
        let mut j = 0;
        while j < table.len() {
            out[written] = table[j];
            written += 1;
            j += 1;
        }
        t += 1;
    }
    assert!(
        written == 78,
        "OWL 2 RL Tables 4-9 must contribute 78 rules"
    );
    out
}

/// Splice the RDF and RDFS pattern tables into one array, RDF first.
const fn splice_rdf_rdfs() -> [RuleId; 18] {
    let mut out = [RuleId::RdfD1; 18];
    let mut written = 0;
    let mut j = 0;
    while j < RDF_PATTERNS.len() {
        out[written] = RDF_PATTERNS[j];
        written += 1;
        j += 1;
    }
    j = 0;
    while j < RDFS_PATTERNS.len() {
        out[written] = RDFS_PATTERNS[j];
        written += 1;
        j += 1;
    }
    assert!(
        written == 18,
        "the RDF and RDFS tables must contribute 18 rules"
    );
    out
}

/// All 78 OWL 2 RL/RDF rules, Table 4 through Table 9.
static OWL_RL_RULES: [RuleId; 78] = splice_owl_rl();

/// All 18 RDF + RDFS entailment patterns; RDFS entailment subsumes RDF entailment.
static RDF_AND_RDFS_RULES: [RuleId; 18] = splice_rdf_rdfs();

/// No rules: `Simple` entailment is the identity closure, and `OWL-Direct` / `RIF` /
/// `D` are not defined by a fixed rule table this crate can enumerate.
static NO_RULES: [RuleId; 0] = [];

// --- What the declared calculus in `calculus.rs` actually fires today. ---

/// The RDF patterns the bare-`RDF` lane evaluates: all three of RDF 1.2 Semantics §8.1.1.
///
/// `rdfD1` and `rdfD1a` conclude about a FRESH blank node, which is an existential head the
/// semi-naive evaluator has no semantics for — so this lane is evaluated by
/// `purrdf-datalog`'s restricted chase instead, which mints each surrogate as a
/// frontier-addressed Skolem witness. The surrogates do not reach the ANSWER, and that is a
/// requirement of SPARQL's entailment regimes rather than a shortcut; see
/// [`Construct::Surrogate`](crate::Construct::Surrogate).
static IMPLEMENTED_RDF: [RuleId; 3] = [RuleId::RdfD1, RuleId::RdfD1a, RuleId::RdfD2];

/// The RDF and RDFS patterns the `RDFS` lane evaluates: all eighteen.
///
/// The §8.1.1 patterns head the list because RDFS entailment subsumes RDF entailment and
/// [`rules`] has always listed them first. Four of the eighteen — `rdfD1`, `rdfD1a`,
/// `rdfs14`, `rdfs14a` — conclude about a FRESH blank node, which is an existentially
/// quantified head the semi-naive evaluator has no semantics for, so this lane is
/// evaluated by `purrdf-datalog`'s restricted chase: each surrogate is a
/// frontier-addressed Skolem witness, re-deriving an obligation reuses its witness, and the
/// clause set's termination is COMPUTED from the position dependency graph rather than
/// assumed.
///
/// The surrogates do not reach the ANSWER. A SPARQL entailment regime draws its answers
/// from the scoping graph and a surrogate is not in it, so every conclusion mentioning one
/// is dropped at the materialization boundary and reported as the
/// [`Construct::Surrogate`](crate::Construct::Surrogate) boundary. That is why claiming
/// these four here is honest and not an overclaim: they FIRE, exactly as `dt-type2`,
/// `dt-eq` and `dt-diff` fire while every one of their conclusions is unrepresentable.
static IMPLEMENTED_RDFS: [RuleId; 18] = [
    RuleId::RdfD1,
    RuleId::RdfD1a,
    RuleId::RdfD2,
    RuleId::Rdfs1,
    RuleId::Rdfs2,
    RuleId::Rdfs3,
    RuleId::Rdfs4,
    RuleId::Rdfs5,
    RuleId::Rdfs6,
    RuleId::Rdfs7,
    RuleId::Rdfs8,
    RuleId::Rdfs9,
    RuleId::Rdfs10,
    RuleId::Rdfs11,
    RuleId::Rdfs12,
    RuleId::Rdfs13,
    RuleId::Rdfs14,
    RuleId::Rdfs14a,
];

/// The OWL 2 RL rules the chase evaluates: all seventy-eight, in specification table
/// order.
///
/// The list is the whole of Tables 4–9. Seventeen of its entries conclude `false` rather
/// than a triple — `eq-diff1`, `eq-diff2`, `eq-diff3`, `prp-irp`, `prp-asyp`, `prp-pdw`,
/// `prp-adp`, `prp-npa1`, `prp-npa2`, `cls-nothing2`, `cls-com`, `cls-maxc1`,
/// `cls-maxqc1`, `cls-maxqc2`, `cax-dw`, `cax-adc` and `dt-not-type` — and they are
/// claimed here because a body match on one of them is DETECTED: the calculus lowers each
/// to a clause the evaluator runs ([`crate::calculus::constraint_clause`]) and a match
/// becomes [`EntailError::Inconsistent`](crate::EntailError) carrying a witness. "Fires"
/// for such a rule means "is decided", which is the only thing a rule with no conclusion
/// can do.
static IMPLEMENTED_OWL_RL: [RuleId; 78] = splice_owl_rl();

/// The rules the `D` lane evaluates: OWL 2 Profiles §4.3 Table 8, entire.
static IMPLEMENTED_D: [RuleId; 5] = OWL_RL_DATATYPES;

/// The rules the `OWL-RL` lane fires that no specification table states.
///
/// One, and its whole content is symmetry of `owl:differentFrom`. See
/// [`RuleId::ExtEqDiffSym`] for the soundness argument and [`extensions`] for why the
/// list is published separately rather than folded into [`implemented`].
static EXTENSIONS_OWL_RL: [RuleId; 1] = [RuleId::ExtEqDiffSym];

/// The rule table `regime` is *defined by* — what the specification requires of a
/// complete implementation, not what this crate currently does.
///
/// The slice is `&'static` and in specification table order (OWL 2 RL Tables 4–9; RDF
/// patterns before RDFS patterns), so the inventory is byte-for-byte reproducible.
///
/// * `Simple` — the identity closure; no rules.
/// * `Rdf` — RDF 1.2 Semantics §8.1.1: `rdfD1`, `rdfD1a`, `rdfD2`.
/// * `Rdfs` — RDFS entailment subsumes RDF entailment, so both tables: 18 rules.
/// * `OwlRl` — OWL 2 Profiles §4.3 Tables 4–9: 78 rules. Note that OWL 2 RL/RDF
///   deliberately omits the RDF/RDFS axiomatic triples and "most, but not all of the
///   entailment rules of RDFS", so this list is *not* a superset of the RDFS table.
/// * `D` — datatype entailment, realized as OWL 2 Profiles §4.3 Table 8: the five `dt-*`
///   rules, which are the fixed rule table this crate can enumerate for it. What Table 8
///   does not cover — the infinite value spaces themselves — is reported as the
///   [`Construct::DatatypeValueSpace`](crate::Construct::DatatypeValueSpace) boundary
///   rather than claimed.
/// * `OwlDirect`, `Rif` — not defined by a fixed rule table (a tableau and a
///   caller-supplied rule set respectively); empty.
///
/// ```
/// use purrdf_entail::{Regime, rules};
///
/// assert_eq!(rules(Regime::OwlRl).len(), 78);
/// assert_eq!(rules(Regime::Rdfs).len(), 18);
/// assert_eq!(rules(Regime::D).len(), 5);
/// assert!(rules(Regime::Simple).is_empty());
/// ```
#[must_use]
pub fn rules(regime: Regime) -> &'static [RuleId] {
    match regime {
        Regime::Rdf => &RDF_PATTERNS,
        Regime::Rdfs => &RDF_AND_RDFS_RULES,
        Regime::OwlRl => &OWL_RL_RULES,
        Regime::D => &OWL_RL_DATATYPES,
        Regime::Simple | Regime::OwlDirect | Regime::Rif => &NO_RULES,
    }
}

/// The subset of [`rules`] this crate's engines actually fire today.
///
/// Always a subsequence of `rules(regime)` — same order, no additions — so
/// `rules(r).len() - implemented(r).len()` is the regime's measurable gap.
///
/// Three honesty notes, so this list is read for exactly what it claims:
///
/// * It lists rules the chase *evaluates directly*, which is a stronger claim than "the
///   closure contains what this rule would conclude": several rules are redundant given
///   others (`cax-eqc1` also follows from `scm-eqc1` then `cax-sco`, one round later), and
///   listing a rule this crate does not run because something else reaches its conclusions
///   would make a report credit a rule the caller's ontology never used.
/// * Under `OwlRl` the chase additionally fires the RDFS-shaped rules `rdfs6`,
///   `rdfs8`, and `rdfs10`. Those are not OWL 2 RL rule ids — OWL 2 RL/RDF omits them
///   from its tables — so they cannot appear in an `OwlRl` list that must stay a subset
///   of the 78, but an `OwlRl` report does name them, under their RDFS ids.
/// * For the seventeen rules whose conclusion is `false`, "fires" means DECIDES: a body
///   match is [`EntailError::Inconsistent`](crate::EntailError) carrying a witness rather
///   than a triple in the closure. That is the only thing a rule with no conclusion can
///   do, and it is what makes claiming them here honest.
///
/// A complete rule table is NOT a complete closure, and the two are reported separately:
/// see [`Completeness::ExactWithinBoundaries`](crate::Completeness::ExactWithinBoundaries).
///
/// ```
/// use purrdf_entail::{Regime, RuleId, implemented, rules};
///
/// // The gap is data, not prose — and for OWL 2 RL there is none left.
/// let missing: Vec<RuleId> = rules(Regime::OwlRl)
///     .iter()
///     .copied()
///     .filter(|r| !implemented(Regime::OwlRl).contains(r))
///     .collect();
/// assert!(missing.is_empty());
/// // `cax-dw` concludes `false`; it is claimed because a body match on it is DECIDED and
/// // reported as an inconsistency witness.
/// assert!(implemented(Regime::OwlRl).contains(&RuleId::CaxDw));
///
/// // RDFS has none either: the four rules that mint a fresh blank node are evaluated by
/// // `purrdf-datalog`'s restricted chase rather than by the semi-naive evaluator.
/// let missing: Vec<RuleId> = rules(Regime::Rdfs)
///     .iter()
///     .copied()
///     .filter(|r| !implemented(Regime::Rdfs).contains(r))
///     .collect();
/// assert!(missing.is_empty());
/// assert!(implemented(Regime::Rdfs).contains(&RuleId::RdfD1));
/// ```
#[must_use]
pub fn implemented(regime: Regime) -> &'static [RuleId] {
    match regime {
        Regime::Rdf => &IMPLEMENTED_RDF,
        Regime::Rdfs => &IMPLEMENTED_RDFS,
        Regime::OwlRl => &IMPLEMENTED_OWL_RL,
        Regime::D => &IMPLEMENTED_D,
        Regime::Simple | Regime::OwlDirect | Regime::Rif => &NO_RULES,
    }
}

/// The rules `regime`'s lane fires that NO specification table states.
///
/// The third inventory, and the one that keeps the other two honest. [`rules`] is what a
/// specification defines the regime by and [`implemented`] is the subset this crate
/// evaluates, so `rules(r).len()` and `implemented(r).len()` are both statements about a
/// normative table — `OWL-RL` is 78 and 78, and stays 78 and 78 however far this list
/// grows. Everything the chase fires BEYOND that table is here, and nowhere else.
///
/// Publishing it separately rather than widening [`implemented`] is the whole point: a
/// caller that wants strictly normative behaviour needs to be able to tell, from data,
/// which conclusions its closure owes to a rule W3C wrote and which to a rule PurRDF
/// added. Folding the two together would make `78 / 78` a number about a mixed set.
///
/// Every entry answers [`RuleId::is_extension`] with `true` and appears in neither
/// [`rules`] nor [`implemented`], for every regime. The slice is `&'static` and in
/// declaration order, like the other two.
///
/// * `OwlRl` — one: `ext-eq-diff-sym`, symmetry of `owl:differentFrom`. It is sound and
///   shaped exactly like `prp-symp`, and it is not in Tables 4–9 because the table's only
///   `owl:differentFrom` rules conclude `false`. See [`RuleId::ExtEqDiffSym`].
/// * every other regime — none. Extending a lane is a decision taken per lane, and no
///   other lane has one taken for it.
///
/// ```
/// use purrdf_entail::{Regime, RuleId, extensions, implemented, rules};
///
/// // The normative statement is unmoved: the OWL 2 RL table is 78 rules, all fired.
/// assert_eq!(rules(Regime::OwlRl).len(), 78);
/// assert_eq!(implemented(Regime::OwlRl).len(), 78);
/// // And what the lane adds beyond it is one named rule, stated as data.
/// assert_eq!(extensions(Regime::OwlRl), &[RuleId::ExtEqDiffSym]);
/// assert!(extensions(Regime::Rdfs).is_empty());
/// ```
#[must_use]
pub fn extensions(regime: Regime) -> &'static [RuleId] {
    match regime {
        Regime::OwlRl => &EXTENSIONS_OWL_RL,
        Regime::Simple
        | Regime::Rdf
        | Regime::Rdfs
        | Regime::OwlDirect
        | Regime::Rif
        | Regime::D => &NO_RULES,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        EXTENSIONS_OWL_RL, IMPLEMENTED_D, IMPLEMENTED_OWL_RL, IMPLEMENTED_RDF, IMPLEMENTED_RDFS,
        OWL_RL_CLASS_AXIOMS, OWL_RL_CLASS_EXPRESSIONS, OWL_RL_DATATYPES, OWL_RL_EQUALITY,
        OWL_RL_PROPERTY_AXIOMS, OWL_RL_SCHEMA_VOCABULARY, RDF_PATTERNS, RDFS_PATTERNS, RuleId,
        extensions, implemented, rules,
    };
    use crate::Regime;
    use std::collections::BTreeSet;

    /// Every regime, for the cross-cutting invariants. The exhaustive match in
    /// [`regime_name`] is what forces this list to be revisited when `Regime` grows.
    const ALL_REGIMES: [Regime; 7] = [
        Regime::Simple,
        Regime::Rdf,
        Regime::Rdfs,
        Regime::OwlRl,
        Regime::OwlDirect,
        Regime::Rif,
        Regime::D,
    ];

    /// Exhaustive over `Regime`: adding a variant fails to compile here, which is the
    /// signal to extend [`ALL_REGIMES`], [`rules`], and [`implemented`].
    fn regime_name(regime: Regime) -> &'static str {
        match regime {
            Regime::Simple => "Simple",
            Regime::Rdf => "RDF",
            Regime::Rdfs => "RDFS",
            Regime::OwlRl => "OWL-RL",
            Regime::OwlDirect => "OWL-Direct",
            Regime::Rif => "RIF",
            Regime::D => "D",
        }
    }

    /// An independent, literal transcription of OWL 2 Profiles §4.3 Tables 4–9. This
    /// is deliberately *not* derived from the tables in the module above: it is a
    /// second reading of the specification, and set equality between the two is the
    /// gate. Reading order is Table 4, 5, 6, 7, 8, 9.
    const SPEC_RL_78: [&str; 78] = [
        // Table 4 — equality (9).
        "eq-ref",
        "eq-sym",
        "eq-trans",
        "eq-rep-s",
        "eq-rep-p",
        "eq-rep-o",
        "eq-diff1",
        "eq-diff2",
        "eq-diff3",
        // Table 5 — axioms about properties (20).
        "prp-ap",
        "prp-dom",
        "prp-rng",
        "prp-fp",
        "prp-ifp",
        "prp-irp",
        "prp-symp",
        "prp-asyp",
        "prp-trp",
        "prp-spo1",
        "prp-spo2",
        "prp-eqp1",
        "prp-eqp2",
        "prp-pdw",
        "prp-adp",
        "prp-inv1",
        "prp-inv2",
        "prp-key",
        "prp-npa1",
        "prp-npa2",
        // Table 6 — classes (19).
        "cls-thing",
        "cls-nothing1",
        "cls-nothing2",
        "cls-int1",
        "cls-int2",
        "cls-uni",
        "cls-com",
        "cls-svf1",
        "cls-svf2",
        "cls-avf",
        "cls-hv1",
        "cls-hv2",
        "cls-maxc1",
        "cls-maxc2",
        "cls-maxqc1",
        "cls-maxqc2",
        "cls-maxqc3",
        "cls-maxqc4",
        "cls-oo",
        // Table 7 — class axioms (5).
        "cax-sco",
        "cax-eqc1",
        "cax-eqc2",
        "cax-dw",
        "cax-adc",
        // Table 8 — datatypes (5).
        "dt-type1",
        "dt-type2",
        "dt-eq",
        "dt-diff",
        "dt-not-type",
        // Table 9 — schema vocabulary (20).
        "scm-cls",
        "scm-sco",
        "scm-eqc1",
        "scm-eqc2",
        "scm-op",
        "scm-dp",
        "scm-spo",
        "scm-eqp1",
        "scm-eqp2",
        "scm-dom1",
        "scm-dom2",
        "scm-rng1",
        "scm-rng2",
        "scm-hv",
        "scm-svf1",
        "scm-svf2",
        "scm-avf1",
        "scm-avf2",
        "scm-int",
        "scm-uni",
    ];

    /// An independent, literal transcription of the RDF (§8.1.1) and RDFS (§9.2.1)
    /// entailment patterns of RDF 1.2 Semantics, in the order the tables present them,
    /// in the canonical spellings only — no superseded aliases.
    const SPEC_RDF_RDFS_18: [&str; 18] = [
        "rdfD1", "rdfD1a", "rdfD2", "rdfs1", "rdfs2", "rdfs3", "rdfs4", "rdfs5", "rdfs6", "rdfs7",
        "rdfs8", "rdfs9", "rdfs10", "rdfs11", "rdfs12", "rdfs13", "rdfs14", "rdfs14a",
    ];

    /// Every superseded spelling [`FromStr`] accepts, and the variant it must resolve
    /// to. The RDF pair is numbered *crossed* (`rdf1` is `rdfD2`, `rdf2` is `rdfD1`),
    /// which is the whole reason this is pinned rather than assumed.
    const LEGACY_ALIASES: [(&str, RuleId); 4] = [
        ("rdf1", RuleId::RdfD2),
        ("rdf2", RuleId::RdfD1),
        ("rdfs4a", RuleId::Rdfs4),
        ("rdfs4b", RuleId::Rdfs4),
    ];

    fn parse_all(spellings: &[&str]) -> BTreeSet<RuleId> {
        spellings
            .iter()
            .map(|s| s.parse::<RuleId>().expect("spec spelling parses"))
            .collect()
    }

    #[test]
    fn spec_rl_78_equals_the_owl_rl_rule_set() {
        assert_eq!(
            SPEC_RL_78.len(),
            78,
            "the literal transcription must hold 78 entries"
        );
        let transcribed = parse_all(&SPEC_RL_78);
        assert_eq!(
            transcribed.len(),
            78,
            "the literal transcription must hold 78 *distinct* rules"
        );
        let listed: BTreeSet<RuleId> = rules(Regime::OwlRl).iter().copied().collect();
        assert_eq!(
            transcribed, listed,
            "rules(OwlRl) must equal OWL 2 Profiles §4.3 Tables 4-9 exactly"
        );
    }

    #[test]
    fn spec_rdf_and_rdfs_18_equals_the_rdfs_rule_set() {
        let transcribed = parse_all(&SPEC_RDF_RDFS_18);
        assert_eq!(transcribed.len(), 18, "18 distinct RDF/RDFS patterns");
        let listed: BTreeSet<RuleId> = rules(Regime::Rdfs).iter().copied().collect();
        assert_eq!(transcribed, listed, "rules(Rdfs) must equal RDF + RDFS");
        let rdf_only: BTreeSet<RuleId> = rules(Regime::Rdf).iter().copied().collect();
        assert_eq!(rdf_only, parse_all(&["rdfD1", "rdfD1a", "rdfD2"]));
        assert!(
            rdf_only.is_subset(&listed),
            "RDFS entailment subsumes RDF entailment"
        );
        // The transcription is canonical-only: no superseded spelling may appear in it.
        for (legacy, _) in LEGACY_ALIASES {
            assert!(
                !SPEC_RDF_RDFS_18.contains(&legacy),
                "{legacy} is a superseded spelling and must not be transcribed as spec"
            );
        }
    }

    #[test]
    fn legacy_spellings_parse_to_their_current_variant() {
        for (legacy, expected) in LEGACY_ALIASES {
            assert_eq!(
                legacy.parse::<RuleId>().expect("legacy alias is accepted"),
                expected,
                "alias {legacy}"
            );
            // Input-only: the canonical direction never answers with a legacy name.
            assert_ne!(
                expected.as_str(),
                legacy,
                "as_str must not emit the superseded spelling {legacy}"
            );
        }
        // The crossed RDF numbering, pinned explicitly — this is the trap.
        assert_eq!(
            "rdf1".parse::<RuleId>().expect("rdf1"),
            "rdfD2".parse::<RuleId>().expect("rdfD2"),
        );
        assert_eq!(
            "rdf2".parse::<RuleId>().expect("rdf2"),
            "rdfD1".parse::<RuleId>().expect("rdfD1"),
        );
        assert_ne!(
            "rdf1".parse::<RuleId>().expect("rdf1"),
            "rdfD1".parse::<RuleId>().expect("rdfD1"),
            "rdf1 is NOT rdfD1",
        );
        // Both halves of the retired rdfs4a/rdfs4b split widen to the merged rdfs4.
        assert_eq!(
            "rdfs4a".parse::<RuleId>().expect("rdfs4a"),
            "rdfs4b".parse::<RuleId>().expect("rdfs4b"),
        );
    }

    #[test]
    fn each_owl_rl_table_has_its_specified_size() {
        // Asserted per table so a transcription slip names the table it came from.
        assert_eq!(OWL_RL_EQUALITY.len(), 9, "Table 4: equality");
        assert_eq!(
            OWL_RL_PROPERTY_AXIOMS.len(),
            20,
            "Table 5: axioms about properties"
        );
        assert_eq!(OWL_RL_CLASS_EXPRESSIONS.len(), 19, "Table 6: classes");
        assert_eq!(OWL_RL_CLASS_AXIOMS.len(), 5, "Table 7: class axioms");
        assert_eq!(OWL_RL_DATATYPES.len(), 5, "Table 8: datatypes");
        assert_eq!(
            OWL_RL_SCHEMA_VOCABULARY.len(),
            20,
            "Table 9: schema vocabulary"
        );
        assert_eq!(RDF_PATTERNS.len(), 3, "RDF 1.2 Semantics §8.1.1");
        assert_eq!(RDFS_PATTERNS.len(), 15, "RDF 1.2 Semantics §9.2.1");
    }

    #[test]
    fn owl_rl_rule_list_is_the_six_tables_concatenated_in_order() {
        let mut expected: Vec<RuleId> = Vec::with_capacity(78);
        expected.extend_from_slice(&OWL_RL_EQUALITY);
        expected.extend_from_slice(&OWL_RL_PROPERTY_AXIOMS);
        expected.extend_from_slice(&OWL_RL_CLASS_EXPRESSIONS);
        expected.extend_from_slice(&OWL_RL_CLASS_AXIOMS);
        expected.extend_from_slice(&OWL_RL_DATATYPES);
        expected.extend_from_slice(&OWL_RL_SCHEMA_VOCABULARY);
        assert_eq!(rules(Regime::OwlRl), expected.as_slice());
    }

    #[test]
    fn as_str_and_from_str_round_trip_for_every_variant() {
        assert_eq!(
            RuleId::ALL.len(),
            97,
            "78 OWL 2 RL + 3 RDF + 15 RDFS + 1 extension"
        );
        for &id in RuleId::ALL {
            let spelling = id.as_str();
            assert_eq!(
                spelling
                    .parse::<RuleId>()
                    .expect("canonical spelling parses"),
                id,
                "round trip for {id:?}"
            );
            assert_eq!(id.to_string(), spelling, "Display matches as_str");
        }
    }

    #[test]
    fn every_spelling_is_unique() {
        let spellings: BTreeSet<&str> = RuleId::ALL.iter().map(|r| r.as_str()).collect();
        assert_eq!(
            spellings.len(),
            RuleId::ALL.len(),
            "two variants share a spelling"
        );
        let ids: BTreeSet<RuleId> = RuleId::ALL.iter().copied().collect();
        assert_eq!(ids.len(), RuleId::ALL.len(), "ALL repeats a variant");
    }

    #[test]
    fn unknown_spellings_are_rejected_with_the_offending_text() {
        // Near-misses that must not parse: wrong case, wrong separator, plausible
        // inventions next to real ids (`rdfs14`, `rdfD1a` and `cls-maxqc1` all exist,
        // so these neighbours are the interesting negatives), and the empty string.
        for bad in [
            "Eq-Ref",
            "eq_ref",
            "rdfs15",
            "rdfs4c",
            "rdfD3",
            "cls-minqc1",
            "",
        ] {
            let err = bad.parse::<RuleId>().expect_err("must not parse");
            assert_eq!(err.unrecognized(), bad);
            assert!(err.to_string().contains(bad) || bad.is_empty());
        }
    }

    #[test]
    fn implemented_is_a_subsequence_of_the_spec_rules() {
        for regime in ALL_REGIMES {
            let spec = rules(regime);
            let done = implemented(regime);
            // Subsequence, which is subset plus specification-table ordering.
            let mut cursor = spec.iter();
            for id in done {
                assert!(
                    cursor.any(|s| s == id),
                    "{} lists {id} as implemented, but it is absent from (or out of \
                     order in) the spec rule table",
                    regime_name(regime)
                );
            }
        }
    }

    #[test]
    fn rule_lists_are_duplicate_free() {
        for regime in ALL_REGIMES {
            for (label, list) in [
                ("rules", rules(regime)),
                ("implemented", implemented(regime)),
            ] {
                let unique: BTreeSet<RuleId> = list.iter().copied().collect();
                assert_eq!(
                    unique.len(),
                    list.len(),
                    "{label}({}) repeats a rule",
                    regime_name(regime)
                );
            }
        }
    }

    #[test]
    fn current_gap_is_pinned_as_a_ratchet() {
        // A ratchet, not a drift guard: when a later task teaches the chase a new
        // rule, this test FAILS and must be updated deliberately, in the same commit
        // that adds the rule. Never widen it to an inequality.
        let counts: Vec<(&str, usize, usize)> = ALL_REGIMES
            .iter()
            .map(|&r| (regime_name(r), rules(r).len(), implemented(r).len()))
            .collect();
        assert_eq!(
            counts,
            vec![
                ("Simple", 0, 0),
                ("RDF", 3, 3),
                ("RDFS", 18, 18),
                ("OWL-RL", 78, 78),
                ("OWL-Direct", 0, 0),
                ("RIF", 0, 0),
                ("D", 5, 5),
            ],
            "(regime, spec rules, implemented rules)"
        );
    }

    #[test]
    fn implemented_lists_are_pinned_exactly() {
        // The counts above catch a size change; these catch a swap that keeps the
        // count. Both must be updated together when the chase learns a rule.
        assert_eq!(implemented(Regime::Simple), &[] as &[RuleId]);
        assert_eq!(implemented(Regime::Rdf), &IMPLEMENTED_RDF);
        assert_eq!(implemented(Regime::Rdfs), &IMPLEMENTED_RDFS);
        assert_eq!(implemented(Regime::OwlRl), &IMPLEMENTED_OWL_RL);

        assert_eq!(implemented(Regime::D), &IMPLEMENTED_D);

        // `OWL-RL` is COMPLETE: the chase evaluates the whole of Tables 4-9, so the
        // implemented list IS the specification list — asserted as an identity rather than
        // as a copied transcription, because a copy of seventy-eight names would only
        // repeat what `rules` already says.
        assert_eq!(implemented(Regime::OwlRl), rules(Regime::OwlRl));
        // The seventeen rules of those tables whose conclusion is `false`. They are
        // claimed here because a body match on one of them is DETECTED and reported as an
        // inconsistency witness, which is the only thing a rule with no conclusion can do;
        // see `crate::calculus::constraint_clause`.
        for decided in [
            RuleId::EqDiff1,
            RuleId::EqDiff2,
            RuleId::EqDiff3,
            RuleId::PrpIrp,
            RuleId::PrpAsyp,
            RuleId::PrpPdw,
            RuleId::PrpAdp,
            RuleId::PrpNpa1,
            RuleId::PrpNpa2,
            RuleId::ClsNothing2,
            RuleId::ClsCom,
            RuleId::ClsMaxc1,
            RuleId::ClsMaxqc1,
            RuleId::ClsMaxqc2,
            RuleId::CaxDw,
            RuleId::CaxAdc,
            RuleId::DtNotType,
        ] {
            assert!(
                implemented(Regime::OwlRl).contains(&decided),
                "{decided} concludes `false` and is DECIDED, so it is implemented"
            );
        }
        let rdfs: Vec<&str> = implemented(Regime::Rdfs)
            .iter()
            .map(|r| r.as_str())
            .collect();
        assert_eq!(
            rdfs,
            [
                "rdfD1", "rdfD1a", "rdfD2", "rdfs1", "rdfs2", "rdfs3", "rdfs4", "rdfs5", "rdfs6",
                "rdfs7", "rdfs8", "rdfs9", "rdfs10", "rdfs11", "rdfs12", "rdfs13", "rdfs14",
                "rdfs14a",
            ]
        );
        // The RDFS lane is COMPLETE over its own eighteen patterns.
        assert_eq!(implemented(Regime::Rdfs), rules(Regime::Rdfs));
        // …and the bare-RDF regime over its three.
        let rdf: Vec<&str> = implemented(Regime::Rdf)
            .iter()
            .map(|r| r.as_str())
            .collect();
        assert_eq!(rdf, ["rdfD1", "rdfD1a", "rdfD2"]);
    }

    #[test]
    fn rule_lists_are_stable_across_calls() {
        // `&'static` slices from statics: the same pointer every time, so nothing here
        // can depend on hash or map iteration order.
        for regime in ALL_REGIMES {
            assert!(std::ptr::eq(rules(regime), rules(regime)));
            assert!(std::ptr::eq(implemented(regime), implemented(regime)));
            assert!(std::ptr::eq(extensions(regime), extensions(regime)));
        }
    }

    /// THE `ext-` PREFIX IS DISJOINT FROM EVERY SPECIFICATION SPELLING.
    ///
    /// [`RuleId::is_extension`] reads the canonical spelling, which is only sound if no
    /// published rule name begins `ext-`. It does not — OWL 2 Profiles §4.3 uses `eq-`,
    /// `prp-`, `cls-`, `cax-`, `dt-` and `scm-`, and RDF 1.2 Semantics uses `rdfD` and
    /// `rdfs` — and this is what keeps that true as the enum grows: a normative rule added
    /// with an `ext-` spelling, or an extension added without one, fails here.
    #[test]
    fn the_extension_prefix_is_disjoint_from_the_specification_names() {
        let tabled: BTreeSet<RuleId> = ALL_REGIMES
            .into_iter()
            .flat_map(|regime| rules(regime).iter().copied())
            .collect();
        for &rule in RuleId::ALL {
            let spelled = rule.as_str().starts_with("ext-");
            assert_eq!(
                rule.is_extension(),
                spelled,
                "{rule}: is_extension disagrees with the spelling"
            );
            assert_eq!(
                rule.is_extension(),
                !tabled.contains(&rule),
                "{rule}: an extension is exactly a rule no specification table names"
            );
        }
    }

    /// The extension inventory, pinned exactly, and pinned to be DISJOINT from the other
    /// two — which is the property that keeps `OWL-RL 78 / 78` a claim about Tables 4–9.
    #[test]
    fn extension_lists_are_pinned_exactly() {
        assert_eq!(extensions(Regime::OwlRl), &EXTENSIONS_OWL_RL);
        assert_eq!(extensions(Regime::OwlRl), &[RuleId::ExtEqDiffSym]);
        for regime in ALL_REGIMES {
            if matches!(regime, Regime::OwlRl) {
                continue;
            }
            assert!(
                extensions(regime).is_empty(),
                "{}: no extension is declared for this lane",
                regime_name(regime)
            );
        }
        // The three inventories are pairwise disjoint, for every regime.
        for regime in ALL_REGIMES {
            for extension in extensions(regime) {
                assert!(!rules(regime).contains(extension));
                assert!(!implemented(regime).contains(extension));
            }
        }
        // And the sizes the whole project quotes are unmoved by the extension.
        assert_eq!(rules(Regime::OwlRl).len(), 78);
        assert_eq!(implemented(Regime::OwlRl).len(), 78);
    }
}
