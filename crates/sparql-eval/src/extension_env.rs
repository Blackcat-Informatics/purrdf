// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The environment a SPARQL text must be interpreted relative to — as one value,
//! with one identity, computed once.
//!
//! # Why this type exists
//!
//! Whether a triple pattern is a *data edge* or a *call* is not a property of the
//! text. `?s <https://example.org/rel/near> ?o` is a perfectly legal triple pattern
//! and a perfectly legal relation invocation, and only configuration decides which.
//! The same is true of an extension function's namespace, of a `Custom` aggregate's
//! admission, and of which registry a call resolves against once lowered. Those four
//! answers together are what a SPARQL text means, and until this type existed they
//! were four separate arguments that every consumer re-assembled by hand.
//!
//! They were not assembled the same way twice. [`crate::engine::QueryOptions`]
//! carried three registries but not the parser options, which lived on the engine.
//! The plan-cache key carried the parser options and two registry fingerprints but
//! not the function registry. `check_plan_matches_relations` carried two.
//! `purrdf-shapes` pulled three out of three thread-locals and never configured the
//! fourth at all. A prepared shapes product had an identity row for each registry
//! and none for the parser options. And a `sh:SPARQLFunction` body was parsed with
//! none of them, which is the defect that occasioned this module: a registered
//! relation IRI in a function body lowered to an ordinary triple pattern, matched
//! nothing, and reported conformance.
//!
//! Six consumers, six arities, one class of bug — "door X forgot registry Y" — that
//! can only exist while there is a Y to forget *separately from* X's other
//! arguments. An [`ExtensionEnv`] removes the separateness. A door either has the
//! environment or it does not compile.
//!
//! # What it holds, and what it deliberately does not
//!
//! It holds the configuration that decides **how text becomes algebra**: the base
//! [`ParserOptions`], the [`PropertyFunctionRegistry`] a lowered call resolves
//! against, and the [`AggregateRegistry`] a `Custom` aggregate is admitted against.
//!
//! It does **not** hold the user-function registry, and that is a factoring
//! decision rather than an omission. A SPARQL-bodied user function's body is itself
//! a SPARQL text that must be interpreted relative to an environment, so the
//! function registry is a *consumer* of an environment, not a component of one.
//! Making it a field would be circular: binding a function body needs an
//! environment, and the environment would need the bound functions. Instead
//! [`crate::user_fn::UserFunctionRegistry`] is bound *against* an `ExtensionEnv` and
//! the result carries this environment's identity, so the pairing is checkable and
//! the cycle never forms.
//!
//! Nor does it hold per-call state — the SHACL pre-binding mode, a blank-node mint
//! prefix, a focus graph, a call depth. Those vary per invocation while the
//! environment is fixed for a whole validation, and keeping the two apart is what
//! makes "compute this once" a safe thing to say.
//!
//! # Computed once
//!
//! The effective [`ParserOptions`] and both instance-tier registry fingerprints are
//! derived in [`ExtensionEnv::new`]; the content digest is deferred to first demand,
//! because only a caller crossing a process boundary ever asks for it. That is not
//! merely tidy. `PlanCache`'s
//! `prepare_with_relations` computed both registry fingerprints on every call,
//! ahead of the cache probe — and a SHACL validation issues one query per focus
//! node, so a host that had configured the seam paid a full `describe()` walk and
//! digest per focus node purely to build a lookup key, on a path whose scratch
//! buffer exists precisely so that a cache hit allocates nothing. Hoisting the
//! derivation to the environment makes the hit path what it was built to be.
//!
//! # Two identity tiers, exactly as [`crate::registry_id`] defines them
//!
//! [`ExtensionEnv::id`] is the instance tier: a process-lifetime counter answering
//! "is this the same live environment the thing in my hand was bound against?". It
//! is the cheap per-call comparison, and it must never be persisted.
//!
//! [`ExtensionEnv::content_fingerprint`] is the content tier: a pure function of
//! the declared configuration, reproducible byte-for-byte in any process. It is
//! what a persisted artifact binds itself to when it must name the environment it
//! requires — which is how a prepared shapes product refuses to restore under a
//! host whose declared namespaces differ from the ones it was written under.

use std::borrow::Cow;
use std::sync::OnceLock;

