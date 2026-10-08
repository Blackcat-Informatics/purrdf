// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original native university corpus; its sampling law is in `LUBM_PROFILE.md`.

pub mod check;
mod generate;
mod output;
pub use output::{FileReceipt, Receipt, generate_directory};

/// Identity of the original native sampling and byte law.
pub const PROFILE: &str = "purrdf-lubm-native-v1";

/// Validated configuration, independent of filesystem and process environment.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Spec {
    /// Seed for independent university streams.
    pub seed: u64,
    /// First absolute university index.
    pub index: u64,
    /// Positive number of universities.
    pub universities: u64,
    /// Absolute ontology IRI without fragment, query or trailing `#`.
    pub ontology: String,
    /// Absolute document base ending in `/`, without fragment or query.
    pub document_base: String,
}

impl Spec {
    fn ontology_identity(ontology: &str) -> Result<(), String> {
        identity("ontology", ontology)?;
        if ontology.ends_with('/') {
            return Err("ontology must not end in /".into());
        }
        Ok(())
    }
    /// Admit decimal command-line values through the same configuration boundary.
    /// Leading zeros normalize by value; seed/index retain their full u64 range.
    ///
    /// # Errors
    /// Refuses non-decimal/out-of-range integers and every invalid configuration.
    pub fn from_decimal(
        seed: &str,
        index: &str,
        universities: &str,
        ontology: String,
        document_base: String,
    ) -> Result<Self, String> {
        let uint = |name, value: &str| {
            if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
                return Err(format!("{name} must be unsigned decimal"));
            }
            value
                .parse::<u64>()
                .map_err(|error| format!("invalid {name}: {error}"))
        };
        Self::new(
            uint("seed", seed)?,
            uint("index", index)?,
            uint("universities", universities)?,
            ontology,
            document_base,
        )
    }

    /// Validate the complete configuration before creating any output.
    ///
    /// # Errors
    /// Refuses empty ranges, overflow and incompatible IRI components.
    pub fn new(
        seed: u64,
        index: u64,
        universities: u64,
        ontology: String,
        document_base: String,
    ) -> Result<Self, String> {
        if universities == 0 || index.checked_add(universities).is_none() {
            return Err(
                "university count must be positive and the exclusive range end must fit u64".into(),
            );
        }
        Self::ontology_identity(&ontology)?;
        identity("document base", &document_base)?;
        if !document_base.ends_with('/') || ontology == document_base.trim_end_matches('/') {
            return Err("document base must end in / and remain distinct from the ontology".into());
        }
        Ok(Self {
            seed,
            index,
            universities,
            ontology,
            document_base,
        })
    }
}

struct Draw(u64);
impl Draw {
    fn new(seed: u64, index: u64) -> Self {
        Self(purrdf_hash::mix::splitmix64_finalize(
            seed ^ purrdf_hash::mix::splitmix64_finalize(index),
        ))
    }
    fn between(&mut self, low: usize, high: usize) -> usize {
        let width = u64::try_from(high - low + 1).expect("profile ranges fit u64");
        let threshold = width.wrapping_neg() % width;
        loop {
            let bits = purrdf_hash::mix::splitmix64_next(&mut self.0);
            if bits >= threshold {
                return low + usize::try_from(bits % width).expect("remainder fits usize");
            }
        }
    }
    fn sample(&mut self, length: usize, count: usize) -> Vec<usize> {
        let mut values: Vec<_> = (0..length).collect();
        for at in 0..count {
            let picked = self.between(at, length - 1);
            values.swap(at, picked);
        }
        values.truncate(count);
        values
    }
}

fn identity(name: &str, text: &str) -> Result<(), String> {
    let iri = purrdf_iri::parse(text).map_err(|error| format!("invalid {name}: {error}"))?;
    if !iri.has_scheme() || iri.fragment().is_some() || iri.query().is_some() {
        return Err(format!("{name} must be absolute without query or fragment"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{Draw, Spec};

    #[test]
    fn bounded_draws_reach_both_endpoints_and_selections_are_distinct() {
        for (low, high) in [(0, 0), (7, 10), (15, 25), (1, 2), (0, 999)] {
            let mut draw = Draw::new(0, u64::MAX);
            let mut observed = std::collections::BTreeSet::new();
            for _ in 0..100_000 {
                let value = draw.between(low, high);
                assert!((low..=high).contains(&value));
                observed.insert(value);
            }
            assert_eq!(observed.first(), Some(&low));
            assert_eq!(observed.last(), Some(&high));
        }
        let mut draw = Draw::new(u64::MAX, 1);
        for count in 0..=30 {
            let selection = draw.sample(30, count);
            assert_eq!(selection.len(), count);
            assert_eq!(
                selection
                    .iter()
                    .collect::<std::collections::BTreeSet<_>>()
                    .len(),
                count
            );
        }
    }

    #[test]
    fn public_configuration_revalidation_precedes_directory_creation() {
        let root = purrdf_testkit::TempDir::for_unit_test().unwrap();
        let spec = Spec {
            seed: 0,
            index: 0,
            universities: 0,
            ontology: "relative".into(),
            document_base: "relative".into(),
        };
        assert!(super::generate_directory(&spec, &root.path().join("invalid")).is_err());
        assert!(!root.path().join("invalid").exists());
    }
}
