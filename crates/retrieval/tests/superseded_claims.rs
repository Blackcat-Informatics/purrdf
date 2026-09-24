// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The design record and the crate's own prose do not re-state a claim the code
//! has already reversed.
//!
//! `docs/SUPERSEDED-CLAIMS.md` exists because a rewritten comment leaves no trace
//! that the old reading was ever the rule. That argument has a hole this file
//! closes: the ledger records the reversal, and nothing stops the reversed
//! sentence from reappearing where it was rewritten out of. A sentence restored
//! by a bad merge, by a revert, or by an author working from a cached reading
//! would be a claim the ledger says is false, printed as the design.
//!
//! One claim is rarely stated once. "The engine has no random access" stood in
//! the ladder, in `fuse.rs`, in `ranked_stream.rs` and in the producer contract,
//! in four different wordings; a guard over the ladder alone would have caught
//! one of the four. So the corpus is every document that carried the claim, and
//! a fragment is held **absent from all of them**.
//!
//! Each claim is then checked twice, and the pair is the point:
//!
//! * the superseded wording is absent from the whole corpus; and
//! * the wording that replaced it is present somewhere in it.
//!
//! The second assertion is what makes the first mean anything. Absence alone is
//! satisfied by deleting the paragraph — or the section, or the file's content —
//! and a guard that passed over an empty document would be checking that nobody
//! writes rather than that the record is right.
//!
//! The comparison runs over a **whitespace-flattened** copy of each document, so
//! a reflowed paragraph is still the same sentence. A fragment that only matched
//! at one hard-wrap position would guard against re-wrapping rather than against
//! re-asserting.
//!
//! This follows the `include_str!` precedent in `planner_golden.rs`, where the
//! planner's own source is read as text to hold it to a property no value it
//! returns could express.

/// The ledger, so a guarded claim with no entry behind it is a failure too.
const LEDGER: &str = include_str!("../../../docs/SUPERSEDED-CLAIMS.md");

/// Every document that carried one of the reversed claims, by name.
///
/// The design record first, then the crate prose that re-stated it. A document
/// added here costs nothing; a document left out is a place a superseded claim
/// can go on standing.
const CORPUS: &[(&str, &str)] = &[
    (
        "docs/design/purrdf-retrieval-ladder.md",
        include_str!("../../../docs/design/purrdf-retrieval-ladder.md"),
    ),
    (
        "PRODUCER-CONTRACT.md",
        include_str!("../PRODUCER-CONTRACT.md"),
    ),
    ("src/lib.rs", include_str!("../src/lib.rs")),
    ("src/fuse.rs", include_str!("../src/fuse.rs")),
    (
        "src/fusion_stream.rs",
        include_str!("../src/fusion_stream.rs"),
    ),
    (
        "src/ranked_stream.rs",
        include_str!("../src/ranked_stream.rs"),
    ),
    ("src/compile.rs", include_str!("../src/compile.rs")),
    ("src/execute.rs", include_str!("../src/execute.rs")),
    ("src/search.rs", include_str!("../src/search.rs")),
];

/// One reversal: the ledger entry that records it, the wording that is now
/// false, and the wording that replaced it.
struct Reversal {
    /// The ledger heading, verbatim, minus its `### `.
    entry: &'static str,
    /// Wordings of the superseded claim. None may appear anywhere in the corpus.
    superseded: &'static [&'static str],
    /// Wordings that replaced them. Each must appear somewhere in the corpus.
    replacement: &'static [&'static str],
}

/// Collapse every run of whitespace to one space, so a fragment matches whatever
/// column the paragraph was wrapped at.
///
/// A line's leading comment marker goes with the whitespace, for the same reason:
/// a sentence wrapped across three `///` lines carries two markers *inside* it,
/// and a fragment written the way a reader reads the sentence would never match
/// it. Stripping them is what lets one fragment be checked against a Markdown
/// document and a Rust doc comment alike.
fn flatten(document: &str) -> String {
    let mut words: Vec<&str> = Vec::new();
    for line in document.lines() {
        let trimmed = line.trim_start();
        let body = trimmed
            .strip_prefix("//!")
            .or_else(|| trimmed.strip_prefix("///"))
            .or_else(|| trimmed.strip_prefix("//"))
            .unwrap_or(trimmed);
        words.extend(body.split_whitespace());
    }
    words.join(" ")
}

