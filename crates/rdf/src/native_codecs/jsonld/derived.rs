// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Deterministic, vocabulary-neutral context derivation from typed carrier IRI slots.

use std::collections::{BTreeMap, HashSet};
use std::hash::BuildHasherDefault;

use super::carrier::{Document, Node, Term, Value};
use super::{CompiledJsonLdContext, RdfDiagnostic};

const MAX_DISTINCT_IRIS: usize = 65_536;
const MAX_NAMESPACE_CANDIDATES: usize = 4_096;
const MAX_DERIVED_TERMS: usize = 4_096;
const MAX_DERIVED_CONTEXT_BYTES: usize = 1_048_576;
const MAX_DERIVATION_WORK: usize = 262_144;

type FixedHashSet<T> = HashSet<T, BuildHasherDefault<ahash::AHasher>>;

#[derive(Debug, Clone, Copy)]
struct DerivationLimits {
    distinct_iris: usize,
    namespace_candidates: usize,
    terms: usize,
    context_bytes: usize,
    work: usize,
}

impl Default for DerivationLimits {
    fn default() -> Self {
        Self {
            distinct_iris: MAX_DISTINCT_IRIS,
            namespace_candidates: MAX_NAMESPACE_CANDIDATES,
            terms: MAX_DERIVED_TERMS,
            context_bytes: MAX_DERIVED_CONTEXT_BYTES,
            work: MAX_DERIVATION_WORK,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct CandidateStats {
    expanded_bytes: usize,
    suffix_bytes: usize,
    occurrences: usize,
}

impl CandidateStats {
    fn record(&mut self, iri: &str, namespace: &str) -> Result<(), RdfDiagnostic> {
        self.expanded_bytes = self
            .expanded_bytes
            .checked_add(iri.len())
            .ok_or_else(|| derived_limit("derived namespace expanded-byte count overflow"))?;
        self.suffix_bytes = self
            .suffix_bytes
            .checked_add(iri.len() - namespace.len())
            .ok_or_else(|| derived_limit("derived namespace suffix-byte count overflow"))?;
        self.occurrences = self
            .occurrences
            .checked_add(1)
            .ok_or_else(|| derived_limit("derived namespace occurrence count overflow"))?;
        Ok(())
    }

    fn is_profitable(self, alias: &str, namespace: &str) -> Result<bool, RdfDiagnostic> {
        let compacted_bytes = self
            .occurrences
            .checked_mul(alias.len() + 1)
            .and_then(|bytes| bytes.checked_add(self.suffix_bytes))
            .ok_or_else(|| derived_limit("derived namespace compacted-byte count overflow"))?;
        let context_cost = prefix_definition_cost(alias, namespace)?;
        Ok(self
            .expanded_bytes
            .checked_sub(compacted_bytes)
            .is_some_and(|saving| saving > context_cost))
    }
}

struct Collector {
    limits: DerivationLimits,
    distinct: FixedHashSet<String>,
    reserved_schemes: FixedHashSet<String>,
    candidates: BTreeMap<String, CandidateStats>,
    work: usize,
}

impl Collector {
    fn new(limits: DerivationLimits) -> Self {
        Self {
            limits,
            distinct: FixedHashSet::default(),
            reserved_schemes: FixedHashSet::default(),
            candidates: BTreeMap::new(),
            work: 0,
        }
    }

    fn record(&mut self, iri: &str) -> Result<(), RdfDiagnostic> {
        self.work = self
            .work
            .checked_add(1)
            .ok_or_else(|| derived_limit("derived-context work count overflow"))?;
        if self.work > self.limits.work {
            return Err(derived_limit(format!(
                "derived-context analysis exceeds {} IRI-slot operations",
                self.limits.work
            )));
        }
        if iri.starts_with("_:") {
            return Ok(());
        }
        let parsed = purrdf_iri::parse(iri).map_err(|source| {
            derived_invalid(format!("cannot derive from invalid IRI `{iri}`: {source}"))
        })?;
        if !parsed.has_scheme() {
            return Err(derived_invalid(format!(
                "cannot derive from relative IRI `{iri}`"
            )));
        }
        if !self.distinct.contains(iri) {
            if self.distinct.len() == self.limits.distinct_iris {
                return Err(derived_limit(format!(
                    "derived-context analysis exceeds {} distinct IRIs",
                    self.limits.distinct_iris
                )));
            }
            self.distinct.insert(iri.to_owned());
            self.reserved_schemes.insert(
                parsed
                    .scheme()
                    .expect("absolute IRI has a scheme")
                    .to_ascii_lowercase(),
            );
        }
        let Some(namespace) = namespace_boundary(iri) else {
            return Ok(());
        };
        if !self.candidates.contains_key(namespace) {
            if self.candidates.len() == self.limits.namespace_candidates {
                return Err(derived_limit(format!(
                    "derived-context analysis exceeds {} namespace candidates",
                    self.limits.namespace_candidates
                )));
            }
            self.candidates
                .insert(namespace.to_owned(), CandidateStats::default());
        }
        self.candidates
            .get_mut(namespace)
            .expect("candidate was inserted or already present")
            .record(iri, namespace)
    }

    fn finish(self) -> Result<CompiledJsonLdContext, RdfDiagnostic> {
        let mut prefixes = Vec::new();
        let mut alias_index = 0usize;
        for (namespace, stats) in self.candidates {
            while self.reserved_schemes.contains(&format!("ns{alias_index}")) {
                alias_index = alias_index
                    .checked_add(1)
                    .ok_or_else(|| derived_limit("derived alias index overflow"))?;
            }
            let alias = format!("ns{alias_index}");
            if stats.is_profitable(&alias, &namespace)? {
                prefixes.push((alias, namespace));
                alias_index = alias_index
                    .checked_add(1)
                    .ok_or_else(|| derived_limit("derived alias index overflow"))?;
            }
        }
        if prefixes.len() > self.limits.terms {
            return Err(derived_limit(format!(
                "derived context requires {} terms; limit is {}",
                prefixes.len(),
                self.limits.terms
            )));
        }
        let validation_work = self
            .distinct
            .len()
            .checked_mul(prefixes.len())
            .and_then(|work| work.checked_add(self.work))
            .ok_or_else(|| derived_limit("derived-context validation work count overflow"))?;
        if validation_work > self.limits.work {
            return Err(derived_limit(format!(
                "derived-context analysis and validation require {validation_work} operations; limit is {}",
                self.limits.work
            )));
        }
        let context = CompiledJsonLdContext::from_prefixes(prefixes)?;
        let bytes = context.canonical_json().len();
        if bytes > self.limits.context_bytes {
            return Err(derived_limit(format!(
                "derived context is {bytes} bytes; limit is {}",
                self.limits.context_bytes
            )));
        }
        let mut distinct = self.distinct.iter().collect::<Vec<_>>();
        distinct.sort_unstable();
        for iri in distinct {
            let compacted = context.compact_iri(iri, true)?;
            let expanded = context
                .expand_iri(&compacted, true, false)?
                .ok_or_else(|| derived_invalid("derived compact IRI has a null mapping"))?;
            if expanded != *iri {
                return Err(derived_invalid(format!(
                    "derived compact IRI `{compacted}` does not round-trip `{iri}`"
                )));
            }
        }
        Ok(context)
    }
}

pub(super) fn derive_context(document: &Document) -> Result<CompiledJsonLdContext, RdfDiagnostic> {
    let mut collector = Collector::new(DerivationLimits::default());
    collect_document(document, &mut collector)?;
    collector.finish()
}

fn collect_document(document: &Document, collector: &mut Collector) -> Result<(), RdfDiagnostic> {
    for node in &document.default_nodes {
        collect_node(node, collector)?;
    }
    for graph in &document.named_graphs {
        collector.record(&graph.id)?;
        for node in &graph.nodes {
            collect_node(node, collector)?;
        }
    }
    Ok(())
}

fn collect_node(node: &Node, collector: &mut Collector) -> Result<(), RdfDiagnostic> {
    collector.record(&node.id)?;
    for rdf_type in &node.types {
        collector.record(rdf_type)?;
    }
    for (predicate, values) in node.properties.iter().chain(node.reverse_properties.iter()) {
        collector.record(predicate)?;
        for value in values {
            collect_value(value, collector)?;
        }
    }
    Ok(())
}

fn collect_value(value: &Value, collector: &mut Collector) -> Result<(), RdfDiagnostic> {
    collect_term(&value.term, collector)?;
    for annotation in &value.annotations {
        collect_node(annotation, collector)?;
    }
    Ok(())
}

fn collect_term(term: &Term, collector: &mut Collector) -> Result<(), RdfDiagnostic> {
    match term {
        Term::Id(iri) => collector.record(iri),
        Term::Literal(literal) => {
            if let Some(datatype) = &literal.datatype {
                collector.record(datatype)?;
            }
            Ok(())
        }
        Term::Triple(triple) => {
            collect_term(&triple.subject, collector)?;
            collector.record(&triple.predicate)?;
            collect_term(&triple.object, collector)
        }
        Term::List(values) => {
            for value in values {
                collect_value(value, collector)?;
            }
            Ok(())
        }
    }
}

fn namespace_boundary(iri: &str) -> Option<&str> {
    let parsed = purrdf_iri::parse(iri).ok()?;
    if !parsed.has_scheme() || parsed.query().is_some() {
        return None;
    }
    let scheme_end = parsed.scheme()?.len() + 1;
    let boundary = iri
        .rfind('#')
        .filter(|index| *index >= scheme_end)
        .or_else(|| iri.rfind('/').filter(|index| *index >= scheme_end))
        .or_else(|| iri.rfind(':').filter(|index| *index >= scheme_end))?;
    let split = boundary.checked_add(1)?;
    let suffix = iri.get(split..)?;
    if suffix.is_empty() || suffix.starts_with("//") || suffix.chars().any(char::is_whitespace) {
        return None;
    }
    let namespace = iri.get(..split)?;
    purrdf_iri::parse(namespace)
        .is_ok_and(|parsed| parsed.has_scheme())
        .then_some(namespace)
}

fn prefix_definition_cost(alias: &str, namespace: &str) -> Result<usize, RdfDiagnostic> {
    let value = serde_json::json!({alias: {"@id": namespace, "@prefix": true}});
    serde_json::to_vec(&value)
        .map(|bytes| bytes.len())
        .map_err(|source| derived_invalid(format!("encode derived prefix definition: {source}")))
}

fn derived_limit(message: impl Into<String>) -> RdfDiagnostic {
    RdfDiagnostic::error("jsonld-derived-limit", message)
}

fn derived_invalid(message: impl Into<String>) -> RdfDiagnostic {
    RdfDiagnostic::error("jsonld-derived-invalid", message)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn derive_from<'a>(
        iris: impl IntoIterator<Item = &'a str>,
        limits: DerivationLimits,
    ) -> Result<CompiledJsonLdContext, RdfDiagnostic> {
        let mut collector = Collector::new(limits);
        for iri in iris {
            collector.record(iri)?;
        }
        collector.finish()
    }

    fn repeated(namespace: &str, count: usize) -> Vec<String> {
        (0..count)
            .map(|index| format!("{namespace}term{index}"))
            .collect()
    }

    #[test]
    fn safe_boundaries_cover_fragment_path_and_urn_namespaces() {
        assert_eq!(
            namespace_boundary("https://example.org/vocab#Term"),
            Some("https://example.org/vocab#")
        );
        assert_eq!(
            namespace_boundary("https://example.org/vocab/Term"),
            Some("https://example.org/vocab/")
        );
        assert_eq!(namespace_boundary("urn:example:Term"), Some("urn:example:"));
        assert_eq!(namespace_boundary("https://example.org/"), None);
        assert_eq!(namespace_boundary("https://example.org/x?q=1"), None);
    }

    #[test]
    fn aliases_are_sorted_neutral_profitable_and_vocab_free() {
        let mut iris = repeated("https://z.example/vocab/", 32);
        iris.extend(repeated("https://a.example/vocab#", 32));
        iris.reverse();
        let borrowed = iris.iter().map(String::as_str).collect::<Vec<_>>();
        let context = derive_from(borrowed.iter().copied(), DerivationLimits::default())
            .expect("derive context");
        assert_eq!(context.vocab_mapping(), None);
        assert_eq!(
            context.canonical_context()["ns0"]["@id"],
            "https://a.example/vocab#"
        );
        assert_eq!(
            context.canonical_context()["ns1"]["@id"],
            "https://z.example/vocab/"
        );
        assert_eq!(context.canonical_context()["ns0"]["@prefix"], true);

        let mut sorted = borrowed.clone();
        sorted.sort_unstable();
        let reordered = derive_from(sorted, DerivationLimits::default()).expect("reordered");
        assert_eq!(context.canonical_json(), reordered.canonical_json());
    }

    #[test]
    fn unprofitable_candidates_are_discarded_stably() {
        let context = derive_from(["https://example.org/only"], DerivationLimits::default())
            .expect("derive empty context");
        assert_eq!(context.canonical_context(), &serde_json::json!({}));
    }

    #[test]
    fn distinct_candidate_and_work_limits_accept_boundary_and_reject_one_over() {
        let limits = DerivationLimits {
            distinct_iris: 2,
            namespace_candidates: 2,
            terms: 2,
            context_bytes: MAX_DERIVED_CONTEXT_BYTES,
            work: 2,
        };
        derive_from(["https://a.example/x", "https://b.example/y"], limits)
            .expect("exact boundaries");
        for constrained in [
            DerivationLimits {
                distinct_iris: 1,
                ..limits
            },
            DerivationLimits {
                namespace_candidates: 1,
                ..limits
            },
            DerivationLimits { work: 1, ..limits },
        ] {
            let error = derive_from(["https://a.example/x", "https://b.example/y"], constrained)
                .expect_err("one over limit");
            assert_eq!(error.code, "jsonld-derived-limit");
        }
    }

    #[test]
    fn term_and_context_byte_limits_are_exact() {
        let first = repeated("https://a.example/a/", 32);
        let second = repeated("https://b.example/b/", 32);
        let iris = first
            .iter()
            .chain(&second)
            .map(String::as_str)
            .collect::<Vec<_>>();
        let context = derive_from(iris.iter().copied(), DerivationLimits::default())
            .expect("derive two terms");
        let exact_bytes = context.canonical_json().len();
        derive_from(
            iris.iter().copied(),
            DerivationLimits {
                terms: 2,
                context_bytes: exact_bytes,
                ..DerivationLimits::default()
            },
        )
        .expect("exact term and byte boundary");
        for limits in [
            DerivationLimits {
                terms: 1,
                ..DerivationLimits::default()
            },
            DerivationLimits {
                context_bytes: exact_bytes - 1,
                ..DerivationLimits::default()
            },
        ] {
            let error = derive_from(iris.iter().copied(), limits).expect_err("one over limit");
            assert_eq!(error.code, "jsonld-derived-limit");
        }
    }

    #[test]
    fn aliases_skip_schemes_present_in_the_dataset() {
        let mut iris = repeated("https://example.org/vocab/", 32);
        iris.push("ns0:external".to_owned());
        let context = derive_from(iris.iter().map(String::as_str), DerivationLimits::default())
            .expect("derive collision-free context");
        assert!(context.canonical_context().get("ns0").is_none());
        assert_eq!(
            context.canonical_context()["ns1"]["@id"],
            "https://example.org/vocab/"
        );
        assert_eq!(
            context
                .expand_iri(
                    &context
                        .compact_iri("ns0:external", true)
                        .expect("compact absolute IRI"),
                    true,
                    false,
                )
                .expect("expand compact IRI"),
            Some("ns0:external".to_owned())
        );
    }

    #[test]
    fn validation_work_limit_charges_every_iri_prefix_pair() {
        let iris = repeated("https://example.org/vocab/", 32);
        let exact = DerivationLimits {
            work: 64,
            ..DerivationLimits::default()
        };
        derive_from(iris.iter().map(String::as_str), exact).expect("exact work boundary");
        let error = derive_from(
            iris.iter().map(String::as_str),
            DerivationLimits { work: 63, ..exact },
        )
        .expect_err("validation work exceeds limit by one");
        assert_eq!(error.code, "jsonld-derived-limit");
    }
}
