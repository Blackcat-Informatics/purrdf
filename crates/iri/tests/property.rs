// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Property tests.
//!
//! Two families live here. The IRI one asserts that parse is verbatim-faithful,
//! that normalization is idempotent, and that resolving a reference against an
//! absolute base yields an absolute IRI.
//!
//! The RFC 5646 language-tag one asserts the invariants the module documents but
//! that no finite corpus can pin: the parser is **total** (no input panics it,
//! which matters because it slices by byte offset), §2.1.1 canonical case is
//! **idempotent** and **acceptance-preserving**, the three profiles are **totally
//! ordered** by acceptance with the wider ones preserving the narrower one's
//! decomposition, and the owning and borrowing forms **agree section for
//! section**.
//!
//! Inputs come from two generators on purpose. `structured_tag` builds tags out
//! of the §2.1 ABNF's own shapes, so the overwhelming majority are *accepted* and
//! the properties above have something to say; `noise` supplies arbitrary
//! Unicode, control characters, multi-byte characters and strings hundreds of
//! characters long, which is where totality is actually at risk. A generator
//! that only ever produced rejects would make every property vacuous, so
//! `the_structured_generator_produces_well_formed_tags` measures the accepted
//! fraction and fails if it collapses.

use std::collections::BTreeMap;

use purrdf_iri::langtag::{self, Extension, LanguageTag, LanguageTagBuf, Profile, TagForm};
use purrdf_iri::parse;
use purrdf_testkit::prop::prelude::*;

/// A strategy generating syntactically valid http(s) IRIs from constrained parts,
/// so we exercise the parser/normalizer over a broad-but-legal input space.
fn valid_iri() -> impl Strategy<Value = String> {
    let scheme = prop::sample::select(vec!["http", "https", "ftp"]);
    let host = prop::string::regex("[a-z][a-z0-9]{0,8}(\\.[a-z][a-z0-9]{0,8}){0,3}");
    let segs = prop::collection::vec(prop::string::regex("[a-z0-9._~-]{1,6}"), 0..4);
    let frag = prop::option::of(prop::string::regex("[a-z0-9]{0,6}"));
    (scheme, host, segs, frag).prop_map(|(s, h, segs, frag)| {
        let mut out = format!("{s}://{h}");
        for seg in segs {
            out.push('/');
            out.push_str(&seg);
        }
        if let Some(f) = frag {
            out.push('#');
            out.push_str(&f);
        }
        out
    })
}

prop_test! {
    /// The parser never rewrites its input: `as_str()` is byte-identical.
    #[test]
    fn parse_is_verbatim(s in valid_iri()) {
        let iri = parse(&s).expect("generated IRI must parse");
        prop_assert_eq!(iri.as_str(), s.as_str());
    }

    /// Normalization is idempotent.
    #[test]
    fn normalize_idempotent(s in valid_iri()) {
        let n1 = parse(&s).unwrap().normalize();
        let n2 = n1.normalize();
        prop_assert_eq!(n1.as_str(), n2.as_str());
    }

    /// Resolving any relative path-segment against an absolute base stays absolute.
    #[test]
    fn resolution_preserves_absoluteness(s in valid_iri(), rel in prop::string::regex("[a-z0-9]{1,5}(/[a-z0-9]{1,5}){0,3}")) {
        let base = parse(&s).unwrap();
        // The generated base always has a scheme, so resolution must succeed.
        let resolved = base.resolve(&rel).expect("absolute base resolves");
        prop_assert!(resolved.has_scheme());
    }
}

/// The three profiles in acceptance order, which is also their declaration
/// order and therefore their `Ord`.
const PROFILES: [Profile; 3] = [
    Profile::Rfc5646,
    Profile::Rfc5646PrivateUseRelaxed,
    Profile::ConcreteSyntaxLangtag,
];

/// A sample of the closed §2.2.8 grandfathered list — one whole-tag literal from
/// each of the shapes the ordinary grammar could not otherwise reach (a
/// one-letter primary, a three-subtag irregular tag, and regular tags that would
/// analyse into different components).
const GRANDFATHERED_SAMPLE: &[&str] = &[
    "i-enochian",
    "en-GB-oed",
    "sgn-BE-FR",
    "art-lojban",
    "zh-min-nan",
];

