// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Format-name resolution for the native text codecs.
//!
//! Accepts both IANA media types and friendly short names, normalising to the exact
//! media-type strings `purrdf::native_codecs` expects. The codecs themselves ride
//! purrdf's wasm-clean native codec stack — no oxigraph Store and no purrdf-gts
//! RDF-codec feature.

/// Resolve a caller-supplied format string to a canonical media type.
///
/// Returns a plain `String` error (NOT a `JsError`) so it is unit-testable on the
/// native build — constructing a `JsError` calls a wasm-only import that panics off
/// wasm. The `#[wasm_bindgen]` call sites map the `String` to a `JsError`.
pub(crate) fn resolve_media_type(format: &str) -> Result<&'static str, String> {
    let normalized = format.trim().to_ascii_lowercase();
    Ok(match normalized.as_str() {
        "text/turtle" | "turtle" | "ttl" => "text/turtle",
        "application/n-triples" | "n-triples" | "ntriples" | "nt" => "application/n-triples",
        "application/n-quads" | "n-quads" | "nquads" | "nq" => "application/n-quads",
        "application/trig" | "trig" => "application/trig",
        "application/rdf+xml" | "rdf+xml" | "rdf/xml" | "rdfxml" => "application/rdf+xml",
        other => {
            return Err(format!(
                "unsupported RDF format {other:?} \
                 (use turtle/ntriples/nquads/trig/rdfxml or their media types)"
            ));
        }
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
        assert_eq!(
            resolve_media_type("rdf/xml").unwrap(),
            "application/rdf+xml"
        );
    }

    #[test]
    fn unknown_format_is_an_error() {
        assert!(resolve_media_type("yaml-ld").is_err());
    }
}
