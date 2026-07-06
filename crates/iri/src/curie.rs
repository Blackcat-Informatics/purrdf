// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! CURIE (Compact URI) ↔ IRI expansion/contraction.
//!
//! This subsumes the hand-rolled `curie_prefix` / `resolve_iri` logic in
//! `crates/rdf-core/src/sssom.rs` (a later PR deletes those duplicates). The
//! load-bearing semantics carried over verbatim:
//!
//! * A CURIE is `prefix:reference` with a **non-empty** prefix whose reference does
//!   **not** start with `//` — that guard prevents an absolute IRI (`http://…`)
//!   from being mistaken for an `http:` CURIE.
//! * Expanding an **undeclared** prefix yields the entity **verbatim** (greenfield
//!   best-effort; prefix completeness is a validator's concern, not this layer's).

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
    let mut best: Option<(&str, &str)> = None; // (prefix, namespace)
    for (prefix, namespace) in &prefixes.map {
        // Skip an empty prefix: it would produce a leading-colon ":X" that
        // `curie_prefix` rejects, breaking the contract->expand round-trip.
        if prefix.is_empty() {
            continue;
        }
        if !namespace.is_empty() && iri.starts_with(namespace.as_str()) {
            match best {
                Some((_, ns)) if ns.len() >= namespace.len() => {}
                _ => best = Some((prefix, namespace)),
            }
        }
    }
    let (prefix, namespace) = best?;
    Some(format!("{prefix}:{}", &iri[namespace.len()..]))
}