/// The recorded reversals, each keyed by its ledger entry.
fn reversals() -> Vec<Reversal> {
    vec![
        Reversal {
            entry: "The executor's rung is not a cursor, and `execute` is the only stage that \
                    runs a query",
            superseded: &[
                "That rung is not a cursor and stopping there buys no laziness",
                "Only `execute` runs a query",
                "is the only stage that runs a query",
            ],
            replacement: &[
                "What that rung is not is a closed value.",
                "`execute` is the only stage that runs the *ranked read*",
                "the query itself runs when `fuse` asks",
                "the only stage that runs a **ranked read**",
            ],
        },
        Reversal {
            entry: "The engine has no random access, so only a domain declaration can settle \
                    what a stream will not name",
            superseded: &[
                "The engine has no random access",
                "the engine has no random access",
                "this engine has no random access",
                "jointly achievable only if the producers say which candidates they can name",
                "jointly unachievable unless the producers say which candidates they can name",
                "jointly reachable only if fusion is told which candidates a stream can name",
            ],
            replacement: &[
                "The engine therefore has random access where a producer sold it, and only there.",
                "jointly achievable by **either** route",
                "**The observational half** is a question the consumer asks.",
                "jointly unachievable unless the producers say something",
                "jointly reachable only if fusion is told, or can ask, which candidates a stream \
                 can name",
            ],
        },
        Reversal {
            entry: "An undeclared-domain run drains",
            superseded: &[
                "What the licence buys is the reading: without it, strata whose candidate sets \
                 do not overlap are read to their ends",
                "Without one, a candidate cannot certify until every open stream has named it",
                "Where they do not overlap, and nothing has been declared, they never arrive",
                "The way out is the producers' own declaration.",
                "Without a declaration, \"could this stream still name the candidate\" is true \
                 of every open stream",
            ],
            replacement: &[
                "With the observational half alone — domains undeclared, lookups answered — the \
                 read is not a drain",
                "There are two ways out and a producer may sell either.",
                "Told and asked nothing, a candidate cannot certify until every open stream has \
                 named it",
                "With nothing declared and nothing askable",
            ],
        },
        Reversal {
            entry: "The request's bound narrows every stratum's depth or none",
            superseded: &[
                "where every stratum's declared blocks are pairwise disjoint the planner derives \
                 each depth from that bound",
                "Any overlap, or any unrestricted stratum, and the declared-or-measured bound \
                 stands",
                "Over strata whose declared blocks are pairwise disjoint the recorded depth is \
                 the request's own bound",
                "every surviving stratum declares a block set, no two of those sets meet",
            ],
            replacement: &[
                "the planner derives a depth from that bound **per stratum**",
                "A stratum that cannot narrow takes no other stratum's narrowing with it",
                "that exclusion is request-wide because the declaration is",
                "The reading is **per stratum**",
                "and it is decided per stratum",
            ],
        },
        Reversal {
            entry: "The probe row is counted nowhere",
            superseded: &[
                "read, never reported, absent from every identity and every resolution number",
                "no plan field, identity or resolution number moves by one because of it",
                "No plan field, no identity and no recorded resolution moves by one",
                "never ranked, and never counted anywhere",
                "never emitted onto the stream, never ranked, never counted, present in no plan \
                 field, no identity and no resolution number",
                "its only effect is the ending",
            ],
            replacement: &[
                "counted in exactly one number, the rows a read materialized",
                "how many rows each stratum's read actually returned",
                "counted in exactly one place",
                "counted in exactly one number — the rows a read materialized",
            ],
        },
        Reversal {
            entry: "A read that is not read to its end cannot carry a verifiable receipt",
            superseded: &[
                "Discarding rather than resuming is what keeps the answer verifiable",
                "exists only on a *completed* governed run",
                "an attestation exists only on a completed run",
                "the one-call entry point reads at most **twice**",
            ],
            replacement: &[
                "The answer stays verifiable because the receipt is taken when the read stops \
                 rather than before its first row",
                "# Each stratum is read as far as the fusion asks, once",
                "# A stratum may be read on demand, and its receipt is taken when the reading \
                 stops",
            ],
        },
        Reversal {
            entry: "The evaluator exposes no cursor, and incremental enumeration is separate, \
                    larger work",
            superseded: &[
                "the evaluator exposes no cursor, stream or iterator surface at all",
                "Making enumeration incremental is separate, larger work",
            ],
            replacement: &[
                "The same unit can be read **on demand**",
                "`NativeSparqlEngine::open_call_cursor`",
            ],
        },
        Reversal {
            entry: "A stream's ending is fixed before its first row, or a caller that stopped \
                    early would be told something different",
            superseded: &[
                "a stream whose ending were derived at the end would say something different to \
                 a caller that stopped early",
            ],
            replacement: &["it says nothing to such a caller at all"],
        },
    ]
}