use purrdf_core::ContentDigest;
use purrdf_sparql_algebra::ParserOptions;

use crate::agg_fn::AggregateRegistry;
use crate::error::EvalError;
use crate::property_fn::PropertyFunctionRegistry;
use crate::registry_id::{RegistryId, append_framed_part};

/// The domain separator this environment's content digest opens with, so it can
/// never collide with a digest of any other kind that happens to fold a
/// structurally identical field sequence.
const CONTENT_DOMAIN: &str = "purrdf-sparql-eval/extension-env";

/// The schema version of the field sequence [`ExtensionEnv::content_fingerprint`]
/// folds. Length-framing makes each version's encoding injective *within* a
/// version and says nothing across versions, so the version is folded in and the
/// question stops being open. Bump it whenever a field is added, removed, or
/// reordered below.
const CONTENT_VERSION: u16 = 1;

/// [`ParserOptions::default`] spelled as a `const`, so [`ExtensionEnv::EMPTY`] can be
/// one. `Default` is not a const trait, and the three fields are `pub`, so the
/// literal is written out rather than derived.
const EMPTY_PARSER_OPTIONS: ParserOptions = ParserOptions {
    extension_fn_namespaces: Vec::new(),
    property_fn_namespaces: Vec::new(),
    property_fn_iris: Vec::new(),
};

/// The configuration a SPARQL text is interpreted relative to, with every derived
/// value computed at construction.
///
/// See the module documentation for why this is one value rather than four
/// arguments.
#[derive(Debug)]
pub struct ExtensionEnv {
    /// The caller-declared parser options, before the registry's exact IRIs are
    /// unioned in. Retained because it is part of this environment's identity: two
    /// environments whose *effective* options coincide by accident, one having
    /// declared a namespace and the other having registered every IRI under it,
    /// are not the same environment — the first accepts an unregistered IRI under
    /// that namespace as a hard error, the second reads it as ordinary data.
    base: ParserOptions,
    /// Held by value rather than behind an `Arc`, so [`Self::EMPTY`] can be a `const`
    /// and `QueryOptions::EMPTY` can stay one. Cloning either registry copies a map
    /// of `Arc<dyn …>` trait objects and — load-bearing — PRESERVES its
    /// [`RegistryId`], so a clone is the same registry instance for every purpose a
    /// plan's identity cares about.
    relations: PropertyFunctionRegistry,
    aggregates: AggregateRegistry,
    /// [`Self::base`] with `relations`' registered IRIs unioned into
    /// [`ParserOptions::property_fn_iris`]. Derived once; see
    /// [`Self::parser_options`].
    effective: ParserOptions,
    /// The instance-tier fingerprint `PlanCache` folds into its lookup key.
    relations_fingerprint: String,
    /// The instance-tier fingerprint `PlanCache` folds into its lookup key.
    aggregates_fingerprint: String,
    /// The content digest, computed on first demand rather than at construction.
    ///
    /// Everything else here is derived eagerly because every query needs it. This is
    /// not: only a caller that must state its environment ACROSS A PROCESS BOUNDARY
    /// — a prepared product writing or checking its identity row — ever asks, and
    /// that happens once per artifact rather than once per query.
    ///
    /// Eager computation would therefore have made this type strictly more expensive
    /// to construct than the loose registries it replaces: a caller building one per
    /// query would pay a full digest over both registries' declarations on a path
    /// that previously paid two fingerprints. Deferring it means constructing an
    /// environment costs what deriving the parse configuration always cost, and the
    /// hoisting caller — the one this type exists for — pays neither per query.
    content: OnceLock<ContentDigest>,
    id: RegistryId,
}

/// The canonical environment that configures nothing.
///
/// One shared value rather than a fresh one per caller, for the reason
/// [`RegistryId::EMPTY`] gives: every empty environment interprets every text
/// identically, so a distinction between two of them would be a distinction nothing
/// honours. Sharing it also means [`ExtensionEnv::empty`] is the answer to "no
/// environment", and there is no second spelling — no `Option`, no
/// absent-versus-present-but-empty pair — for a consumer to disagree about.
///
/// A `static` holding the literal rather than a `const` anyone can copy. The
/// difference is load-bearing here: a `const` is substituted at each use site, so
/// every user would get its own [`OnceLock`] and the memoized content digest would
/// be computed once PER USE rather than once. A `static` is one value with one
/// cell — which is also why its interior mutability is correct rather than a hazard.
static EMPTY: ExtensionEnv = ExtensionEnv {
    base: EMPTY_PARSER_OPTIONS,
    relations: PropertyFunctionRegistry::EMPTY,
    aggregates: AggregateRegistry::EMPTY,
    effective: EMPTY_PARSER_OPTIONS,
    relations_fingerprint: String::new(),
    aggregates_fingerprint: String::new(),
    content: OnceLock::new(),
    id: RegistryId::EMPTY,
};

