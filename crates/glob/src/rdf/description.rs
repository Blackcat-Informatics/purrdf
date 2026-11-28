// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Portable checked descriptions of compiled RDF plans, without executable bytes.

use super::{
    Account, Admission, DESCRIPTION_VERSION, DatasetHandle, Error, ExportError, FIRST,
    LANGUAGE_VERSION, Memory, Mode, NIL, PreparedPlan, REST, RdfDataset, RdfDatasetBuilder, Reason,
    SURFACES, Schema, SoleObject, StorageError, TYPE, Vocabulary, check_fields, sole, sources,
    text,
};
use purrdf_core::{TermId, TermRef};
use purrdf_hash::{
    Domain, blake3,
    frame::frame_le_into,
    hex::{Digest32, encode_to_slice},
};

const DOMAIN: Domain = Domain::new(b"purrdf-glob/compiled-description/v1");

/// An explicitly selected version of the compiled source checksum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Law {
    /// Length-framed authored source occurrences with BLAKE3-256.
    Blake3V1,
}

impl Law {
    /// Exact version spelling stored in the compiled description.
    pub const fn version(self) -> &'static str {
        match self {
            Self::Blake3V1 => "purrdf-glob-compiled-blake3-v1",
        }
    }
}

/// Caller-provided identity, vocabulary and checksum law.
#[derive(Debug, Clone, Copy)]
pub struct Description<'a> {
    /// Exact absolute IRI identifying the compiled description.
    pub identity: &'a str,
    /// Absolute IRI used as the description's class.
    pub class: &'a str,
    /// Absolute predicate carrying the checksum literal.
    pub digest_predicate: &'a str,
    /// Selected checksum construction and version.
    pub law: Law,
}

fn refusal(field: &'static str, code: &'static str, node: Option<TermId>) -> Reason {
    Reason::Description { field, code, node }
}

fn check_source<'a>(
    source: &'a RdfDataset,
    schema: Schema,
    description: Description<'_>,
    vocab: Vocabulary<'_>,
) -> Result<&'a str, Reason> {
    if source.named_graphs().next().is_some() {
        return Err(refusal("source", "named-graphs-not-portable", None));
    }
    let TermRef::Iri(root) = source.resolve(schema.root) else {
        return Err(refusal("plan", "expected-absolute-iri", Some(schema.root)));
    };
    if purrdf_iri::absolute_verdict(root) != Some(true) {
        return Err(refusal("plan", "expected-absolute-iri", Some(schema.root)));
    }
    for (field, iri) in [
        ("identity", description.identity),
        ("class", description.class),
        ("digest predicate", description.digest_predicate),
    ] {
        if purrdf_iri::absolute_verdict(iri) != Some(true) {
            return Err(refusal(field, "expected-absolute-iri", None));
        }
        if iri == root || vocab.terms.contains(&iri) || [TYPE, FIRST, REST, NIL].contains(&iri) {
            return Err(refusal(field, "colliding-iri", source.term_id_by_iri(iri)));
        }
    }
    if description.identity == description.class
        || description.identity == description.digest_predicate
        || description.class == description.digest_predicate
    {
        return Err(refusal("vocabulary", "colliding-iri", None));
    }
    Ok(root)
}

fn stamp(
    source: &RdfDataset,
    schema: Schema,
    mode: Mode,
    surface: usize,
    root: &str,
    law: Law,
) -> Digest32 {
    let mode = match mode {
        Mode::Text => "text",
        Mode::Path => "path",
    };
    let mut digest = match law {
        Law::Blake3V1 => blake3::Hasher::new(),
    };
    digest.update(DOMAIN.as_bytes());
    for value in [
        law.version(),
        DESCRIPTION_VERSION,
        LANGUAGE_VERSION,
        root,
        mode,
        SURFACES[surface],
    ] {
        frame_le_into(&mut digest, value.as_bytes());
    }
    digest.update(&(schema.count as u64).to_le_bytes());
    for source in sources(source, schema) {
        frame_le_into(&mut digest, source.as_bytes());
    }
    Digest32::new(*digest.finalize().as_bytes())
}

fn required(
    data: &RdfDataset,
    node: TermId,
    iri: &str,
    field: &'static str,
) -> Result<TermId, Reason> {
    match sole(data, node, data.term_id_by_iri(iri)) {
        SoleObject::One(value) => Ok(value),
        SoleObject::None | SoleObject::Many => {
            Err(refusal(field, "required-single-value", Some(node)))
        }
    }
}

