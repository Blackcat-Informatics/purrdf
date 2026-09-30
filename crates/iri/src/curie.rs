// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! CURIE (Compact URI) ↔ IRI expansion/contraction, and the namespace/local-name
//! split of an IRI.
//!
//! The load-bearing semantics:
//!
//! * A CURIE is `prefix:reference` with a **non-empty** prefix whose reference does
//!   **not** start with `//` — that guard prevents an absolute IRI (`http://…`)
//!   from being mistaken for an `http:` CURIE.
//! * Expanding an **undeclared** prefix yields the entity **verbatim** (greenfield
//!   best-effort; prefix completeness is a validator's concern, not this layer's).
//! * An IRI's **local name** is its longest suffix holding none of `#`, `/` and `:`
//!   ([`split_local_name`]).

use std::collections::BTreeMap;

/// A prefix → namespace-IRI map. `BTreeMap` for deterministic iteration (the
/// contraction longest-match must be reproducible).
///
/// # Examples
///
/// ```rust
/// use purrdf_iri::PrefixMap;
///
/// let mut prefixes = PrefixMap::new();
/// prefixes.insert("ex", "http://example.org/ns#");
/// assert_eq!(prefixes.get("ex"), Some("http://example.org/ns#"));
/// assert_eq!(prefixes.get("undeclared"), None);
/// assert_eq!(prefixes.len(), 1);
///
/// // Also constructible from an iterator of pairs.
/// let same: PrefixMap = [("ex", "http://example.org/ns#")].into_iter().collect();
/// assert_eq!(same, prefixes);
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PrefixMap {
    map: BTreeMap<String, String>,
}

impl PrefixMap {
    /// An empty map.
    pub fn new() -> Self {
        Self {
            map: BTreeMap::new(),
        }
    }

    /// Bind `prefix` to `namespace` (replacing any existing binding).
    pub fn insert(&mut self, prefix: impl Into<String>, namespace: impl Into<String>) {
        self.map.insert(prefix.into(), namespace.into());
    }

    /// The namespace bound to `prefix`, if any.
    pub fn get(&self, prefix: &str) -> Option<&str> {
        self.map.get(prefix).map(String::as_str)
    }

    /// Number of bindings.
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// `true` iff there are no bindings.
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

impl<K, V> FromIterator<(K, V)> for PrefixMap
where
    K: Into<String>,
    V: Into<String>,
{
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        let mut pm = Self::new();
        for (k, v) in iter {
            pm.insert(k, v);
        }
        pm
    }
}

/// The CURIE prefix of `entity`, or `None` if it is not a CURIE.
///
/// Mirrors `sssom::curie_prefix`: non-empty prefix, reference not starting `//`.
///
/// # Examples
///
/// ```rust
/// use purrdf_iri::curie_prefix;
///
/// assert_eq!(curie_prefix("ex:Thing"), Some("ex"));
/// // An absolute IRI is NOT mistaken for an `http:` CURIE (`//` guard).
/// assert_eq!(curie_prefix("http://example.org/Thing"), None);
/// // A leading colon has an empty prefix, so it is not a CURIE either.
/// assert_eq!(curie_prefix(":Thing"), None);
/// ```
pub fn curie_prefix(entity: &str) -> Option<&str> {
    let idx = entity.find(':')?;
    let prefix = &entity[..idx];
    if prefix.is_empty() {
        return None;
    }
    if entity[idx + 1..].starts_with("//") {
        return None;
    }
    Some(prefix)
}

/// Expand a CURIE against `prefixes`. Returns `Some(absolute-iri)` only when
/// `entity` is a CURIE **and** its prefix is declared; otherwise `None`.
///
/// Use [`resolve`] for the verbatim-fallback behavior that matches the SSSOM
/// serializer (`resolve_iri`): "expand if possible, else pass through unchanged".
///
/// # Examples
///
/// ```rust
/// use purrdf_iri::{PrefixMap, expand_curie};
///
/// let mut prefixes = PrefixMap::new();
/// prefixes.insert("ex", "http://example.org/ns#");
///
/// assert_eq!(
///     expand_curie("ex:Thing", &prefixes),
///     Some("http://example.org/ns#Thing".to_owned())
/// );
/// // Undeclared prefix → `None` (a semantic signal, not an error).
/// assert_eq!(expand_curie("other:Thing", &prefixes), None);
/// // Not a CURIE at all (absolute IRI) → `None`.
/// assert_eq!(expand_curie("http://example.org/ns#Thing", &prefixes), None);
/// ```
pub fn expand_curie(entity: &str, prefixes: &PrefixMap) -> Option<String> {
    let prefix = curie_prefix(entity)?;
    let namespace = prefixes.get(prefix)?;
    let reference = &entity[prefix.len() + 1..];
    Some(format!("{namespace}{reference}"))
}