impl ExtensionEnv {
    /// Build an environment and derive everything it will ever be asked for.
    ///
    /// # The derivation, and why it is spelled here and nowhere else
    ///
    /// A relation is reachable from SPARQL only if the parser lowered its predicate
    /// IRI to a call node, and the parser does that only for an IRI under a
    /// configured [`ParserOptions::property_fn_namespaces`] entry OR an entry of
    /// [`ParserOptions::property_fn_iris`]. Deriving the latter here, from the very
    /// registry the evaluation will resolve against, is what keeps the two from
    /// drifting: a host cannot register a relation the parser does not recognize,
    /// and cannot configure an IRI whose call resolves against a different table.
    ///
    /// Registered IRIs go into [`ParserOptions::property_fn_iris`] — EXACT match —
    /// and deliberately never into [`ParserOptions::property_fn_namespaces`] —
    /// PREFIX match. A registry's keys are exact IRIs, not namespaces: folding
    /// `https://example.org/rel/a` in as a prefix would reclassify the unrelated,
    /// merely-same-prefixed data predicate `https://example.org/rel/ab` as a call to
    /// an unregistered relation, which then hard-errors — a previously-working query
    /// breaking with a diagnostic that names the wrong cause. A host that wants a
    /// whole namespace recognized — including IRIs it has deliberately left
    /// unregistered, so that spelling one is a hard error rather than a silent data
    /// triple — declares that namespace in `base`; the two sets (caller-declared
    /// namespaces, registry-derived exact IRIs) are unioned, never conflated.
    ///
    /// [`PropertyFunctionRegistry::describe`] is IRI-sorted, so the derived set is a
    /// pure function of the registry's contents rather than of its registration
    /// order.
    ///
    /// # Errors
    ///
    /// [`EvalError`] if a registered relation's or aggregate's declaration methods
    /// panic — each registry's own `describe` failure, propagated unchanged. An
    /// environment over empty registries reads no declaration and cannot fail.
    pub fn new(
        base: ParserOptions,
        relations: PropertyFunctionRegistry,
        aggregates: AggregateRegistry,
    ) -> Result<Self, EvalError> {
        let effective = match derive_parser_options(&base, &relations)? {
            Cow::Borrowed(_) => base.clone(),
            Cow::Owned(owned) => owned,
        };
        let relations_fingerprint = crate::property_fn_plan::registry_fingerprint(&relations)?;
        let aggregates_fingerprint = crate::agg_fn::registry_fingerprint(&aggregates)?;
        Ok(Self {
            base,
            relations,
            aggregates,
            effective,
            relations_fingerprint,
            aggregates_fingerprint,
            content: OnceLock::new(),
            id: RegistryId::fresh(),
        })
    }

    /// An environment over `relations` and nothing else: default parser options and
    /// no custom aggregates.
    ///
    /// The shape almost every caller of the relation seam wants, and short enough to
    /// write inline where the old `property_functions:` field used to go.
    ///
    /// # Errors
    ///
    /// [`EvalError`] if a registered relation's declaration methods panic.
    pub fn over_relations(relations: PropertyFunctionRegistry) -> Result<Self, EvalError> {
        Self::new(EMPTY_PARSER_OPTIONS, relations, AggregateRegistry::EMPTY)
    }

    /// An environment over both registries, under default parser options.
    ///
    /// The shape a host wiring both seams wants, without having to name
    /// [`ParserOptions`] to say "the default ones" — which is most hosts, since
    /// declaring a namespace is the deliberate exception rather than the rule.
    ///
    /// # Errors
    ///
    /// [`EvalError`] if a registered relation's or aggregate's declaration methods
    /// panic.
    pub fn over(
        relations: PropertyFunctionRegistry,
        aggregates: AggregateRegistry,
    ) -> Result<Self, EvalError> {
        Self::new(EMPTY_PARSER_OPTIONS, relations, aggregates)
    }

