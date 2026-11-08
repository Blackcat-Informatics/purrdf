// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The checkout's community inventories and independent review evidence.

#![cfg(not(target_arch = "wasm32"))]

#[test]
fn all_catalogued_inventory_schemas_preserve_every_case() {
    use purrdf_conformance_kit::inventory;
    let catalog =
        inventory::decode_catalog(include_bytes!("../../../corpora/community/catalog.json"))
            .unwrap();
    let inventories = [inventory::decode_inventory(include_bytes!(
        "../../../corpora/community/shacl/inventory.json"
    ))
    .unwrap()];
    assert_eq!(
        inventories
            .iter()
            .map(|inventory| inventory.cases.len())
            .sum::<usize>(),
        catalog.case_count
    );
    assert_eq!(catalog.suites.len(), inventories.len());
    let mut executions = std::collections::BTreeMap::<&str, usize>::new();
    for (suite, inventory) in catalog.suites.iter().zip(&inventories) {
        assert_eq!(suite.case_count, inventory.cases.len());
        for case in &inventory.cases {
            let profiles = case.selected_profiles().unwrap();
            assert_ne!(profiles, [] as [&str; 0]);
            for profile in profiles {
                *executions.entry(profile).or_default() += 1;
            }
            if let inventory::Expected::Outcome(spec) = &case.expected
                && matches!(
                    spec.kind.as_str(),
                    "admission-rejection" | "semantic-failure"
                )
            {
                assert!(
                    spec.reason
                        .as_deref()
                        .is_some_and(|reason| !reason.is_empty())
                );
            }
        }
    }
    assert_eq!(
        executions,
        [("shacl-20170720", 57), ("shacl12-20260918", 58)].into()
    );
}

#[test]
fn inventory_operation_and_carrier_cannot_be_changed_independently() {
    use purrdf_lex::json::record::ToJson as _;
    let mut inventory = purrdf_conformance_kit::inventory::decode_inventory(include_bytes!(
        "../../../corpora/community/shacl/inventory.json"
    ))
    .unwrap();
    let json = purrdf_lex::json::write_compact(&inventory.to_json());
    assert_eq!(
        purrdf_conformance_kit::inventory::decode_inventory(json.as_bytes()).unwrap(),
        inventory
    );
    "query-evaluation".clone_into(&mut inventory.cases[0].kind);
    let json = purrdf_lex::json::write_compact(&inventory.to_json());
    assert!(purrdf_conformance_kit::inventory::decode_inventory(json.as_bytes()).is_err());
}

#[test]
fn accepted_oracles_require_exact_input_and_derivation_bytes() {
    use purrdf_conformance_kit::{inventory, reviews};
    use purrdf_lex::json::record::ToJson as _;
    let mut catalog =
        inventory::decode_catalog(include_bytes!("../../../corpora/community/catalog.json"))
            .unwrap();
    catalog.suites.truncate(1);
    catalog.suites[0].case_count = 1;
    catalog.case_count = 1;
    let mut inventory = inventory::decode_inventory(include_bytes!(
        "../../../corpora/community/shacl/inventory.json"
    ))
    .unwrap();
    inventory.cases.truncate(1);
    let case = format!("shacl/{}", inventory.cases[0].id);
    let manifest = format!("{case}/manifest.ttl");
    let shapes = format!("{case}/shapes.ttl");
    let payloads = [
        reviews::Payload {
            path: "shacl/manifest.ttl",
            bytes: b"independent suite manifest",
        },
        reviews::Payload {
            path: &manifest,
            bytes: b"independent manifest, data and expected report",
        },
        reviews::Payload {
            path: &shapes,
            bytes: b"independent shapes",
        },
        reviews::Payload {
            path: "reviews/shacl.md",
            bytes: b"independent derivation and approval",
        },
    ];
    let index = reviews::Index {
        contract_version: 1,
        copyright: "2026 fixture author".into(),
        license: "CC0-1.0".into(),
        cases: vec![reviews::Case {
            id: inventory.cases[0].id.clone(),
            status: "accepted".into(),
            reviewer: "independent fixture reviewer".into(),
            derivation: "reviews/shacl.md".into(),
            payloads: payloads
                .iter()
                .map(|payload| reviews::Artifact {
                    path: payload.path.to_owned(),
                    blake3: purrdf_hash::hex::Digest32::new(
                        *purrdf_hash::blake3::hash(payload.bytes).as_bytes(),
                    )
                    .to_hex(),
                })
                .collect(),
        }],
    };
    let json = purrdf_lex::json::write_compact(&index.to_json());
    assert_eq!(reviews::decode(json.as_bytes()).unwrap(), index);
    let inventories = [inventory];
    reviews::admit(&catalog, &inventories, &index, &payloads).unwrap();
    for position in [1, 2, 3] {
        let mut changed = payloads;
        changed[position].bytes = b"changed after approval";
        assert!(reviews::admit(&catalog, &inventories, &index, &changed).is_err());
    }
    assert!(reviews::admit(&catalog, &inventories, &index, &payloads[..3]).is_err());
    let mut wrong = index.clone();
    "unreviewed".clone_into(&mut wrong.cases[0].status);
    assert!(reviews::admit(&catalog, &inventories, &wrong, &payloads).is_err());
    let mut wrong = index.clone();
    wrong.cases[0].payloads.pop();
    assert!(reviews::admit(&catalog, &inventories, &wrong, &payloads).is_err());
    let mut wrong = index.clone();
    "catalog.json".clone_into(&mut wrong.cases[0].derivation);
    "catalog.json".clone_into(&mut wrong.cases[0].payloads[3].path);
    assert!(reviews::admit(&catalog, &inventories, &wrong, &payloads).is_err());
    let mut wrong = index.clone();
    "../outside.md".clone_into(&mut wrong.cases[0].derivation);
    assert!(reviews::admit(&catalog, &inventories, &wrong, &payloads).is_err());
    let mut wrong = index.clone();
    wrong.cases.push(wrong.cases[0].clone());
    assert!(reviews::admit(&catalog, &inventories, &wrong, &payloads).is_err());
    let mut wrong = index.clone();
    "other-case".clone_into(&mut wrong.cases[0].id);
    assert!(reviews::admit(&catalog, &inventories, &wrong, &payloads).is_err());
    let mut wrong = index.clone();
    wrong.cases[0].payloads[0].blake3.make_ascii_uppercase();
    assert!(reviews::admit(&catalog, &inventories, &wrong, &payloads).is_err());
    let mut wrong = index;
    "shacl/another-case/shapes.ttl".clone_into(&mut wrong.cases[0].payloads[2].path);
    assert!(reviews::admit(&catalog, &inventories, &wrong, &payloads).is_err());
}
