// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Projection identity over the existing native dictionary, without owned suffixes.

use purrdf_core::hash::{FastMap, reserve_map_with_memory};
use purrdf_core::{DatasetView, RdfDatasetBuilder, TermGuard as _, TermId, TermRef};
use purrdf_lex::allocation::{Admission, Memory, Resident, StorageError};
use purrdf_lex::json_escape::{JsonEscapes, try_escape_body};

use super::term::{validate_blank_label, validate_literal_identity};
use super::{ProjectionError, validate_absolute_iri};

#[derive(Clone, Copy)]
pub(super) struct CapturedTerm {
    pub id: TermId,
    /// Number of triple nodes on the longest path; leaves have height zero.
    pub height: usize,
}

pub(super) type CaptureMemo<Id> = FastMap<Id, Option<CapturedTerm>>;

pub(super) struct NativeCapture<'a, D: DatasetView> {
    view: &'a D,
    maximum: usize,
    pub builder: RdfDatasetBuilder,
    memo: CaptureMemo<D::Id>,
}

impl<'a, D: DatasetView> NativeCapture<'a, D> {
    pub(super) fn new(view: &'a D, maximum: usize) -> Self {
        Self {
            view,
            maximum,
            builder: RdfDatasetBuilder::new(),
            memo: CaptureMemo::default(),
        }
    }

    pub(super) fn term<S: Admission + ?Sized>(
        &mut self,
        id: D::Id,
        memory: &mut Memory<'_, S>,
    ) -> Result<TermId, ProjectionError> {
        capture(
            self.view,
            id,
            self.maximum,
            &mut self.builder,
            &mut self.memo,
            memory,
        )
    }
}

enum CaptureStep<Id, Guard> {
    Enter(Id, usize),
    Literal(Id, Guard, Id),
    Triple(Id, Id, Id, Id),
}

pub(super) fn check_depth(
    height: usize,
    depth: usize,
    maximum: usize,
) -> Result<(), ProjectionError> {
    if height != 0 && (depth > maximum || height - 1 > maximum - depth) {
        return Err(ProjectionError::limit(format!(
            "RDF triple term exceeds the configured depth limit of {maximum}"
        )));
    }
    Ok(())
}