    /// This environment with its relation registry replaced, and everything else —
    /// the caller's declared parser options and the aggregate registry — carried
    /// through unchanged.
    ///
    /// For a caller that must swap the relation table mid-flight without losing the
    /// rest of the configuration. The entailment lanes do exactly this: a
    /// dataset-derived relation has to be re-derived over the materialized closure
    /// that is about to be queried, so the walk and the surrounding patterns read one
    /// dataset rather than two.
    ///
    /// Rebuilding from scratch instead would silently drop the caller's declared
    /// namespaces — and an environment that has forgotten a declared namespace reads
    /// a prefixed relation IRI as an ordinary data triple, which is the exact
    /// silent-wrong-answer shape this type exists to prevent.
    ///
    /// # Errors
    ///
    /// [`EvalError`] if a registered relation's declaration methods panic.
    pub fn with_relations(&self, relations: PropertyFunctionRegistry) -> Result<Self, EvalError> {
        Self::new(self.base.clone(), relations, self.aggregates.clone())
    }

    /// An environment over `aggregates` and nothing else: default parser options and
    /// no relations.
    ///
    /// # Errors
    ///
    /// [`EvalError`] if a registered aggregate's declaration methods panic.
    pub fn over_aggregates(aggregates: AggregateRegistry) -> Result<Self, EvalError> {
        Self::new(
            EMPTY_PARSER_OPTIONS,
            PropertyFunctionRegistry::EMPTY,
            aggregates,
        )
    }

    /// The environment that configures nothing: default parser options and the
    /// canonical empty registries — the one answer to "no environment", with no
    /// second spelling.
    ///
    /// A `const fn` so [`crate::engine::QueryOptions::EMPTY`] can stay a `const`,
    /// which is what keeps every caller that writes `..QueryOptions::EMPTY` working
    /// without naming an environment at all. Its fingerprints are the empty string
    /// for the same reason both registry modules' `registry_fingerprint`
    /// short-circuit there, and its id is [`RegistryId::EMPTY`].
    #[must_use]
    pub const fn empty() -> &'static Self {
        &EMPTY
    }

    /// The options a parse of any text under this environment must use: the
    /// caller's declared namespaces unioned with this environment's registry-derived
    /// exact IRIs.
    ///
    /// Returned as a borrow of a value derived once in [`Self::new`], so asking is
    /// free however often a consumer asks.
    #[must_use]
    pub fn parser_options(&self) -> &ParserOptions {
        &self.effective
    }

    /// The caller-declared options this environment was built from, before the
    /// registry's exact IRIs were unioned in.
    #[must_use]
    pub fn base_parser_options(&self) -> &ParserOptions {
        &self.base
    }

    /// The relation registry a lowered call resolves against.
    #[must_use]
    pub fn relations(&self) -> &PropertyFunctionRegistry {
        &self.relations
    }

    /// The custom-aggregate registry a `Custom` call is admitted against.
    #[must_use]
    pub fn aggregates(&self) -> &AggregateRegistry {
        &self.aggregates
    }

    /// The instance-tier relation fingerprint `PlanCache` folds into its key,
    /// computed once in [`Self::new`] rather than per prepare.
    pub(crate) fn relations_fingerprint(&self) -> &str {
        &self.relations_fingerprint
    }

    /// The instance-tier aggregate fingerprint `PlanCache` folds into its key,
    /// computed once in [`Self::new`] rather than per prepare.
    pub(crate) fn aggregates_fingerprint(&self) -> &str {
        &self.aggregates_fingerprint
    }

    /// This environment's **instance** identity — the cheap comparison, valid only
    /// within this process.
    ///
    /// This is what a per-call check compares, because a call happens once per row
    /// per call site and a content digest is a walk plus a hash. Never persist it;
    /// see [`crate::registry_id`] for why a counter read back in another process
    /// carries no information.
    #[must_use]
    pub fn id(&self) -> RegistryId {
        self.id
    }

    /// This environment's **content** identity — a pure function of the declared
    /// configuration, reproducible in any process.
    ///
    /// This is what a persisted artifact binds itself to. It folds the declared
    /// parser options together with both registries' content digests, so an
    /// artifact written under one set of declared namespaces refuses to restore
    /// under another — the failure that would otherwise reproduce this module's
    /// occasioning defect across a process boundary, silently.
    ///
    /// It binds **declarations**, not implementations: two independently built
    /// registries that declare identically digest identically even when their
    /// trait objects return entirely different rows. That is the hole
    /// [`Self::id`] closes in-process and no content-derived value can close, so a
    /// caller crossing a process boundary must pair this digest with whatever
    /// separately identifies the implementations behind those declarations.
    /// # Errors
    ///
    /// [`EvalError`] if a registered relation's or aggregate's declaration methods
    /// panic. Computed once and memoized; a second call cannot fail if the first
    /// succeeded.
    pub fn content_fingerprint(&self) -> Result<ContentDigest, EvalError> {
        if let Some(digest) = self.content.get() {
            return Ok(*digest);
        }
        let digest = content_digest(&self.base, &self.relations, &self.aggregates)?;
        Ok(*self.content.get_or_init(|| digest))
    }
}