/// Every reversal in the ledger is out of the prose, and its replacement is in.
#[test]
fn no_document_re_states_a_claim_the_ledger_supersedes() {
    let corpus: Vec<(&str, String)> = CORPUS
        .iter()
        .map(|(name, body)| (*name, flatten(body)))
        .collect();
    let ledger = flatten(LEDGER);

    for reversal in reversals() {
        let entry = flatten(reversal.entry);
        assert!(
            ledger.contains(&entry),
            "no ledger entry named `{entry}`; a claim guarded here without an entry \
             is a reversal that was fixed in place and never recorded, which is the \
             one thing the ledger exists to stop"
        );
        // A guarded claim's entry names the test as well as the rule. That is a
        // stricter demand than the ledger's format makes of every entry — an
        // entry may honestly locate its new rule in the code — and it is made
        // here, of exactly these entries, because a guarded claim has a
        // measurement behind it and an entry that did not name one would leave
        // this file as the only thing holding the wording while nothing held the
        // behaviour.
        let body = LEDGER
            .split("\n### ")
            .find(|section| flatten(section.lines().next().unwrap_or_default()) == entry)
            .expect("the entry was just found in the ledger");
        assert!(
            body.contains("Pinned by") || body.contains("pinned by"),
            "ledger entry `{entry}` names no test; the wording is guarded here and \
             the rule behind it would be guarded by nothing"
        );
        for fragment in reversal.superseded {
            let fragment = flatten(fragment);
            for (name, body) in &corpus {
                assert!(
                    !body.contains(&fragment),
                    "{name} states `{fragment}`, which the ledger entry `{entry}` \
                     records as superseded. It is false about the code as it \
                     stands: correct it in place, or — if the code moved back — \
                     retire the ledger entry rather than leaving the record and \
                     the prose disagreeing"
                );
            }
        }
        for fragment in reversal.replacement {
            let fragment = flatten(fragment);
            assert!(
                corpus.iter().any(|(_, body)| body.contains(&fragment)),
                "no document states `{fragment}`, part of the wording that \
                 replaced the claim `{entry}` records. Deleting the correction \
                 satisfies the absence check above while leaving the prose silent \
                 on a question it used to answer wrongly, which is why both halves \
                 are asserted"
            );
        }
    }
}

/// The ladder names the API the crate actually has, and counts what it actually
/// has.
///
/// Not ledger material — none of these was ever an invariant, they are a
/// signature and two tallies that drifted — but they regress the same way a
/// claim does, and for the same reason: nothing executes a design record.
#[test]
fn the_ladder_names_the_api_and_the_tallies_the_crate_actually_has() {
    let ladder = flatten(CORPUS[0].1);
    for stale in [
        "query_with_options_view",
        "Vec<(u64, Term)>",
        "execute(query, dataset)",
        "There are five read endings",
        "Fifteen obligations",
    ] {
        assert!(
            !ladder.contains(stale),
            "the ladder says `{stale}`, which the crate has not been true of since \
             before this guard was written"
        );
    }
    for current in [
        "query_prepared_governed_view",
        "Vec<(u64, Term, RowBlock)>",
        "execute(compiled, registry, dataset)",
        "There are seven read endings",
        "Sixteen obligations",
    ] {
        assert!(
            ladder.contains(current),
            "the ladder no longer says `{current}`; the correction was deleted \
             rather than kept current"
        );
    }
    // The tally has a source, and it is the document being counted. Asserting the
    // word alone would pass over a contract that grew a seventeenth obligation.
    let contract = CORPUS[1].1;
    let obligations = (1..=16)
        .filter(|index| contract.contains(&format!("\n## A{index} — ")))
        .count();
    assert_eq!(
        obligations, 16,
        "the producer contract states {obligations} obligations and the ladder says \
         sixteen"
    );
    assert!(
        !contract.contains("\n## A17 — "),
        "the producer contract grew an obligation the ladder's tally does not count"
    );
}

/// Every entry carries the four paragraphs the ledger's own header specifies.
///
/// The format is the entry's whole value: what the claim was, why a competent
/// reader believed it, what moved, and where the new rule lives. An entry
/// missing one of those reads as a complete record while answering a question it
/// never answered, which is the failure mode a ledger of reversals can least
/// afford.
#[test]
fn every_ledger_entry_states_the_claim_the_belief_the_change_and_the_rule() {
    let entries: Vec<&str> = LEDGER.split("\n### ").skip(1).collect();
    assert!(
        entries.len() >= 13,
        "the ledger holds {} entries; this branch added eight to the five already \
         there, so a smaller count means one was dropped rather than superseded",
        entries.len()
    );
    for entry in entries {
        let heading = entry.lines().next().unwrap_or_default();
        for marker in [
            "**Was stated in**",
            "**Why it was believed.**",
            "**What changed.**",
            "**The rule now.**",
        ] {
            assert!(
                entry.contains(marker),
                "ledger entry `{heading}` has no `{marker}` paragraph"
            );
        }
    }
}
