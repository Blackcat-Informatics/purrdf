// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Exact-byte, loss, backend, and insertion-order evidence for `okf-terms`.

use std::fmt::Write as _;

use purrdf_rdf::{
    PackBuilder, PackView, ProjectionConfig, ProjectionPackage, ProjectionProfile, SerializeGraph,
    parse_dataset, project_archive, project_okf_terms, serialize_dataset,
};
use sha2::{Digest, Sha256};

const CONFIG: &[u8] = include_bytes!("fixtures/okf-terms.json");
const SOURCE: &[u8] = include_bytes!("fixtures/okf-terms.trig");

const CLASS_DOCUMENT: &str = r#"---
type: "Class"
title: "Alpha"
description: "An *important* class."
resource: "https://example.org/A"
tags:
  - "alpha"
  - "named-graph"
  - "zeta"
timestamp: "2026-07-19T12:34:56Z"
active: true
count: 1
identity:
  - "https://example.org/Class"
score: 1.5
---
## Definition

An \*important\* class.

## Notes

- Use **carefully**.

## Relations

- related: [Beta](../properties/B.md)
- related: [https://outside.example/External](<https://outside.example/External>)
"#;

const CLASS_INDEX: &str = "# Classes\n\n* [Alpha](A.md) - An \\*important\\* class.\n";

const ROOT_INDEX: &str = r"# Example knowledge

## Projection fidelity

This bundle carries only the caller-configured concept view; consult the located loss ledger for omitted RDF 1.2 records.

## Categories

* [Classes](classes/index.md) - Terms classified as example classes. (1)

* [Properties](properties/index.md) - Terms classified as example properties. (1)
";

const PROPERTY_DOCUMENT: &str = r#"---
type: "Property"
title: "Beta"
description: "A linked property."
resource: "https://example.org/B"
identity:
  - "https://example.org/Property"
---
## Definition

A linked property.
"#;

const PROPERTY_INDEX: &str = "# Properties\n\n* [Beta](B.md) - A linked property.\n";

const LOSS_LEDGER: &str = r#"{
  "schema_version": 1,
  "losses": [
    {
      "code": "named-graph-dropped",
      "from": "rdf-1.2-dataset",
      "to": "okf",
      "intentional": true,
      "note": "OKF documents have no named-graph placement; a quad asserted outside the default graph cannot be represented in Markdown frontmatter or body text.",
      "location": "okf-terms:named-graph subject=OkfTermsGraph_9dc301f2a53e6251c3b75db81bcc816cd67937f8522d74658f285d97cbf67545"
    },
    {
      "code": "named-graph-dropped",
      "from": "rdf-1.2-dataset",
      "to": "okf",
      "intentional": true,
      "note": "OKF documents have no named-graph placement; a quad asserted outside the default graph cannot be represented in Markdown frontmatter or body text.",
      "location": "okf-terms:quad subject=OkfTermsQuad_69199f28e43f6d3d9f3f5163b9c6942dbe35e6913e78953e1ac0d58c0a231ca1"
    },
    {
      "code": "okf-non-profile-quad-dropped",
      "from": "rdf-1.2-dataset",
      "to": "okf",
      "intentional": true,
      "note": "An RDF statement outside the caller-configured OKF profile, including an OWL axiom, has no Markdown/frontmatter field and is omitted from the bundle.",
      "location": "okf-terms:quad subject=OkfTermsQuad_5f65b0f5d8da82f971f4714cd9ab12a713089c929ca9031ac908af887900066f"
    }
  ]
}
"#;

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .fold(String::with_capacity(64), |mut output, byte| {
            write!(output, "{byte:02x}").expect("writing to String cannot fail");
            output
        })
}

#[test]
fn fixture_pins_documents_indexes_losses_member_order_and_archive_digest() {
    let dataset = parse_dataset(SOURCE, "application/trig", None).expect("fixture dataset");
    let config = ProjectionConfig::from_json(CONFIG).expect("fixture config");
    let ProjectionConfig::OkfTerms(terms) = &config else {
        panic!("fixture must be tagged okf-terms");
    };
    let projected = project_okf_terms(dataset.as_ref(), terms).expect("typed projection");
    assert_eq!(projected.report.source_records, 18);
    assert_eq!(projected.report.scoped_quads, 17);
    assert_eq!(projected.report.concepts, 2);
    assert_eq!(projected.report.categories, 2);
    assert_eq!(projected.report.frontmatter_values, 15);
    assert_eq!(projected.report.body_values, 3);
    assert_eq!(projected.report.links, 2);
    assert_eq!(projected.loss_ledger.render_json(), LOSS_LEDGER);

    let expected = [
        ("classes/A.md", CLASS_DOCUMENT),
        ("classes/index.md", CLASS_INDEX),
        ("index.md", ROOT_INDEX),
        ("properties/B.md", PROPERTY_DOCUMENT),
        ("properties/index.md", PROPERTY_INDEX),
    ];
    assert_eq!(
        projected
            .package
            .artifacts()
            .map(|(path, _)| path)
            .collect::<Vec<_>>(),
        expected.iter().map(|(path, _)| *path).collect::<Vec<_>>()
    );
    for (path, body) in expected {
        assert_eq!(projected.package.get(path), Some(body.as_bytes()), "{path}");
    }

    let archive = project_archive(dataset.as_ref(), ProjectionProfile::OkfTerms, &config)
        .expect("unified projection");
    assert_eq!(
        archive.archive,
        projected.package.to_ustar().expect("typed canonical USTAR")
    );
    assert_eq!(
        sha256(&archive.archive),
        "f9509c34d752627e5365edbfe847b08710f1ce8b253dd7d153f2a5bc5b6282d0"
    );
    let decoded = ProjectionPackage::from_ustar(&archive.archive, config.limits())
        .expect("canonical archive decodes");
    assert_eq!(
        decoded.artifacts().collect::<Vec<_>>(),
        projected.package.artifacts().collect::<Vec<_>>()
    );
}

#[test]
fn resident_reordering_and_succinct_pack_are_byte_identical() {
    let dataset = parse_dataset(SOURCE, "application/trig", None).expect("fixture dataset");
    let config = ProjectionConfig::from_json(CONFIG).expect("fixture config");
    let resident = project_archive(dataset.as_ref(), ProjectionProfile::OkfTerms, &config)
        .expect("resident projection");

    let pack = PackBuilder::build_bytes(dataset.as_ref()).expect("succinct pack");
    let view = PackView::from_bytes(&pack).expect("succinct view");
    let succinct =
        project_archive(&view, ProjectionProfile::OkfTerms, &config).expect("succinct projection");
    assert_eq!(succinct.archive, resident.archive);
    assert_eq!(
        succinct.loss_ledger.render_json(),
        resident.loss_ledger.render_json()
    );

    let nquads = serialize_dataset(
        dataset.as_ref(),
        "application/n-quads",
        SerializeGraph::Dataset,
    )
    .expect("serialize fixture");
    let mut lines = String::from_utf8(nquads)
        .expect("N-Quads UTF-8")
        .lines()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    lines.reverse();
    let reversed = parse_dataset(
        format!("{}\n", lines.join("\n")).as_bytes(),
        "application/n-quads",
        None,
    )
    .expect("reversed fixture");
    let reordered = project_archive(reversed.as_ref(), ProjectionProfile::OkfTerms, &config)
        .expect("reordered projection");
    assert_eq!(reordered.archive, resident.archive);
    assert_eq!(
        reordered.loss_ledger.render_json(),
        resident.loss_ledger.render_json()
    );
}
