// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Format-name resolution for the native text codecs.
//!
//! Delegates to the ONE core registry (`purrdf::classify`), so the wasm surface accepts
//! exactly the spellings every other first-party surface does — including JSON-LD and
//! YAML-LD — and there is no second, drifting format table. The codecs ride purrdf's
//! wasm-clean native codec stack — no oxigraph Store and no purrdf-gts RDF-codec feature.

/// Resolve a caller-supplied format string to a canonical media type.
///
/// Returns a plain `String` error (NOT a `JsError`) so it is unit-testable on the
/// native build — constructing a `JsError` calls a wasm-only import that panics off
/// wasm. The `#[wasm_bindgen]` call sites map the `String` to a `JsError`. `classify`
/// already lowercases and strips any `;charset=…` parameter.
pub(crate) fn resolve_media_type(format: &str) -> Result<&'static str, String> {
    purrdf::classify(format)
        .map(purrdf::NativeRdfFormat::media_type)
        .map_err(|_| {
            format!(
                "unsupported RDF format {format:?} (use turtle/ntriples/nquads/trig/rdfxml/\
                 jsonld/yamlld or their media types)"
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_names_and_media_types_resolve() {
        assert_eq!(resolve_media_type("ttl").unwrap(), "text/turtle");
        assert_eq!(resolve_media_type("Turtle").unwrap(), "text/turtle");
        assert_eq!(resolve_media_type("text/turtle").unwrap(), "text/turtle");
        assert_eq!(resolve_media_type("nq").unwrap(), "application/n-quads");
        // Both RDF/XML spellings the wasm resolver historically accepted still resolve
        // (absorbed into the core registry), so delegation drops nothing.
        assert_eq!(
            resolve_media_type("rdf/xml").unwrap(),
            "application/rdf+xml"
        );
        assert_eq!(resolve_media_type("rdfxml").unwrap(), "application/rdf+xml");
    }

    #[test]
    fn jsonld_and_yamlld_now_resolve() {
        assert_eq!(resolve_media_type("jsonld").unwrap(), "application/ld+json");
        assert_eq!(
            resolve_media_type("application/ld+json").unwrap(),
            "application/ld+json"
        );
        assert_eq!(
            resolve_media_type("yaml-ld").unwrap(),
            "application/ld+yaml"
        );
        assert_eq!(resolve_media_type("yamlld").unwrap(), "application/ld+yaml");
    }

    #[test]
    fn unknown_format_is_an_error() {
        assert!(resolve_media_type("application/json").is_err());
    }
}