/// Capture only this root's closure. A memo belongs to one source and builder.
/// Original guards survive dependencies; no leaf is read again to recover text.
fn capture<D: DatasetView, S: Admission + ?Sized>(
    view: &D,
    root: D::Id,
    maximum: usize,
    builder: &mut RdfDatasetBuilder,
    memo: &mut CaptureMemo<D::Id>,
    memory: &mut Memory<'_, S>,
) -> Result<TermId, ProjectionError> {
    view.checked_read(|view| {
        let mut work = Vec::new();
        let result = (|| {
            memory.push(&mut work, CaptureStep::Enter(root, 0))?;
            while let Some(step) = work.pop() {
                let (source, captured) = match step {
                    CaptureStep::Enter(id, depth) => {
                        if let Some(entry) = memo.get(&id) {
                            let Some(entry) = entry else {
                                return Err(ProjectionError::term(
                                    "cyclic triple-term component graph",
                                ));
                            };
                            check_depth(entry.height, depth, maximum)?;
                            continue;
                        }
                        let required = memo
                            .len()
                            .checked_add(1)
                            .ok_or(StorageError::SizeOverflow)?;
                        reserve_map_with_memory(memo, required, memory)?;
                        memo.insert(id, None);
                        let guard = view.resolve(id).map_err(ProjectionError::source_read)?;
                        let mapped = match guard.term() {
                            TermRef::Iri(value) => {
                                validate_absolute_iri(value, "term IRI")
                                    .map_err(|error| ProjectionError::term(error.message()))?;
                                builder.intern_iri_with_memory(value, memory)?
                            }
                            TermRef::Blank { label, scope } => {
                                validate_blank_label(label)?;
                                builder.intern_blank_with_memory(label, scope, memory)?
                            }
                            TermRef::Literal { datatype, .. } => {
                                memory
                                    .push(&mut work, CaptureStep::Literal(id, guard, datatype))?;
                                memory.push(&mut work, CaptureStep::Enter(datatype, depth))?;
                                continue;
                            }
                            TermRef::Triple { s, p, o } => {
                                check_depth(1, depth, maximum)?;
                                let next =
                                    depth.checked_add(1).ok_or(StorageError::SizeOverflow)?;
                                memory.push(&mut work, CaptureStep::Triple(id, s, p, o))?;
                                for child in [o, p, s] {
                                    memory.push(&mut work, CaptureStep::Enter(child, next))?;
                                }
                                continue;
                            }
                        };
                        (
                            id,
                            CapturedTerm {
                                id: mapped,
                                height: 0,
                            },
                        )
                    }
                    CaptureStep::Literal(id, guard, datatype) => {
                        let datatype = completed(memo, datatype)?;
                        let TermRef::Iri(datatype) = builder.resolve(datatype.id) else {
                            return Err(ProjectionError::term(
                                "literal datatype position does not resolve to an IRI",
                            ));
                        };
                        let TermRef::Literal {
                            lexical,
                            language,
                            direction,
                            ..
                        } = guard.term()
                        else {
                            unreachable!("literal continuation retains its original guard")
                        };
                        validate_literal_identity(datatype, language, direction)?;
                        // Keep the original literal guard, and admit the one
                        // temporary datatype spelling needed while mutating its
                        // native dictionary. No source leaf is read twice.
                        let datatype_guard = memory.string(datatype)?;
                        let result = builder.intern_literal_parts_with_memory(
                            lexical,
                            Some(&datatype_guard),
                            language,
                            direction,
                            memory,
                        );
                        let refund = memory.release_string(datatype_guard);
                        let mapped = result.and_then(|value| refund.map(|()| value))?;
                        (
                            id,
                            CapturedTerm {
                                id: mapped,
                                height: 0,
                            },
                        )
                    }
                    CaptureStep::Triple(id, s, p, o) => {
                        let (s, p, o) = (
                            completed(memo, s)?,
                            completed(memo, p)?,
                            completed(memo, o)?,
                        );
                        if !matches!(
                            builder.resolve(s.id),
                            TermRef::Iri(_) | TermRef::Blank { .. }
                        ) {
                            return Err(ProjectionError::term(
                                "native triple-term subject must be an IRI or blank node",
                            ));
                        }
                        if !matches!(builder.resolve(p.id), TermRef::Iri(_)) {
                            return Err(ProjectionError::term(
                                "triple-term predicate position does not resolve to an IRI",
                            ));
                        }
                        let height = s
                            .height
                            .max(o.height)
                            .checked_add(1)
                            .ok_or(StorageError::SizeOverflow)?;
                        let mapped = builder.intern_triple_with_memory(s.id, p.id, o.id, memory)?;
                        (id, CapturedTerm { id: mapped, height })
                    }
                };
                memo.insert(source, Some(captured));
            }
            let captured = completed(memo, root)?;
            check_depth(captured.height, 0, maximum)?;
            Ok(captured.id)
        })();
        let refund = memory.release_vec(work).map_err(ProjectionError::from);
        result.and_then(|value| refund.map(|()| value))
    })
    .map_err(ProjectionError::source_read)?
}

fn completed<Id: purrdf_core::dataset_view::ViewTermId>(
    memo: &CaptureMemo<Id>,
    id: Id,
) -> Result<CapturedTerm, ProjectionError> {
    memo.get(&id).copied().flatten().ok_or_else(|| {
        ProjectionError::integrity("native projection continuation has no completed dependency")
    })
}

enum JsonStep {
    Term(TermId),
    Text(&'static str),
}

#[derive(Clone, Copy)]
enum JsonRoot<'a> {
    Term(TermId),
    Row(&'a [(&'a str, Option<TermId>)]),
}

/// One bounded output buffer and one admitted continuation list. The spelling
/// is CANONICAL_TERM_JSON_LAW_ID, including member order and explicit nulls.
struct TermJson {
    text: String,
    maximum: usize,
}