/// The characters a language-tag parser branches on, plus the ones that break a
/// naive byte-offset slice: a control character, a two-byte character, a
/// three-byte one and a four-byte one.
const TAG_ALPHABET: &[char] = &[
    'a', 'b', 'z', 'A', 'M', 'Z', '0', '5', '9', '-', 'x', 'X', '!', ' ', '\u{0}', 'ü', '中', '𝔘',
];

/// `language = 2*3ALPHA ["-" extlang] / 4ALPHA / 5*8ALPHA`.
///
/// The `["-" extlang]` is attached to the first alternative and to that one
/// only, which is the load-bearing boundary `langtag_corpus.rs` names: emitting
/// an `extlang` after a 4ALPHA or 5*8ALPHA primary would make the generator
/// produce rejects that look like accepts.
fn language() -> impl Strategy<Value = String> {
    prop_oneof![
        6 => (prop::string::regex("[a-zA-Z]{2,3}"), prop::collection::vec(prop::string::regex("[a-zA-Z]{3}"), 0..=3))
            .prop_map(|(primary, extlang)| hyphenate(primary, extlang)),
        1 => prop::string::regex("[a-zA-Z]{4}"),
        1 => prop::string::regex("[a-zA-Z]{5,8}"),
    ]
}

/// `region = 2ALPHA / 3DIGIT`.
fn region() -> impl Strategy<Value = String> {
    prop_oneof![
        prop::string::regex("[a-zA-Z]{2}"),
        prop::string::regex("[0-9]{3}")
    ]
}

/// `variant = 5*8alphanum / (DIGIT 3alphanum)`.
fn variant() -> impl Strategy<Value = String> {
    prop_oneof![
        prop::string::regex("[a-zA-Z0-9]{5,8}"),
        prop::string::regex("[0-9][a-zA-Z0-9]{3}")
    ]
}

/// `extension = singleton 1*("-" (2*8alphanum))`, where `singleton` is one
/// alphanumeric character with `x`/`X` carved out.
fn extension() -> impl Strategy<Value = String> {
    (
        prop::string::regex("[0-9a-wyzA-WYZ]"),
        prop::collection::vec(prop::string::regex("[a-zA-Z0-9]{2,8}"), 1..=3),
    )
        .prop_map(|(singleton, subtags)| hyphenate(singleton, subtags))
}

/// `privateuse = "x" 1*("-" (1*8alphanum))`, held to the §2.1 eight-character
/// ceiling so the section is accepted by every profile rather than only the
/// relaxed ones.
fn private_use() -> impl Strategy<Value = String> {
    (
        prop::string::regex("[xX]"),
        prop::collection::vec(prop::string::regex("[a-zA-Z0-9]{1,8}"), 1..=3),
    )
        .prop_map(|(marker, subtags)| hyphenate(marker, subtags))
}

/// `langtag = language ["-" script] ["-" region] *("-" variant)
/// *("-" extension) ["-" privateuse]`, assembled section by section.
fn langtag_shape() -> impl Strategy<Value = String> {
    (
        language(),
        prop::option::of(prop::string::regex("[a-zA-Z]{4}")),
        prop::option::of(region()),
        prop::collection::vec(variant(), 0..3),
        prop::collection::vec(extension(), 0..3),
        prop::option::of(private_use()),
    )
        .prop_map(
            |(language, script, region, variants, extensions, private_use)| {
                let sections = script
                    .into_iter()
                    .chain(region)
                    .chain(variants)
                    .chain(extensions)
                    .chain(private_use);
                hyphenate(language, sections)
            },
        )
}

/// `Language-Tag = langtag / privateuse / grandfathered` — all three
/// alternatives, weighted towards the one with structure to check.
///
/// Case is randomized by the character classes above (and, for the closed
/// grandfathered list, by an explicit fold), because case is insignificant to
/// the judgement but is the whole subject of §2.1.1 canonical case.
fn structured_tag() -> impl Strategy<Value = String> {
    prop_oneof![
        8 => langtag_shape(),
        1 => private_use(),
        1 => (prop::sample::select(GRANDFATHERED_SAMPLE.to_vec()), any::<bool>())
            .prop_map(|(tag, upper)| if upper {
                tag.to_ascii_uppercase()
            } else {
                tag.to_owned()
            }),
    ]
}

