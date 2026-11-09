// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! One positional term decoder for sequential and indexed readers.

use super::{FoldError, RowError, TermIdBase, as_i128};
use crate::model::{Term, TermKind, language_tag_refusal};
use crate::wire::map_get;
use purrdf_lex::cbor::Value;

pub(super) enum TermDiagnostic {
    Fold(FoldError),
    Row(RowError),
}

fn reference(
    value: Option<&Value>,
    base: TermIdBase,
    diagnose: &mut impl FnMut(TermDiagnostic),
) -> Option<i128> {
    match base.integer(value?) {
        Ok(value) => value,
        Err(error) => {
            diagnose(TermDiagnostic::Row(error));
            None
        }
    }
}

/// Non-map rows consume no positional id. Damaged fields retain the baseline
/// reader's survivor semantics and report their original typed diagnostic.
pub(super) fn decode(
    raw: &Value,
    term_id: usize,
    base: TermIdBase,
    mut diagnose: impl FnMut(TermDiagnostic),
) -> Option<Term> {
    let Value::Map(entries) = raw else {
        return None;
    };
    let kind = TermKind::from_wire(map_get(entries, "k").and_then(as_i128));
    let value = map_get(entries, "v")
        .and_then(Value::as_text)
        .map(str::to_string);
    // This row's id, read before any decode that can raise a diagnostic:
    // diagnostics leave the term table unchanged,
    // so the id a refusal quotes is the id this row goes on to take.
    let tid = term_id as i128;
    // THE language-tag decode point. `"l"` is the only field of the wire
    // format that becomes a `LANGTAG` token downstream, and this is the
    // only place it is read: `h_snapshot` re-enters through this very
    // function, `reader_union` and `compact` copy already-decoded
    // `Term`s, and the event bridge (`event_stream::stream_events`)
    // drives `read_to_sink_with_options`, i.e. this loop. So a tag that
    // does not pass here reaches neither the fold view's N-Quads token
    // (`purrdf_rdf::gts_view`'s `render_literal`) nor any `GtsEventSink`.
    //
    // Ask the grammar, on the profile every RDF codec and the IR kernel
    // already name (`model::LANGUAGE_TAG_PROFILE`). A refusal is NOT a
    // quiet coercion: it is a `DamagedFrame` diagnostic — the registry's
    // code for a malformed payload, whose stated reader behaviour is
    // "isolate the damaged item ..., surface a diagnostic, and fold
    // survivors" (GTS-CONFORMANCE.md §6) — that quotes the term id, the
    // production that rejected the tag, and the tag verbatim, so nothing
    // about the refusal has to be guessed. The damaged item is the tag;
    // the survivor is the literal's lexical form, which folds on under
    // §7.1's `xsd:string` default. This is the same shape the sibling
    // `"dir"` field already has directly below, where a value outside
    // `"ltr" / "rtl"` is likewise not carried through.
    //
    // The Baseline Reader cannot answer a malformed tag any other way:
    // `read` returns a `Graph`, not a `Result` (permissive-read "never
    // panic ..., return graph state plus diagnostics", §7), and dropping
    // the whole row is impossible because term ids are positional —
    // `term_id` is the next append-order id, so skipping a row would renumber
    // every later term in the segment.
    // `lang_refused` distinguishes "there was no tag" from "there was a
    // tag and the grammar refused it". The base-direction arm below
    // needs that distinction: the two cases must not fold to the same
    // outcome, because only one of them is a defect in the input.
    let (lang, lang_refused) = match map_get(entries, "l").and_then(Value::as_text) {
        Some(tag) => match language_tag_refusal(tag) {
            Some(code) => {
                diagnose(TermDiagnostic::Fold(FoldError::Language {
                    term: tid,
                    tag: tag.to_string(),
                    refusal: code,
                }));
                (None, true)
            }
            None => (Some(tag.to_string()), false),
        },
        None => (None, false),
    };
    // Same reasoning as the tag arm above, and the same failure shape. A
    // value outside the closed `ltr`/`rtl` space cannot be carried
    // through, and dropping it without saying so is worse than it looks:
    // the term keeps its lexical form and its language, so an
    // `rdf:dirLangString` quietly becomes an `rdf:langString` and the
    // graph asserts a literal the container does not contain — hence the
    // diagnostic. The writer filters direction before emitting it, so a
    // value here can only have come from a foreign container, which is
    // precisely the case a diagnostic is for.
    let direction = match map_get(entries, "dir").and_then(Value::as_text) {
        Some(value) if crate::model::is_literal_direction(value) => Some(value.to_string()),
        Some(value) => {
            diagnose(TermDiagnostic::Fold(FoldError::Direction {
                term: tid,
                direction: value.to_string(),
            }));
            None
        }
        None => None,
    };
    // A refused tag takes the base direction down with it. `direction`
    // is only ever the second half of an `rdf:dirLangString`, so once
    // the language is gone it describes nothing: carrying it on would
    // build a term with a direction and no language, which is outside
    // RDF 1.2's value space. Leaving it standing is not a harmless
    // remnant either — it makes the importer downstream refuse with
    // `gts-direction-without-language`, naming a defect the container
    // never had and pointing away from the tag that actually failed.
    //
    // This is a consequence of a refusal, not a second input error, so
    // it gets its own diagnostic rather than riding along silently on
    // the tag's: dropping carried data without saying so is the
    // documented mirror of over-refusal, and it costs a reader exactly
    // the field it would need to understand the first diagnostic.
    //
    // Only a *refused* tag triggers this. A `"dir"` with no `"l"` at
    // all is a genuine defect in the container, and the importer's
    // `gts-direction-without-language` is then the correct name for it
    // — so that case is left alone to be reported accurately.
    let direction = match (lang_refused, direction) {
        (true, Some(direction)) => {
            diagnose(TermDiagnostic::Fold(FoldError::RefusedLanguageDirection {
                term: tid,
                direction,
            }));
            None
        }
        (_, direction) => direction,
    };
    let dt_raw = reference(map_get(entries, "dt"), base, &mut diagnose);
    let rf_raw = reference(map_get(entries, "rf"), base, &mut diagnose);
    // Sanitise refs: dt MUST name an already-introduced term, and rf
    // normally does too (§7.5). A quoted-triple term may self-bind
    // its reifier (`rf == term_id`) so the term can be used directly
    // as an RDF 1.2 triple term while the following `reifies` frame
    // supplies the SPO binding.
    let sanitize_prior = |r: Option<i128>| match r {
        Some(d) if (0..tid).contains(&d) => Some(d as usize),
        _ => None,
    };
    let dt = sanitize_prior(dt_raw);
    let rf = match rf_raw {
        Some(d) if (0..tid).contains(&d) => Some(d as usize),
        Some(d) if kind == TermKind::Triple && d == tid => Some(d as usize),
        _ => None,
    };
    let dt_out_of_range = matches!(dt_raw, Some(d) if d >= tid);
    let rf_out_of_range =
        matches!(rf_raw, Some(d) if d >= tid && !(kind == TermKind::Triple && d == tid));
    // `tt` — the quoted triple's OWN components (§7.1). Every id must
    // name an already-introduced term, exactly like `dt`; a triple term
    // therefore cannot contain itself, which is what makes `tt` the
    // acyclic-by-construction spelling of a quoted triple.
    let tt_raw = match map_get(entries, "tt") {
        Some(Value::Array(ids)) if ids.len() == 3 => Some(
            [&ids[0], &ids[1], &ids[2]].map(|value| reference(Some(value), base, &mut diagnose)),
        ),
        _ => None,
    };
    let tt = match tt_raw {
        Some([Some(s), Some(p), Some(o)])
            if kind == TermKind::Triple && [s, p, o].iter().all(|d| (0..tid).contains(d)) =>
        {
            Some((s as usize, p as usize, o as usize))
        }
        _ => None,
    };
    let tt_out_of_range = tt.is_none()
        && matches!(tt_raw, Some(ids) if ids.iter().any(|d| matches!(d, Some(d) if *d >= tid)));
    if dt_out_of_range || rf_out_of_range || tt_out_of_range {
        diagnose(TermDiagnostic::Fold(FoldError::ForwardReference {
            term: tid,
            datatype: dt_raw,
            reifier: rf_raw,
            triple: tt_raw,
        }));
    }
    Some(Term {
        kind,
        value,
        datatype: dt,
        lang,
        direction,
        reifier: rf,
        triple: tt,
    })
}