fn verify(
    data: &RdfDataset,
    schema: Schema,
    mode: Mode,
    surface: usize,
    description: Description<'_>,
    vocab: Vocabulary<'_>,
) -> Result<(), Reason> {
    let root = check_source(data, schema, description, vocab)?;
    let node = data
        .term_id_by_iri(description.identity)
        .ok_or_else(|| refusal("identity", "description-not-found", None))?;
    let version = vocab.terms[2];
    let plan = vocab.terms[9];
    let digest = description.digest_predicate;
    check_fields(
        data,
        node,
        0,
        &[TYPE, version, plan, digest].map(|iri| data.term_id_by_iri(iri)),
    )
    .map_err(|_| refusal("identity", "unknown-or-named-field", Some(node)))?;
    if data.reifier_quads_of(node).next().is_some()
        || data.annotations_of_with_graph(node).next().is_some()
    {
        return Err(refusal(
            "identity",
            "unexpected-statement-layer-field",
            Some(node),
        ));
    }
    let class = required(data, node, TYPE, "class")?;
    if Some(class) != data.term_id_by_iri(description.class) {
        return Err(refusal("class", "wrong-description-type", Some(class)));
    }
    let linked = required(data, node, plan, "plan")?;
    if linked != schema.root {
        return Err(refusal("plan", "wrong-source-plan", Some(linked)));
    }
    let version = required(data, node, version, "version")?;
    if text(data, version, 2).ok() != Some(description.law.version()) {
        return Err(refusal("version", "incompatible-version", Some(version)));
    }
    let digest = required(data, node, digest, "digest")?;
    let declared = text(data, digest, 2)
        .ok()
        .and_then(Digest32::from_hex)
        .ok_or_else(|| refusal("digest", "expected-canonical-digest", Some(digest)))?;
    let expected = stamp(data, schema, mode, surface, root, description.law);
    if declared != expected {
        return Err(Reason::StaleDescription {
            node: digest,
            declared,
            expected,
        });
    }
    Ok(())
}

impl<'v, S: Admission> PreparedPlan<'v, S> {
    /// Check a compiled description in the same snapshot before compiling its
    /// authored source. The description contains no executable state to trust.
    ///
    /// # Errors
    /// Preserves original schema, description, version and compiler refusals.
    #[cfg_attr(
        target_pointer_width = "64",
        expect(
            clippy::result_large_err,
            reason = "The refusal keeps its source owner inline"
        )
    )]
    pub fn compile_described(
        source: DatasetHandle,
        plan: TermId,
        vocab: &Vocabulary<'v>,
        description: Description<'_>,
        storage: S,
    ) -> Result<Self, Error> {
        Self::compile_checked(
            source,
            plan,
            vocab,
            storage,
            |data, schema, mode, surface| verify(data, schema, mode, surface, description, *vocab),
        )
    }

    /// Export a complete graph containing the original source and the
    /// versioned description. The original plan remains reusable.
    ///
    /// # Errors
    /// Refuses incompatible sources, occupied identities and output failures.
    #[cfg_attr(
        target_pointer_width = "64",
        expect(
            clippy::result_large_err,
            reason = "The refusal keeps its source and allocation owner"
        )
    )]
    pub fn describe<O: Admission + Send + Sync + 'static>(
        &self,
        description: Description<'_>,
        storage: O,
    ) -> Result<DatasetHandle, ExportError<O>> {
        let mut account = Account(storage);
        let result = self.describe_in(description, &mut Memory::new(&mut account.0));
        let (reason, account) = match result {
            Ok(dataset) => match DatasetHandle::try_from_admitted(dataset, account) {
                Ok(handle) => return Ok(handle),
                Err(_) => (Reason::Storage(StorageError::AllocationFailed), None),
            },
            Err(reason) => (reason, Some(account)),
        };
        Err(ExportError {
            error: Error {
                source: self.source.clone(),
                plan: self.schema.root,
                input: None,
                surface: Some(self.surface),
                reason,
            },
            _account: account,
        })
    }

    fn describe_in<O: Admission>(
        &self,
        description: Description<'_>,
        memory: &mut Memory<'_, O>,
    ) -> Result<RdfDataset, Reason> {
        let root = check_source(&self.source, self.schema, description, self.vocab)?;
        if let Some(existing) = self.source.term_id_by_iri(description.identity) {
            return Err(refusal(
                "identity",
                "occupied-description-identity",
                Some(existing),
            ));
        }
        let checksum = stamp(
            &self.source,
            self.schema,
            self.compiled.mode,
            self.surface,
            root,
            description.law,
        );
        let mut hex = [0; 2 * Digest32::LEN];
        let digits = encode_to_slice(checksum.as_bytes(), &mut hex)
            .expect("fixed buffer contains one canonical checksum");
        let [ty, version, plan, digest] = [
            TYPE,
            self.vocab.terms[2],
            self.vocab.terms[9],
            description.digest_predicate,
        ];
        let mut builder = RdfDatasetBuilder::new();
        let map = builder
            .push_dataset_with_mapping_with_memory(&self.source, memory)
            .map_err(Reason::Storage)?;
        let result = (|| {
            let node = builder.intern_iri_with_memory(description.identity, memory)?;
            let class = builder.intern_iri_with_memory(description.class, memory)?;
            let version_value = builder.intern_literal_parts_with_memory(
                description.law.version(),
                None,
                None,
                None,
                memory,
            )?;
            let digest_value =
                builder.intern_literal_parts_with_memory(digits, None, None, None, memory)?;
            let source = map[self.schema.root.index()].expect("validated plan root imported");
            for (predicate, object) in [
                (ty, class),
                (version, version_value),
                (plan, source),
                (digest, digest_value),
            ] {
                let predicate = builder.intern_iri_with_memory(predicate, memory)?;
                builder.push_quad_with_memory(node, predicate, object, None, memory)?;
            }
            Ok(())
        })();
        memory
            .release_vec_after(map, result)
            .map_err(Reason::Storage)?;
        let data = builder.freeze_with_memory(memory).map_err(Reason::Build)?;
        memory
            .add_bytes(DatasetHandle::allocation_layout::<Account<O>>().size())
            .map_err(Reason::Storage)?;
        Ok(data)
    }
}