/// `base` with `registry`'s registered IRIs unioned into
/// [`ParserOptions::property_fn_iris`].
///
/// Returns `base` unmodified — no clone, no allocation — when `registry` is empty,
/// which is every request on a host that has not configured the seam. See
/// [`ExtensionEnv::new`] for why the union is EXACT-match and never PREFIX.
///
/// A free function with two reference parameters and no `&self` cannot elide its
/// output lifetime, so the tie to `base` is spelled explicitly.
///
/// # Errors
///
/// [`EvalError`] if a registered relation's declaration methods panic.
pub(crate) fn derive_parser_options<'a>(
    base: &'a ParserOptions,
    registry: &PropertyFunctionRegistry,
) -> Result<Cow<'a, ParserOptions>, EvalError> {
    if registry.is_empty() {
        return Ok(Cow::Borrowed(base));
    }
    let mut options = base.clone();
    for descriptor in registry.describe()? {
        if !options.property_fn_iris.contains(&descriptor.iri) {
            options.property_fn_iris.push(descriptor.iri);
        }
    }
    Ok(Cow::Owned(options))
}

/// The content digest of a declared configuration.
///
/// Framed through [`append_framed_part`], whose length-prefixing makes the byte
/// sequence injective: no combination of declared values can forge the encoding
/// another combination produces. The two registries contribute their own **content**
/// digests rather than their instance-tier fingerprints, because this value must
/// survive a process boundary and an instance id does not.
///
/// The *declared* options are folded, not the effective ones. Two environments whose
/// effective options coincide — one declaring a namespace, the other registering
/// every IRI under it — differ in behaviour on an unregistered IRI under that
/// namespace (a hard error versus ordinary data), so they must not share a digest.
fn content_digest(
    base: &ParserOptions,
    relations: &PropertyFunctionRegistry,
    aggregates: &AggregateRegistry,
) -> Result<ContentDigest, EvalError> {
    let mut bytes = Vec::new();
    append_framed_part(&mut bytes, "domain", CONTENT_DOMAIN.as_bytes());
    append_framed_part(&mut bytes, "version", &CONTENT_VERSION.to_be_bytes());
    for (label, list) in [
        ("extension-fn-namespaces", &base.extension_fn_namespaces),
        ("property-fn-namespaces", &base.property_fn_namespaces),
        ("property-fn-iris", &base.property_fn_iris),
    ] {
        append_framed_part(&mut bytes, label, &(list.len() as u64).to_be_bytes());
        for value in list {
            append_framed_part(&mut bytes, label, value.as_bytes());
        }
    }
    append_framed_part(
        &mut bytes,
        "relations",
        crate::property_fn_plan::content_fingerprint(relations)?
            .to_hex()
            .as_bytes(),
    );
    append_framed_part(
        &mut bytes,
        "aggregates",
        crate::agg_fn::content_fingerprint(aggregates)?
            .to_hex()
            .as_bytes(),
    );
    Ok(ContentDigest::of(&bytes))
}