/// Unstructured input: arbitrary Unicode, dense runs of the characters the
/// grammar branches on, and strings hundreds of characters long.
///
/// This is where totality is actually at risk. `String`'s `Arbitrary` impl
/// excludes control characters, so `TAG_ALPHABET` carries a NUL explicitly.
fn noise() -> impl Strategy<Value = String> {
    prop_oneof![
        4 => any::<String>(),
        4 => prop::collection::vec(prop::sample::select(TAG_ALPHABET.to_vec()), 0..40)
            .prop_map(|chars| chars.into_iter().collect::<String>()),
        1 => prop::collection::vec(prop::sample::select(TAG_ALPHABET.to_vec()), 200..600)
            .prop_map(|chars| chars.into_iter().collect::<String>()),
        1 => prop::string::regex("[a-zA-Z0-9-]{100,600}"),
    ]
}

/// Either generator, so every property below sees both accepted and refused
/// input.
fn any_tag() -> impl Strategy<Value = String> {
    prop_oneof![structured_tag(), noise()]
}

/// `head`, then every section of `rest`, joined by the ABNF's `-`.
fn hyphenate(head: String, rest: impl IntoIterator<Item = String>) -> String {
    let mut tag = head;
    for section in rest {
        tag.push('-');
        tag.push_str(&section);
    }
    tag
}

/// Every accessor of an accepted tag, checked against the invariants the module
/// states: the input is kept verbatim, every reported section is a slice of it,
/// and `is_canonical_case` answers the same question as the allocating rewrite.
fn check_accepted(tag: &str, parsed: &LanguageTag<'_>) -> Result<(), TestCaseError> {
    prop_assert_eq!(parsed.as_str(), tag, "the input must be kept verbatim");

    let sections = [
        parsed.primary_language(),
        parsed.extended_language(),
        parsed.script(),
        parsed.region(),
        parsed.extensions(),
        parsed.private_use(),
    ];
    for section in sections.into_iter().flatten() {
        prop_assert!(
            tag.contains(section),
            "{section:?} must be a slice of {tag:?}"
        );
    }

    // The grouped sections are recovered by re-splitting the run, so drain both
    // iterators: that is the code path with the byte arithmetic in it.
    for variant in parsed.variants() {
        prop_assert!(!variant.is_empty());
    }
    for extension in parsed.extensions_by_singleton() {
        prop_assert!(!extension.as_str().is_empty());
        prop_assert_eq!(
            extension.as_str().chars().next(),
            Some(extension.singleton())
        );
        prop_assert!(
            extension.subtags().next().is_some(),
            "`extension` requires at least one subtag"
        );
        prop_assert_eq!(parsed.extension(extension.singleton()).is_some(), true);
    }
    for subtag in parsed.private_use_subtags() {
        prop_assert!(!subtag.is_empty());
    }

    // A tag with no decomposable sections reports none rather than empty ones.
    if parsed.form() == TagForm::ConcreteSyntaxOnly {
        prop_assert!(sections.iter().all(Option::is_none));
    }

    // The non-allocating predicate must agree with the allocating rewrite.
    prop_assert_eq!(
        parsed.is_canonical_case(),
        parsed.as_str() == parsed.canonical_case()
    );
    Ok(())
}

/// A refusal must name a production: a typed error with a stable `langtag-`
/// code and a non-empty message, and `is_well_formed_with` must agree with it.
fn check_refused(tag: &str, profile: Profile) -> Result<(), TestCaseError> {
    let error = langtag::parse_with(tag, profile).expect_err("caller observed a refusal");
    prop_assert!(error.diagnostic_code().starts_with("langtag-"));
    prop_assert!(!error.message().is_empty());
    prop_assert_eq!(error.to_string(), error.message());
    prop_assert!(!langtag::is_well_formed_with(tag, profile));
    prop_assert!(langtag::canonical_case_with(tag, profile).is_err());
    Ok(())
}

/// The parser is **total**: no input panics it under any profile, and
/// neither does walking every section of whatever it accepts.
///
/// This is not a formality. The parser reports each section as a byte range
/// and slices the input with it, and the grouped-section iterators do their
/// own byte arithmetic (`end = offset - 1`), so a multi-byte character or an
/// off-by-one would be a panic rather than a wrong answer. The noise
/// generator therefore carries two-, three- and four-byte characters, a NUL,
/// and strings hundreds of characters long.
fn langtag_parsing_is_total_over_arbitrary_input_holds(tag: &str) -> Result<(), TestCaseError> {
    for profile in PROFILES {
        match langtag::parse_with(tag, profile) {
            Ok(parsed) => {
                prop_assert!(langtag::is_well_formed_with(tag, profile));
                check_accepted(tag, &parsed)?;
            }
            Err(_) => check_refused(tag, profile)?,
        }
    }
    // The profile-free entry points are `Profile::Rfc5646` exactly.
    prop_assert_eq!(
        langtag::is_well_formed(tag),
        langtag::is_well_formed_with(tag, Profile::Rfc5646)
    );
    prop_assert_eq!(
        langtag::parse(tag).is_ok(),
        langtag::parse_with(tag, Profile::Rfc5646).is_ok()
    );
    Ok(())
}