impl TermJson {
    fn append<S: Admission + ?Sized>(
        &mut self,
        text: &str,
        memory: &mut Memory<'_, S>,
    ) -> Result<(), ProjectionError> {
        if self
            .text
            .len()
            .checked_add(text.len())
            .is_none_or(|len| len > self.maximum)
        {
            return Err(ProjectionError::limit(format!(
                "canonical RDF term JSON exceeds the {}-byte artifact limit",
                self.maximum
            )));
        }
        memory.push_str(&mut self.text, text)?;
        Ok(())
    }

    fn quote<S: Admission + ?Sized>(
        &mut self,
        text: &str,
        memory: &mut Memory<'_, S>,
    ) -> Result<(), ProjectionError> {
        self.append("\"", memory)?;
        try_escape_body(text, JsonEscapes::ShortForms, |piece| {
            self.append(piece, memory)
        })?;
        self.append("\"", memory)
    }

    fn term<'a, S: Admission + ?Sized>(
        &mut self,
        resolve: impl Fn(TermId) -> TermRef<'a> + Copy,
        root: TermId,
        memory: &mut Memory<'_, S>,
    ) -> Result<(), ProjectionError> {
        let mut work = Vec::new();
        let result = (|| {
            memory.push(&mut work, JsonStep::Term(root))?;
            while let Some(step) = work.pop() {
                let id = match step {
                    JsonStep::Term(id) => id,
                    JsonStep::Text(text) => {
                        self.append(text, memory)?;
                        continue;
                    }
                };
                match resolve(id) {
                    TermRef::Iri(value) => {
                        self.append("{\"kind\":\"iri\",\"value\":", memory)?;
                        self.quote(value, memory)?;
                        self.append("}", memory)?;
                    }
                    TermRef::Blank { label, scope } => {
                        self.append("{\"kind\":\"blank\",\"label\":", memory)?;
                        self.quote(label, memory)?;
                        self.append(",\"scope\":", memory)?;
                        let scope = memory.format(&scope.ordinal())?;
                        let result = self.append(&scope, memory);
                        let refund = memory.release_string(scope).map_err(ProjectionError::from);
                        result.and(refund)?;
                        self.append("}", memory)?;
                    }
                    TermRef::Literal {
                        lexical,
                        datatype,
                        language,
                        direction,
                    } => {
                        let TermRef::Iri(datatype) = resolve(datatype) else {
                            unreachable!("native literal datatype is an IRI")
                        };
                        self.append("{\"kind\":\"literal\",\"lexical\":", memory)?;
                        self.quote(lexical, memory)?;
                        self.append(",\"datatype\":", memory)?;
                        self.quote(datatype, memory)?;
                        self.append(",\"language\":", memory)?;
                        if let Some(language) = language {
                            self.quote(language, memory)?;
                        } else {
                            self.append("null", memory)?;
                        }
                        self.append(",\"direction\":", memory)?;
                        if let Some(direction) = direction {
                            self.quote(direction.as_str(), memory)?;
                        } else {
                            self.append("null", memory)?;
                        }
                        self.append("}", memory)?;
                    }
                    TermRef::Triple { s, p, o } => {
                        self.append("{\"kind\":\"triple\",\"subject\":", memory)?;
                        for step in [
                            JsonStep::Text("}"),
                            JsonStep::Term(o),
                            JsonStep::Text(",\"object\":"),
                            JsonStep::Term(p),
                            JsonStep::Text(",\"predicate\":"),
                            JsonStep::Term(s),
                        ] {
                            memory.push(&mut work, step)?;
                        }
                    }
                }
            }
            Ok(())
        })();
        let refund = memory.release_vec(work).map_err(ProjectionError::from);
        result.and(refund)
    }
}

pub(super) fn canonical_term_json<'a>(
    resolve: impl Fn(TermId) -> TermRef<'a> + Copy,
    id: TermId,
    maximum: usize,
) -> Result<Vec<u8>, ProjectionError> {
    canonical_json(resolve, JsonRoot::Term(id), maximum)
}