#[cfg(test)]
mod tests {
    use super::{ExtensionEnv, derive_parser_options};
    use crate::agg_fn::AggregateRegistry;
    use crate::property_fn::{MemoryRelation, PropertyFunctionRegistry};
    use purrdf_sparql_algebra::ParserOptions;
    use std::sync::Arc;

    const A: &str = "http://example.org/rel/a";
    const B: &str = "http://example.org/rel/b";
    const C: &str = "http://example.org/rel/c";

    /// A registry over `iris`, registered in the order given.
    fn relations(iris: &[&str]) -> PropertyFunctionRegistry {
        let mut registry = PropertyFunctionRegistry::new();
        for iri in iris {
            registry.register(
                (*iri).to_owned(),
                Arc::new(MemoryRelation::new(1, 1, Vec::new()).expect("an empty table is valid")),
            );
        }
        registry
    }

    fn env(base: ParserOptions, iris: &[&str]) -> ExtensionEnv {
        ExtensionEnv::new(base, relations(iris), AggregateRegistry::EMPTY)
            .expect("declarations read cleanly")
    }

    /// The derived set, pinned as a committed literal rather than as a comparison
    /// against whatever the code currently does.
    ///
    /// The three IRIs are registered in the order C, A, B and must come back sorted,
    /// because `describe()` is IRI-sorted and that is what makes the derived options
    /// a pure function of the registry's CONTENTS rather than of the order a host
    /// happened to call `register` in.
    #[test]
    fn derived_options_match_a_committed_literal() {
        let env = env(ParserOptions::default(), &[C, A, B]);
        assert_eq!(
            env.parser_options().property_fn_iris,
            vec![A.to_owned(), B.to_owned(), C.to_owned()],
            "registered IRIs are unioned in, IRI-sorted"
        );
        assert!(
            env.parser_options().property_fn_namespaces.is_empty(),
            "a registry's keys are exact IRIs and must NEVER be folded in as prefixes: \
             doing so would reclassify the merely-same-prefixed data predicate \
             `{A}b` as a call to an unregistered relation"
        );
        assert_eq!(
            env.parser_options().extension_fn_namespaces,
            Vec::<String>::new(),
            "a relation registry configures the predicate-position seam only; the \
             call-position seam is the caller's to declare and must be left untouched"
        );
    }

    /// The over-refusal guard the EXACT-vs-PREFIX rule exists for, stated as the
    /// neighbouring valid case: registering `…/rel/a` must leave `…/rel/ab` alone.
    #[test]
    fn a_registered_iri_does_not_capture_a_longer_sibling() {
        let env = env(ParserOptions::default(), &[A]);
        let sibling = format!("{A}b");
        assert!(
            !env.parser_options().property_fn_iris.contains(&sibling),
            "the exact-IRI set must not contain a sibling nobody registered"
        );
        assert!(
            !env.parser_options()
                .property_fn_namespaces
                .iter()
                .any(|ns| sibling.starts_with(ns.as_str())),
            "and no derived namespace prefix may match it either"
        );
    }

    /// A caller-declared namespace and the registry-derived exact IRIs are unioned,
    /// never conflated: the declaration survives untouched alongside the derivation.
    #[test]
    fn declared_namespaces_are_unioned_not_conflated() {
        let base = ParserOptions {
            property_fn_namespaces: vec!["http://example.org/ns/".to_owned()],
            ..ParserOptions::default()
        };
        let env = env(base, &[A]);
        assert_eq!(
            env.parser_options().property_fn_namespaces,
            vec!["http://example.org/ns/".to_owned()],
            "the caller's declared namespace is carried through unchanged"
        );
        assert_eq!(
            env.parser_options().property_fn_iris,
            vec![A.to_owned()],
            "and the registry's exact IRI joins it rather than replacing it"
        );
    }

    /// An environment over empty registries parses exactly as an unconfigured host
    /// always did, and constructing it allocates no option storage at all.
    #[test]
    fn an_empty_environment_derives_the_base_options_without_allocating() {
        let empty = ExtensionEnv::empty();
        let options = empty.parser_options();
        assert_eq!(options, &ParserOptions::default());
        for (label, capacity) in [
            (
                "extension_fn_namespaces",
                options.extension_fn_namespaces.capacity(),
            ),
            (
                "property_fn_namespaces",
                options.property_fn_namespaces.capacity(),
            ),
            ("property_fn_iris", options.property_fn_iris.capacity()),
        ] {
            assert_eq!(capacity, 0, "{label} must not have allocated");
        }
    }

