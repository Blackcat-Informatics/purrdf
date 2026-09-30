// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! An empty identity component is rendered as the quoted empty string `""` on every
//! surface that spells one, through the one renderer
//! (`purrdf_core::artifact::identity::render_value`).
//!
//! The surfaces are the public entry points a caller reaches: `explain_shapes_product`,
//! the `Display` of `diff_shapes_products` (on the present side and beside a missing
//! one) and the refusal `admit_shapes_product` raises when a product's binding is not
//! the one in force. `validate` once printed `0x` for an empty value where the
//! artifact envelope printed `""`. The neighbours are pinned beside it: a component
//! holding text stays quoted, one holding bytes stays `0x` hex, and a side that
//! carries no component is still `missing`.
//!
//! A product whose identity holds an empty component is made the way a foreign
//! producer would make one: the envelope is rebuilt around a changed identity.
//!
//! Fixtures use `example.org` throughout; every IRI is fixture configuration.

use purrdf_core::artifact::{ArtifactBuilder, ArtifactSpec, ArtifactView, Identity};
use purrdf_validate::product::{
    admit_shapes_product, diff_shapes_products, explain_shapes_product, pack_shapes_product,
};

const SHAPES: &str = "@prefix sh: <http://www.w3.org/ns/shacl#> .\n\
    @prefix ex: <http://example.org/> .\n\
    ex:PersonShape a sh:NodeShape ; sh:targetClass ex:Person .\n";

/// The component this test empties: text in a real product, so its neighbours show
/// text staying text.
const LABEL: &str = "profile";

/// Rebuild `product`'s envelope around the identity `edit` returns; a component for
/// which `edit` returns `None` is dropped.
fn repack(product: &[u8], edit: impl Fn(&str, &[u8]) -> Option<Vec<u8>>) -> Vec<u8> {
    // The header opens with the 8-byte magic; the version and section count are the
    // format's own (`explain` reports `format-version 1`, and there are three
    // sections), so the spec is read back off the product rather than restated.
    let magic: [u8; 8] = product[..8].try_into().expect("a product has a header");
    let spec = ArtifactSpec::new(magic, 1, 3);
    let view = ArtifactView::from_bytes(spec, product).expect("the product is well formed");
    let mut identity = Identity::new();
    for component in view.identity().components() {
        if let Some(value) = edit(component.label(), component.value()) {
            identity.push(component.label(), &value);
        }
    }
    let mut builder = ArtifactBuilder::new(spec);
    builder.identity(identity);
    for kind in view.section_kinds() {
        builder.section(
            kind,
            view.section(kind).expect("a listed section is present"),
        );
    }
    builder.build_bytes().expect("the envelope rebuilds")
}

fn products() -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    let plain = pack_shapes_product(SHAPES, None, &[]).expect("shapes pack");
    let emptied = repack(&plain, |label, value| {
        Some(if label == LABEL {
            Vec::new()
        } else {
            value.to_vec()
        })
    });
    let dropped = repack(&plain, |label, value| {
        (label != LABEL).then(|| value.to_vec())
    });
    (plain, emptied, dropped)
}

#[test]
fn explain_renders_the_empty_component_as_quoted_empty_text() {
    let (plain, emptied, _) = products();
    let explained = explain_shapes_product(&emptied).expect("the product explains");
    assert!(
        explained
            .lines()
            .any(|line| line == "identity profile \"\""),
        "empty component must read `\"\"`: {explained}"
    );
    assert!(
        !explained.lines().any(|line| line == "identity profile 0x"),
        "and never a bare `0x`: {explained}"
    );
    // Neighbours: text stays quoted, bytes stay hex.
    let neighbour = explain_shapes_product(&plain).expect("the product explains");
    assert!(
        neighbour
            .lines()
            .any(|line| line == "identity profile \"purrdf-shacl-core-v1\"")
    );
    assert!(
        neighbour
            .lines()
            .any(|line| line == "identity box-role-vocab 0x00")
    );
    assert!(
        explained
            .lines()
            .any(|line| line == "identity box-role-vocab 0x00")
    );
}

#[test]
fn diff_renders_the_empty_component_on_either_side_and_beside_a_missing_one() {
    let (plain, emptied, dropped) = products();
    let expectations = [
        (
            &plain,
            &emptied,
            "diff profile \"purrdf-shacl-core-v1\" \"\"",
        ),
        (
            &emptied,
            &plain,
            "diff profile \"\" \"purrdf-shacl-core-v1\"",
        ),
        (&emptied, &dropped, "diff profile \"\" missing"),
        (&dropped, &emptied, "diff profile missing \"\""),
    ];
    for (a, b, line) in expectations {
        let rendered = diff_shapes_products(a, b)
            .expect("both sides are products")
            .to_string();
        assert_eq!(rendered, format!("diff-count 1\n{line}\n"));
    }
    // A product against itself has nothing to render.
    let same = diff_shapes_products(&emptied, &emptied).expect("diff");
    assert!(same.identical());
}

#[test]
fn an_admission_refusal_renders_the_empty_component_as_quoted_empty_text() {
    let (plain, emptied, _) = products();
    let refusal = admit_shapes_product(&emptied)
        .expect_err("a product bound to an empty profile is not the one in force")
        .to_string();
    assert!(
        refusal.contains("component `profile` (position 4): artifact has \"\", caller supplied \"purrdf-shacl-core-v1\""),
        "the refusal spells the empty component `\"\"`: {refusal}"
    );
    assert!(
        !refusal.contains("0x,") && !refusal.contains("has 0x"),
        "{refusal}"
    );
    // The valid neighbour is admitted.
    admit_shapes_product(&plain).expect("the untouched product is admitted");
}