/// Resolve `entity` to an IRI string: expand a declared CURIE, else return the
/// entity verbatim. This is the exact behavior of `sssom::resolve_iri`.
///
/// # Examples
///
/// ```rust
/// use purrdf_iri::{PrefixMap, resolve};
///
/// let mut prefixes = PrefixMap::new();
/// prefixes.insert("ex", "http://example.org/ns#");
///
/// assert_eq!(resolve("ex:Thing", &prefixes), "http://example.org/ns#Thing");
/// // Anything that cannot be expanded passes through verbatim.
/// assert_eq!(resolve("other:Thing", &prefixes), "other:Thing");
/// assert_eq!(
///     resolve("http://example.org/plain", &prefixes),
///     "http://example.org/plain"
/// );
/// ```
pub fn resolve(entity: &str, prefixes: &PrefixMap) -> String {
    expand_curie(entity, prefixes).unwrap_or_else(|| entity.to_owned())
}

/// Contract an absolute IRI to a CURIE using the **longest** matching namespace
/// (ties broken by prefix name, deterministically). Returns `None` if no declared
/// namespace is a prefix of `iri`.
///
/// # Examples
///
/// ```rust
/// use purrdf_iri::{PrefixMap, contract};
///
/// let mut prefixes = PrefixMap::new();
/// prefixes.insert("ex", "http://example.org/");
/// prefixes.insert("exns", "http://example.org/ns#");
///
/// // The LONGEST matching namespace wins.
/// assert_eq!(
///     contract("http://example.org/ns#Thing", &prefixes),
///     Some("exns:Thing".to_owned())
/// );
/// assert_eq!(
///     contract("http://example.org/other", &prefixes),
///     Some("ex:other".to_owned())
/// );
/// // No declared namespace matches → `None`.
/// assert_eq!(contract("https://example.org/x", &prefixes), None);
/// ```
pub fn contract(iri: &str, prefixes: &PrefixMap) -> Option<String> {
    // An empty prefix would produce a leading-colon ":X" that `curie_prefix`
    // rejects, breaking the contract->expand round-trip.
    contract_where(iri, prefixes, |prefix, _| !prefix.is_empty())
}

/// [`contract`] for a syntax with its own prefixed-name grammar: `iri` as
/// `prefix:local` under the longest declared namespace whose `(prefix, local)`
/// split `accepts` admits, or `None` when no namespace yields an admitted split.
///
/// The grammar is the caller's because it is the syntax's: Turtle and SPARQL
/// admit the empty prefix and require `local` to be a `PN_LOCAL`, RDF/XML
/// requires an XML `NCName`. When the longest namespace leaves a local part the
/// grammar refuses, the next-longest is tried, so a shorter namespace that
/// yields a valid name still compacts. Among equally long namespaces the
/// lexicographically first prefix wins, so the result never depends on
/// insertion order.
///
/// # Examples
///
/// ```rust
/// use purrdf_iri::{PrefixMap, contract_where};
///
/// let mut prefixes = PrefixMap::new();
/// prefixes.insert("", "http://example.org/");
/// prefixes.insert("n", "http://example.org/n/");
/// let digit_free = |_: &str, local: &str| !local.starts_with(|c: char| c.is_ascii_digit());
///
/// // The empty prefix is a prefix when the grammar admits it.
/// assert_eq!(
///     contract_where("http://example.org/a", &prefixes, digit_free),
///     Some(":a".to_owned())
/// );
/// // The longest namespace leaves "1x", which the grammar refuses, so the
/// // shorter one is tried.
/// assert_eq!(
///     contract_where("http://example.org/n/1x", &prefixes, digit_free),
///     Some(":n/1x".to_owned())
/// );
/// ```
pub fn contract_where(
    iri: &str,
    prefixes: &PrefixMap,
    accepts: impl Fn(&str, &str) -> bool,
) -> Option<String> {
    let mut best: Option<(&str, &str)> = None; // (prefix, namespace)
    for (prefix, namespace) in &prefixes.map {
        let Some(local) = iri.strip_prefix(namespace.as_str()) else {
            continue;
        };
        if namespace.is_empty() || !accepts(prefix, local) {
            continue;
        }
        match best {
            Some((best_prefix, ns))
                if ns.len() > namespace.len()
                    || (ns.len() == namespace.len() && best_prefix <= prefix.as_str()) => {}
            _ => best = Some((prefix, namespace)),
        }
    }
    let (prefix, namespace) = best?;
    Some(format!("{prefix}:{}", &iri[namespace.len()..]))
}

