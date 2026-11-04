// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use crate::{
    Ceiling, Profile, SourceDocument, YamlError, admit_count, analyze, profile::validate_iri,
    project,
};
use purrdf_core::{
    ContentDigest, RdfDataset, TermId, TermRef,
    cover::{VerbatimSpan, reconstruct_with_memory},
    datatype::XSD_INTEGER,
    ir::QuadIds,
};
use purrdf_lex::allocation::{Memory, OrderedTable, StorageError};

fn rows(dataset: &RdfDataset) -> impl Iterator<Item = QuadIds> + '_ {
    dataset
        .quads()
        .chain(dataset.reifier_quads())
        .chain(dataset.annotation_quads())
}

/// Reconstruct one expected root/profile through the shared cover law, reparse
/// with Lex and compare the complete selected metadata graph with a reprojection.
/// Both owned subjects and all root-fragment namespace subjects are selected.
/// Other independent carriers may coexist. Phase ceilings compose additively:
/// caller RDF, retained decode scratch/source, Lex/model and reprojection have
/// distinct owners and do not pretend to share one physical account.
///
/// # Errors
/// Refuses incomplete, conflicting, unknown, wrongly typed or forged metadata,
/// original cover/syntax errors and explicit phase/storage refusals.
pub fn decode_stream(
    dataset: &RdfDataset,
    stream: &str,
    profile: &Profile,
) -> Result<String, YamlError> {
    validate_iri(stream)?;
    let bounds = profile.bounds();
    let mut ceiling = Ceiling(bounds.max_decode_bytes);
    let mut memory = Memory::new(&mut ceiling);
    let root = dataset
        .term_id_by_iri(stream)
        .ok_or(YamlError::Metadata("expected stream root"))?;
    let prefix = memory.format(&format_args!("{stream}#"))?;
    let ownership = dataset.term_id_by_iri(profile.vocabulary().term("stream")?);
    let mut selected = OrderedTable::default();
    admit_count(1, bounds.max_statements, "selected subjects")?;
    selected.insert(root, (), &mut memory)?;
    for row in rows(dataset) {
        let owned = ownership == Some(row.p) && row.o == root;
        let namespaced =
            matches!(dataset.resolve(row.s), TermRef::Iri(iri) if iri.starts_with(&prefix));
        if (owned || namespaced) && selected.get(&row.s).is_none() {
            admit_count(
                selected
                    .len()
                    .checked_add(1)
                    .ok_or(StorageError::SizeOverflow)?,
                bounds.max_statements,
                "selected subjects",
            )?;
            selected.insert(row.s, (), &mut memory)?;
        }
    }
    for graph in dataset.named_graphs() {
        if selected.get(&graph).is_some()
            || matches!(dataset.resolve(graph), TermRef::Iri(iri) if iri == stream || iri.starts_with(&prefix))
        {
            return Err(YamlError::Metadata(
                "carrier metadata cannot declare named graphs",
            ));
        }
    }
    let mut claims = Vec::new();
    let mut payload = 0_usize;
    for row in rows(dataset).filter(|row| selected.get(&row.s).is_some()) {
        if row.g.is_some() {
            return Err(YamlError::Metadata(
                "selected metadata must be in the default graph",
            ));
        }
        admit_count(
            claims
                .len()
                .checked_add(1)
                .ok_or(StorageError::SizeOverflow)?,
            bounds.max_statements,
            "selected statements",
        )?;
        for id in [row.s, row.p, row.o] {
            payload = payload
                .checked_add(project::term_bytes(dataset, id)?)
                .ok_or(StorageError::SizeOverflow)?;
            admit_count(payload, bounds.max_payload_bytes, "metadata payload bytes")?;
        }
        memory.push(&mut claims, row)?;
    }
    // One admitted array is also the subject/predicate index. Bootstrap lookup
    // remains logarithmic instead of rescanning the graph for each cover span.
    claims.sort_unstable_by_key(|row| (row.s, row.p, row.o));
    let vocabulary = profile.vocabulary();
    let root_type = required(dataset, &claims, root, purrdf_core::vocab::rdf::TYPE)?;
    if !matches!(root_type, TermRef::Iri(iri) if iri == vocabulary.term("Stream")?) {
        return Err(YamlError::Metadata("expected Stream type"));
    }
    let source = required(dataset, &claims, root, vocabulary.term("source")?)?;
    let TermRef::Iri(source) = source else {
        return Err(YamlError::Metadata("absolute source IRI"));
    };
    validate_iri(source)?;
    let stated_profile = literal(
        dataset,
        &claims,
        root,
        vocabulary.term("profile")?,
        vocabulary.term("sha256Digest")?,
    )?;
    let expected_profile = memory.format(&format_args!("sha256:{}", profile.identity()))?;
    if stated_profile != expected_profile {
        return Err(YamlError::Metadata("selected profile identity"));
    }
    memory.release_string(expected_profile)?;
    let byte_length = number(dataset, &claims, root, vocabulary.term("byteLength")?)?;
    if byte_length > bounds.syntax.syntax.max_input_bytes as u64 {
        return Err(YamlError::Limit {
            resource: "source bytes",
            limit: bounds.syntax.syntax.max_input_bytes,
        });
    }
    let raw_digest = literal(
        dataset,
        &claims,
        root,
        vocabulary.term("sourceDigest")?,
        vocabulary.term("sha256Digest")?,
    )?;
    let digest = ContentDigest::from_hex(
        raw_digest
            .strip_prefix("sha256:")
            .ok_or(YamlError::Metadata("source digest algorithm"))?,
    )
    .ok_or(YamlError::Metadata("source digest"))?;
    let canonical = memory.format(&format_args!("sha256:{digest}"))?;
    if raw_digest != canonical {
        return Err(YamlError::Metadata("canonical source digest"));
    }
    memory.release_string(canonical)?;
    let verbatim = dataset.term_id_by_iri(vocabulary.term("verbatim")?);
    let mut covered = OrderedTable::default();
    let mut spans = Vec::new();
    for row in claims.iter().filter(|row| Some(row.p) == verbatim) {
        if covered.get(&row.s).is_some() {
            continue;
        }
        covered.insert(row.s, (), &mut memory)?;
        if !matches!(required(dataset, &claims, row.s, purrdf_core::vocab::rdf::TYPE)?, TermRef::Iri(iri) if iri == vocabulary.term("CoverSpan")?)
        {
            return Err(YamlError::Metadata("cover span type"));
        }
        let text = literal(
            dataset,
            &claims,
            row.s,
            vocabulary.term("verbatim")?,
            vocabulary.term("verbatimText")?,
        )?;
        admit_count(
            spans
                .len()
                .checked_add(1)
                .ok_or(StorageError::SizeOverflow)?,
            bounds.max_cover_spans,
            "cover spans",
        )?;
        memory.push(
            &mut spans,
            VerbatimSpan {
                byte_start: number(dataset, &claims, row.s, vocabulary.term("byteStart")?)?,
                byte_end: number(dataset, &claims, row.s, vocabulary.term("byteEnd")?)?,
                text,
                continues: None,
            },
        )?;
    }
    let text = reconstruct_with_memory(byte_length, &digest, &spans, &mut memory)?;
    memory.release_vec(spans)?;
    covered.release(&mut memory)?;
    let model = analyze(
        SourceDocument {
            id: source,
            bytes: text.as_bytes(),
        },
        profile,
    )?;
    if model.id() != stream {
        return Err(YamlError::Metadata("recomputed stream identity"));
    }
    let expected = project(&model, profile)?;
    let mut actual = OrderedTable::default();
    for row in &claims {
        let s = translate(dataset, &expected, row.s)?;
        let p = translate(dataset, &expected, row.p)?;
        let o = translate(dataset, &expected, row.o)?;
        actual.insert((s, p, o), (), &mut memory)?;
    }
    let mut wanted_count = 0;
    for row in expected.quads() {
        wanted_count += 1;
        if actual.get(&(row.s, row.p, row.o)).is_none() {
            return Err(YamlError::Metadata("missing reparsed metadata"));
        }
    }
    if actual.len() != wanted_count {
        return Err(YamlError::Metadata("extra selected metadata"));
    }
    // Source output remains alive; local metadata/index storage dies before its
    // account ends. The independent caller RDF/model/projection owners are not
    // refunded into this decode account.
    Ok(text)
}