/// §2.1.1 canonical case is **idempotent**: rewriting a canonical tag is the
/// identity, so the rewrite is a fixed point rather than a rotation.
fn langtag_canonical_case_is_idempotent_holds(tag: &str) -> Result<(), TestCaseError> {
    for profile in PROFILES {
        let Ok(once) = langtag::canonical_case_with(tag, profile) else {
            continue;
        };
        let twice = langtag::canonical_case_with(&once, profile)
            .expect("the canonical form of an accepted tag is accepted");
        prop_assert_eq!(&once, &twice);
        prop_assert!(
            langtag::parse_with(&once, profile)
                .expect("accepted")
                .is_canonical_case()
        );
    }
    Ok(())
}

/// Canonical case **preserves acceptance**: if a tag parses under a profile
/// its canonical form parses under that profile too, to the same `TagForm`
/// and the same section boundaries.
///
/// Case is insignificant to every production, so the rewrite must touch
/// bytes and never subtag boundaries — a normalizer that moved one would be
/// a silent corruption of an accepted tag.
fn langtag_canonical_case_preserves_acceptance_holds(tag: &str) -> Result<(), TestCaseError> {
    for profile in PROFILES {
        let Ok(parsed) = langtag::parse_with(tag, profile) else {
            continue;
        };
        let canonical = parsed.canonical_case();
        let renormalized = langtag::parse_with(&canonical, profile)
            .expect("the canonical form must stay accepted");
        prop_assert_eq!(parsed.form(), renormalized.form());

        // Grandfathered tags are canonical by registration rather than by
        // rule, so their spelling is looked up; every other form is a
        // case-only rewrite and keeps its length and its sections.
        if parsed.form() != TagForm::Grandfathered {
            prop_assert_eq!(canonical.len(), tag.len());
            prop_assert_eq!(
                parsed.primary_language().map(str::len),
                renormalized.primary_language().map(str::len)
            );
            prop_assert_eq!(
                parsed.script().map(str::len),
                renormalized.script().map(str::len)
            );
            prop_assert_eq!(parsed.variants().count(), renormalized.variants().count());
            prop_assert_eq!(
                parsed.extensions_by_singleton().count(),
                renormalized.extensions_by_singleton().count()
            );
            prop_assert_eq!(
                parsed.private_use_subtags().count(),
                renormalized.private_use_subtags().count()
            );
            prop_assert_eq!(
                canonical.to_ascii_lowercase(),
                tag.to_ascii_lowercase(),
                "canonical case may change case and nothing else"
            );
        }
    }
    Ok(())
}

/// The profiles are **totally ordered by acceptance** —
/// `Rfc5646` ⊆ `Rfc5646PrivateUseRelaxed` ⊆ `ConcreteSyntaxLangtag` — and a
/// widening never costs a caller its decomposition: a tag the narrower
/// profile accepts parses to the *same* record under every wider one.
fn langtag_profiles_are_ordered_by_acceptance_holds(tag: &str) -> Result<(), TestCaseError> {
    let strict = langtag::parse_with(tag, Profile::Rfc5646);
    let relaxed = langtag::parse_with(tag, Profile::Rfc5646PrivateUseRelaxed);
    let terminal = langtag::parse_with(tag, Profile::ConcreteSyntaxLangtag);

    if let Ok(strict) = strict {
        prop_assert_eq!(relaxed, Ok(strict));
    }
    if let Ok(relaxed) = relaxed {
        prop_assert_eq!(terminal, Ok(relaxed));
    }
    // …and the ordering is the declared `Ord` on the enum, so a caller can
    // compare profiles directly rather than remembering the chain.
    prop_assert!(Profile::Rfc5646 < Profile::Rfc5646PrivateUseRelaxed);
    prop_assert!(Profile::Rfc5646PrivateUseRelaxed < Profile::ConcreteSyntaxLangtag);
    prop_assert_eq!(Profile::default(), Profile::Rfc5646);

    // Acceptance is monotone in that order.
    let accepted = PROFILES.map(|profile| langtag::is_well_formed_with(tag, profile));
    for window in accepted.windows(2) {
        prop_assert!(
            !window[0] || window[1],
            "a wider profile must accept everything a narrower one does"
        );
    }
    prop_assert_eq!(terminal.is_ok(), accepted[2]);
    Ok(())
}