/// Split `iri` into `(namespace, local)` at its local-name boundary: `local` is the
/// longest suffix that contains none of `#`, `/` and `:`, and `namespace` is everything
/// before it, delimiter included. `namespace + local == iri` always.
///
/// Those three are the delimiters that end a namespace in the three shapes an IRI
/// vocabulary takes: a hash namespace (`…/ns#Term`), a slash namespace (`…/ns/Term`)
/// and a URN or CURIE-like colon namespace (`urn:ex:Term`, `ex:Term`). Splitting after
/// the LAST of any of them means the local part never carries one, which is what every
/// consumer of a local name needs of it: a SPARQL variable, a JSON Schema or LinkML
/// identifier, a file stem, a display label. An IRI that ends in a delimiter has an
/// empty local name, and one with no delimiter at all is all local name.
///
/// This is the one namespace/local split in the workspace; a syntax with its own name
/// grammar (an RDF/XML element name must be an XML `NCName`) searches for its own split
/// instead.
///
/// # Examples
///
/// ```rust
/// use purrdf_iri::split_local_name;
///
/// assert_eq!(
///     split_local_name("http://example.org/ns#Term"),
///     ("http://example.org/ns#", "Term")
/// );
/// assert_eq!(
///     split_local_name("http://example.org/ns/Term"),
///     ("http://example.org/ns/", "Term")
/// );
/// assert_eq!(split_local_name("urn:example:Term"), ("urn:example:", "Term"));
/// assert_eq!(split_local_name("http://example.org/ns#"), ("http://example.org/ns#", ""));
/// assert_eq!(split_local_name("Term"), ("", "Term"));
/// ```
#[must_use]
pub fn split_local_name(iri: &str) -> (&str, &str) {
    let boundary = iri.rfind(['#', '/', ':']).map_or(0, |index| index + 1);
    iri.split_at(boundary)
}

/// The local name of `iri`: the second half of [`split_local_name`].
///
/// # Examples
///
/// ```rust
/// use purrdf_iri::local_name;
///
/// assert_eq!(local_name("http://example.org/ns#requiredParam"), "requiredParam");
/// assert_eq!(local_name("http://example.org/ns/requiredParam"), "requiredParam");
/// assert_eq!(local_name("ex:requiredParam"), "requiredParam");
/// ```
#[must_use]
pub fn local_name(iri: &str) -> &str {
    split_local_name(iri).1
}

#[cfg(test)]
mod tests {
    use super::{local_name, split_local_name};

    #[test]
    fn the_local_name_follows_the_last_hash_slash_or_colon() {
        for (iri, local) in [
            ("http://example.org/ns#Term", "Term"),
            ("http://example.org/ns/Term", "Term"),
            ("https://example.org/a/b/c", "c"),
            ("urn:example:Term", "Term"),
            ("ex:requiredParam", "requiredParam"),
            // A slash inside a fragment, and a colon after the last slash, both split.
            ("http://example.org/ns#a/b", "b"),
            ("http://example.org/a:b", "b"),
            ("http://example.org/a#b:c", "c"),
            // Non-ASCII local names are kept whole.
            ("http://example.org/ns#caf\u{e9}", "caf\u{e9}"),
            ("http://example.org/\u{732b}", "\u{732b}"),
        ] {
            assert_eq!(local_name(iri), local, "{iri}");
        }
    }

    #[test]
    fn a_trailing_delimiter_leaves_an_empty_local_name() {
        for iri in [
            "http://example.org/ns#",
            "http://example.org/ns/",
            "urn:example:",
            "http://example.org/",
        ] {
            assert_eq!(local_name(iri), "", "{iri}");
        }
    }

    #[test]
    fn a_name_without_a_delimiter_is_all_local_name() {
        assert_eq!(split_local_name("Term"), ("", "Term"));
        assert_eq!(split_local_name(""), ("", ""));
    }

    #[test]
    fn the_two_halves_always_concatenate_back_to_the_iri() {
        for iri in [
            "http://example.org/ns#Term",
            "http://example.org/ns/",
            "urn:a:b:c",
            "Term",
            "",
            "#",
            "a#b/c:d",
        ] {
            let (namespace, local) = split_local_name(iri);
            assert_eq!(format!("{namespace}{local}"), iri);
            assert!(!local.contains(['#', '/', ':']), "{iri}");
        }
    }
}