    /// The borrow-not-clone path, asserted on the derivation itself: an empty
    /// registry contributes nothing, so the caller's own options come back
    /// untouched.
    #[test]
    fn an_empty_registry_borrows_the_base_options() {
        let base = ParserOptions::default();
        let derived = derive_parser_options(&base, &PropertyFunctionRegistry::EMPTY)
            .expect("an empty registry reads no declaration");
        assert!(
            matches!(derived, std::borrow::Cow::Borrowed(_)),
            "an empty registry must not clone the caller's options"
        );
    }

    /// The content fingerprint is a pure function of declarations, so the order a
    /// host registered them in cannot change it. Without this, two processes that
    /// built the same logical environment differently would refuse each other's
    /// artifacts.
    #[test]
    fn registration_order_does_not_change_the_content_fingerprint() {
        let forward = env(ParserOptions::default(), &[A, B, C]);
        let reversed = env(ParserOptions::default(), &[C, B, A]);
        assert_eq!(
            forward.content_fingerprint().expect("digest"),
            reversed.content_fingerprint().expect("digest"),
            "same declarations, different registration order, same identity"
        );
    }

    #[test]
    fn different_relation_sets_produce_different_content_fingerprints() {
        let two = env(ParserOptions::default(), &[A, B]);
        let three = env(ParserOptions::default(), &[A, B, C]);
        assert_ne!(
            two.content_fingerprint().expect("digest"),
            three.content_fingerprint().expect("digest")
        );
    }

    /// Declared namespaces are part of the environment's identity even though they
    /// are not part of any registry. An artifact written under a declared namespace
    /// and restored under a host that declares nothing would otherwise read a
    /// prefixed relation IRI as ordinary data and report conformance — the same
    /// silent-wrong-answer shape this module exists to close, one process boundary
    /// over.
    #[test]
    fn declared_namespaces_are_part_of_the_identity() {
        let bare = env(ParserOptions::default(), &[A]);
        let declared = env(
            ParserOptions {
                property_fn_namespaces: vec!["http://example.org/rel/".to_owned()],
                ..ParserOptions::default()
            },
            &[A],
        );
        assert_ne!(
            bare.content_fingerprint().expect("digest"),
            declared.content_fingerprint().expect("digest"),
            "an environment that declares a namespace is not the environment that does not"
        );
    }

    /// The digest is memoized, so the price of deferring it is paid at most once per
    /// environment however many artifacts ask.
    #[test]
    fn the_content_digest_is_computed_once_and_reused() {
        let env = env(ParserOptions::default(), &[A, B]);
        let first = env.content_fingerprint().expect("digest");
        let second = env.content_fingerprint().expect("digest");
        assert_eq!(first, second);
    }

    /// The instance tier answers a question the content tier cannot: two registries
    /// can declare identically and resolve the same IRI to different implementations.
    #[test]
    fn two_environments_never_share_an_instance_id() {
        let left = env(ParserOptions::default(), &[A]);
        let right = env(ParserOptions::default(), &[A]);
        assert_ne!(left.id(), right.id());
        assert_eq!(
            left.content_fingerprint().expect("digest"),
            right.content_fingerprint().expect("digest"),
            "while declaring identically, which is exactly why the instance tier exists"
        );
    }

    /// The fingerprints the plan cache keys on are the ones the standalone
    /// derivations produce — the environment caches them, it does not redefine them.
    #[test]
    fn the_cached_fingerprints_equal_the_standalone_derivations() {
        let registry = relations(&[A, B]);
        let aggregates = AggregateRegistry::EMPTY;
        let env = ExtensionEnv::new(
            ParserOptions::default(),
            registry.clone(),
            aggregates.clone(),
        )
        .expect("declarations read cleanly");
        assert_eq!(
            env.relations_fingerprint(),
            crate::property_fn_plan::registry_fingerprint(&registry).expect("fingerprint"),
        );
        assert_eq!(
            env.aggregates_fingerprint(),
            crate::agg_fn::registry_fingerprint(&aggregates).expect("fingerprint"),
        );
    }
}