pub(super) fn canonical_row_json<'a>(
    resolve: impl Fn(TermId) -> TermRef<'a> + Copy,
    fields: &[(&str, Option<TermId>)],
    maximum: usize,
) -> Result<Vec<u8>, ProjectionError> {
    canonical_json(resolve, JsonRoot::Row(fields), maximum)
}

fn canonical_json<'a>(
    resolve: impl Fn(TermId) -> TermRef<'a> + Copy,
    root: JsonRoot<'_>,
    maximum: usize,
) -> Result<Vec<u8>, ProjectionError> {
    let mut resident = Resident;
    let mut memory = Memory::new(&mut resident);
    let mut output = TermJson {
        text: String::new(),
        maximum,
    };
    let result = (|| {
        match root {
            JsonRoot::Row(fields) => {
                output.append("{", &mut memory)?;
                for (index, (key, term)) in fields.iter().enumerate() {
                    if index != 0 {
                        output.append(",", &mut memory)?;
                    }
                    output.quote(key, &mut memory)?;
                    output.append(":", &mut memory)?;
                    if let Some(term) = term {
                        output.term(resolve, *term, &mut memory)?;
                    } else {
                        output.append("null", &mut memory)?;
                    }
                }
                output.append("}", &mut memory)?;
            }
            JsonRoot::Term(root) => {
                output.term(resolve, root, &mut memory)?;
            }
        }
        Ok(())
    })();
    if let Err(error) = result {
        let _ = memory.release_string(output.text);
        return Err(error);
    }
    Ok(output.text.into_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ProjectionErrorKind;

    #[test]
    fn native_projection_capture_and_json_are_deep_and_reapply_shared_suffix_depth() {
        purrdf_stack::on_stack(128 * 1024, || {
            let mut source = RdfDatasetBuilder::new();
            let subject = source.intern_iri("https://example.org/s").expect("subject");
            let predicate = source
                .intern_iri("https://example.org/p")
                .expect("predicate");
            let mut object = subject;
            let mut boundary = None;
            let mut beyond = None;
            for height in 1..=2048 {
                object = source
                    .intern_triple(subject, predicate, object)
                    .expect("triple");
                if height == 17 {
                    boundary = Some(object);
                }
                if height == 18 {
                    beyond = Some(object);
                }
            }
            source
                .push_quad(subject, predicate, object, None)
                .expect("quad");
            let source = source.freeze().expect("source");
            let mut resident = Resident;
            let mut memory = Memory::new(&mut resident);
            let mut bounded = NativeCapture::new(source.as_ref(), 16);
            bounded
                .term(boundary.expect("boundary"), &mut memory)
                .expect("depth 16 includes its root");
            let refusal = bounded
                .term(beyond.expect("beyond"), &mut memory)
                .expect_err("cached suffix still exceeds caller depth");
            assert_eq!(refusal.kind(), ProjectionErrorKind::ResourceLimit);
            drop(bounded);
            drop(memory);

            let mut memory = Memory::new(&mut resident);
            let mut deep = NativeCapture::new(source.as_ref(), 2047);
            let root = deep.term(object, &mut memory).expect("deep native capture");
            let retained = deep.builder.freeze().expect("native owner");
            drop(source);
            let json = canonical_term_json(|id| retained.as_ref().resolve(id), root, 1_000_000)
                .expect("deep canonical JSON");
            let spelling = std::str::from_utf8(&json).expect("UTF-8");
            assert_eq!(spelling.matches("\"kind\":\"triple\"").count(), 2048);
            assert_eq!(
                canonical_term_json(|id| retained.as_ref().resolve(id), root, json.len())
                    .expect("exact byte boundary"),
                json
            );
            assert_eq!(
                canonical_term_json(|id| retained.as_ref().resolve(id), root, json.len() - 1)
                    .expect_err("one byte over limit")
                    .kind(),
                ProjectionErrorKind::ResourceLimit
            );
            // All native owners and continuation buffers drop on this small stack.
        })
        .expect("native small stack");
    }
}
