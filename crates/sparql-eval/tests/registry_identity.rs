// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! A property-function registry answers two distinct identity questions, and a
//! composition layer over the seam needs both.
//!
//! * **Which instance is this?** [`PropertyFunctionRegistry::instance_id`] mints a
//!   per-process identity at construction, so two registries built independently can
//!   never collide even when every declaration they report is byte-identical. This is
//!   the guard a prepared plan needs: two registries can register the same IRI to two
//!   different [`PropertyFunction`](purrdf_sparql_eval::PropertyFunction)
//!   implementations that describe themselves identically yet return different rows.
//! * **What shape do these declare?** [`PropertyFunctionRegistry::content_fingerprint`]
//!   is the durable, instance-independent digest of the declarations alone (IRIs,
//!   arities, volatilities, modes and their row bounds, IRI-sorted). Two registries
//!   that declare identically must produce the identical fingerprint — here or in
//!   another process — because it carries no ephemeral state.
//!
//! Getting these backwards is a real bug in either direction: an instance id used as
//! durable identity is unstable across processes; a content fingerprint used to guard a
//! plan would let a plan admitted under one registry run under a different one that
//! merely describes itself the same.

use std::sync::Arc;

use purrdf_core::TermValue;
use purrdf_sparql_eval::{MemoryRelation, PropertyFunctionRegistry, RegistryId};

/// The single relation IRI every registry below registers.
const EX_REL: &str = "http://example.org/ns#rel";

/// Build a one-subject/one-object relation whose single row ends in `object`. The
/// row *content* varies between callers while every declaration the registry reports
/// (IRI, arity, volatility, modes, row bound) stays identical.
fn relation_ending_in(object: &str) -> Arc<MemoryRelation> {
    Arc::new(
        MemoryRelation::new(
            1,
            1,
            vec![vec![
                TermValue::iri("http://example.org/ns#one"),
                TermValue::iri(object),
            ]],
        )
        .expect("one row, two values wide"),
    )
}

/// A registry holding exactly one relation, built from scratch — a fresh instance
/// with its own [`RegistryId`].
fn registry_ending_in(object: &str) -> PropertyFunctionRegistry {
    let mut registry = PropertyFunctionRegistry::new();
    registry.register(EX_REL, relation_ending_in(object));
    registry
}

/// Two independently built registries with byte-identical descriptions must still
/// carry different instance ids: the declared shape cannot prove the implementations
/// answer the same way.
#[test]
fn independently_built_registries_have_distinct_instance_ids() {
    let a = registry_ending_in("http://example.org/ns#alpha");
    let b = registry_ending_in("http://example.org/ns#beta");

    assert_eq!(
        a.describe().expect("descriptions are readable"),
        b.describe().expect("descriptions are readable"),
        "the two registries are structurally identical by declaration"
    );

    // Naming the type explicitly proves `RegistryId` itself is reachable from outside
    // the crate, not merely the method that returns it.
    let a_id: RegistryId = a.instance_id();
    let b_id: RegistryId = b.instance_id();
    assert_ne!(
        a_id, b_id,
        "two independently constructed registries must never share an instance id, \
         even when every declaration they report is identical"
    );
}

/// Identical declarations must produce identical content fingerprints regardless of
/// which registry instance reports them (or when they were built).
#[test]
fn identical_declarations_share_a_content_fingerprint_across_instances() {
    let a = registry_ending_in("http://example.org/ns#alpha");
    let b = registry_ending_in("http://example.org/ns#beta");
    let c = registry_ending_in("http://example.org/ns#gamma");

    // The instances are genuinely distinct...
    assert_ne!(a.instance_id(), b.instance_id());
    assert_ne!(b.instance_id(), c.instance_id());
    // ...yet their durable content digests agree, because content is the only input.
    let content = a.content_fingerprint().expect("declarations are readable");
    assert_eq!(
        content,
        b.content_fingerprint().expect("declarations are readable"),
        "identical declarations must share a content fingerprint across instances"
    );
    assert_eq!(
        content,
        c.content_fingerprint().expect("declarations are readable"),
        "a third registry with the same declarations must also agree"
    );
}

/// A changed declaration must change the content fingerprint — the digest is not a
/// constant.
#[test]
fn differing_declarations_produce_differing_content_fingerprints() {
    let a = registry_ending_in("http://example.org/ns#alpha");

    // The same IRI, but declared with a different arity: two subject positions and no
    // object position. Every declaration read differs from `a`'s.
    let mut b = PropertyFunctionRegistry::new();
    b.register(
        EX_REL,
        Arc::new(
            MemoryRelation::new(
                2,
                0,
                vec![vec![
                    TermValue::iri("http://example.org/ns#one"),
                    TermValue::iri("http://example.org/ns#two"),
                ]],
            )
            .expect("one row, two values wide"),
        ),
    );

    assert_ne!(
        a.content_fingerprint().expect("declarations are readable"),
        b.content_fingerprint().expect("declarations are readable"),
        "a differing declaration must not share a content fingerprint"
    );
}