/// The owning and borrowing forms **agree**: same record, same sections,
/// same string, in both directions.
fn langtag_owning_and_borrowing_forms_agree_holds(tag: &str) -> Result<(), TestCaseError> {
    for profile in PROFILES {
        let borrowed = langtag::parse_with(tag, profile);
        let owned = LanguageTagBuf::parse_with(tag, profile);
        prop_assert_eq!(borrowed.is_ok(), owned.is_ok());
        let (Ok(borrowed), Ok(owned)) = (borrowed, owned) else {
            continue;
        };

        prop_assert_eq!(owned.as_language_tag(), borrowed);
        prop_assert_eq!(&borrowed.to_owned_tag(), &owned);
        prop_assert_eq!(&LanguageTagBuf::from(borrowed), &owned);
        prop_assert_eq!(owned.as_str(), borrowed.as_str());
        prop_assert_eq!(owned.to_string(), borrowed.to_string());
        prop_assert_eq!(owned.form(), borrowed.form());
        prop_assert_eq!(owned.primary_language(), borrowed.primary_language());
        prop_assert_eq!(owned.extended_language(), borrowed.extended_language());
        prop_assert_eq!(owned.script(), borrowed.script());
        prop_assert_eq!(owned.region(), borrowed.region());
        prop_assert_eq!(owned.extensions(), borrowed.extensions());
        prop_assert_eq!(owned.private_use(), borrowed.private_use());
        prop_assert_eq!(
            owned.variants().collect::<Vec<_>>(),
            borrowed.variants().collect::<Vec<_>>()
        );
        prop_assert_eq!(
            owned
                .extensions_by_singleton()
                .map(Extension::as_str)
                .collect::<Vec<_>>(),
            borrowed
                .extensions_by_singleton()
                .map(Extension::as_str)
                .collect::<Vec<_>>()
        );
        prop_assert_eq!(
            owned.private_use_subtags().collect::<Vec<_>>(),
            borrowed.private_use_subtags().collect::<Vec<_>>()
        );
        prop_assert_eq!(owned.canonical_case(), borrowed.canonical_case());
        prop_assert_eq!(owned.is_canonical_case(), borrowed.is_canonical_case());

        // The owning form's own round trip: normalize in place, and take the
        // `String` back out.
        let rewritten = owned.canonical_case();
        let canonical = owned.clone().into_canonical_case();
        prop_assert_eq!(canonical.as_str(), rewritten.as_str());
        prop_assert!(canonical.is_canonical_case());
        prop_assert_eq!(owned.clone().into_string(), owned.as_str());

        // `FromStr` is `parse` with the default profile, and no other.
        if profile == Profile::Rfc5646 {
            prop_assert_eq!(tag.parse::<LanguageTagBuf>(), Ok(owned));
        }
    }
    Ok(())
}

prop_test! {
    #![prop_config(Config::with_cases(512))]

    #[test]
    fn langtag_parsing_is_total_over_arbitrary_input(tag in any_tag()) {
        langtag_parsing_is_total_over_arbitrary_input_holds(&tag)?;
    }

    #[test]
    fn langtag_canonical_case_is_idempotent(tag in any_tag()) {
        langtag_canonical_case_is_idempotent_holds(&tag)?;
    }

    #[test]
    fn langtag_canonical_case_preserves_acceptance(tag in any_tag()) {
        langtag_canonical_case_preserves_acceptance_holds(&tag)?;
    }

    #[test]
    fn langtag_profiles_are_ordered_by_acceptance(tag in any_tag()) {
        langtag_profiles_are_ordered_by_acceptance_holds(&tag)?;
    }

    #[test]
    fn langtag_owning_and_borrowing_forms_agree(tag in any_tag()) {
        langtag_owning_and_borrowing_forms_agree_holds(&tag)?;
    }
}

