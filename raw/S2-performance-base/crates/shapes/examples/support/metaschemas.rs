// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The draft 2020-12 meta-schemas the emitter oracles and the observed-behaviour
//! projection tests register with `purrdf-jsonschema`, read once per binary.

use purrdf_lex::json::Value;

/// The draft 2020-12 meta-schemas, parsed on first use.
pub(crate) fn metaschemas() -> &'static purrdf_jsonschema::Metaschemas {
    static SET: std::sync::OnceLock<purrdf_jsonschema::Metaschemas> = std::sync::OnceLock::new();
    SET.get_or_init(|| {
        purrdf_jsonschema::Metaschemas::new(
            purrdf_testkit::jsonschema_metaschemas::DRAFT_2020_12
                .iter()
                .map(|&(uri, text)| {
                    let document: Value = purrdf_lex::json::read(text).expect("meta-schema JSON");
                    (uri, document)
                }),
        )
        .expect("the draft 2020-12 meta-schemas")
    })
}