fn required<'a>(
    dataset: &'a RdfDataset,
    claims: &[QuadIds],
    subject: TermId,
    predicate: &str,
) -> Result<TermRef<'a>, YamlError> {
    let predicate = dataset
        .term_id_by_iri(predicate)
        .ok_or(YamlError::Metadata("missing required predicate"))?;
    let begin = claims.partition_point(|row| (row.s, row.p) < (subject, predicate));
    let mut values = claims[begin..]
        .iter()
        .take_while(|row| row.s == subject && row.p == predicate);
    let object = values
        .next()
        .ok_or(YamlError::Metadata("missing required field"))?
        .o;
    if values.any(|row| row.o != object) {
        return Err(YamlError::Metadata("conflicting required field"));
    }
    Ok(dataset.resolve(object))
}
fn literal<'a>(
    dataset: &'a RdfDataset,
    claims: &[QuadIds],
    subject: TermId,
    predicate: &str,
    datatype: &str,
) -> Result<&'a str, YamlError> {
    match required(dataset, claims, subject, predicate)? {
        TermRef::Literal {
            lexical,
            datatype: stated,
            language: None,
            direction: None,
        } if matches!(dataset.resolve(stated), TermRef::Iri(iri) if iri == datatype) => Ok(lexical),
        _ => Err(YamlError::Metadata("required literal datatype")),
    }
}
fn number(
    dataset: &RdfDataset,
    claims: &[QuadIds],
    subject: TermId,
    predicate: &str,
) -> Result<u64, YamlError> {
    let raw = literal(dataset, claims, subject, predicate, XSD_INTEGER)?;
    if raw.is_empty()
        || raw.len() > 20
        || raw.len() > 1 && raw.starts_with('0')
        || !raw.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(YamlError::Metadata("canonical unsigned integer"));
    }
    raw.parse()
        .map_err(|_| YamlError::Metadata("unsigned integer overflow"))
}
fn translate(dataset: &RdfDataset, expected: &RdfDataset, id: TermId) -> Result<TermId, YamlError> {
    let found = match dataset.resolve(id) {
        TermRef::Iri(iri) => expected.term_id_by_iri(iri),
        TermRef::Literal {
            lexical,
            datatype,
            language: None,
            direction: None,
        } => match dataset.resolve(datatype) {
            TermRef::Iri(datatype) => expected.term_id_by_literal(lexical, datatype, None, None),
            _ => None,
        },
        _ => None,
    };
    found.ok_or(YamlError::Metadata("unexpected metadata term"))
}