/// The structured generator must actually produce **accepted** tags, or every
/// property above is a tautology over rejects.
///
/// Run on a deterministic RNG so the reported fraction is reproducible rather
/// than a different number on every CI run. The floor is deliberately well below
/// the observed rate: this guards against a generator that *collapses*, not
/// against ordinary drift.
#[test]
fn the_structured_generator_produces_well_formed_tags() {
    const SAMPLE: usize = 4_000;
    const FLOOR_PERCENT: usize = 90;

    let mut choices = prop::Choices::random(prop::seed_for(concat!(
        module_path!(),
        "::the_structured_generator_produces_well_formed_tags"
    )));
    let strategy = structured_tag();
    let mut accepted = 0usize;
    let mut forms: BTreeMap<TagForm, usize> = BTreeMap::new();
    for _ in 0..SAMPLE {
        let tag = strategy
            .generate(&mut choices)
            .expect("the structured strategy always yields a value");
        if let Ok(parsed) = langtag::parse(&tag) {
            accepted += 1;
            *forms.entry(parsed.form()).or_default() += 1;
        }
    }

    let percent = accepted * 100 / SAMPLE;
    println!(
        "structured generator: {accepted}/{SAMPLE} accepted under RFC 5646 ({percent}%), forms: {forms:?}"
    );
    assert!(
        percent >= FLOOR_PERCENT,
        "only {accepted}/{SAMPLE} ({percent}%) of generated tags were accepted; \
         a generator that mostly produces rejects proves nothing"
    );
    // All three RFC 5646 alternatives must be reached, not just the common one.
    for form in [
        TagForm::Langtag,
        TagForm::PrivateUse,
        TagForm::Grandfathered,
    ] {
        assert!(
            forms.contains_key(&form),
            "the generator never produced a {form:?} tag: {forms:?}"
        );
    }
}

/// Counterexamples found by an earlier property run, kept as ordinary tests.
///
/// Each entry is the choice sequence (hex, as a failing property prints it) that
/// makes [`any_tag`] generate the tag, and the tag itself: the test first proves
/// the sequence still decodes to that exact tag — so the stored input cannot
/// drift from what the generator means — and then runs every language-tag
/// property over it. `SHRUNK_TAG` is the shrunk counterexample; `SEEDED_TAG`
/// is the input its seed generated before shrinking.
const LANGTAG_REGRESSIONS: [(&str, &str); 2] = [
    (
        "SHRUNK_TAG",
        "01030b0025250b0101002500250025000b0100250125250101250b25250b0101250101250b010125000b00010b25250125000b010101012501002525000b0b00250b010b010b2525250b0b000b2501010b0b01010b010b0b252500252501000b0b0b000b252500",
    ),
    (
        "SEEDED_TAG",
        "01030f00022c00392104312a282e291a3c1907230700240400071e173e0016250027332f23060800330026003e001405002905292f06043d11343c240207370a07321d06012e001f0003192a3e0326001e0a0502053801003a2e001a2000270c031609192537013d010c011e0100011101370107010a011b010b01060103010f0104010c0114012c013801000138012701010100011f0123010d010001230132013a00",
    ),
];

#[test]
fn langtag_regressions_decode_to_their_tags_and_hold_every_property() {
    let expected = [
        "A-aaA00-a-a-a-A0-a0aa00aAaaA00a00aA00a-A-0Aaa0a-A0000a0-aa-AA-aA0A0AaaaAA-Aa00AA00A0AAaa-aa0-AAA-Aaa",
        "E-1h-uW3mfdjePxO6Y6-Z3-6TMz-La-cokY57-o-b-z-J4-e4ek53yGpxZ16s96nS50j-U-2Ofz2b-T9414t0-vj-PV-cB2L8OasyBT-Gs69QA52E3BJht-tc0-UYC-Ynv",
    ];
    for ((label, hex), tag) in LANGTAG_REGRESSIONS.into_iter().zip(expected) {
        assert_eq!(
            prop::replay(&any_tag(), hex),
            tag,
            "{label} decodes to its tag"
        );
        for (property, outcome) in [
            (
                "parsing_is_total",
                langtag_parsing_is_total_over_arbitrary_input_holds(tag),
            ),
            (
                "canonical_case_is_idempotent",
                langtag_canonical_case_is_idempotent_holds(tag),
            ),
            (
                "canonical_case_preserves_acceptance",
                langtag_canonical_case_preserves_acceptance_holds(tag),
            ),
            (
                "profiles_are_ordered_by_acceptance",
                langtag_profiles_are_ordered_by_acceptance_holds(tag),
            ),
            (
                "owning_and_borrowing_forms_agree",
                langtag_owning_and_borrowing_forms_agree_holds(tag),
            ),
        ] {
            if let Err(error) = outcome {
                panic!("{label}: {property} fails on {tag:?}: {error}");
            }
        }
    }
}
