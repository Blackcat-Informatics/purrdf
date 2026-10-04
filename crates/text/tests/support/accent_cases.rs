// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Every supported accent set, shared by the native and WASM runners.

use std::collections::BTreeSet;
use std::sync::Arc;

use purrdf_text::unicode::AccentScript;
use purrdf_text::{
    AccentFold, AccentScripts, Analyzer, AnalyzerProfile, AnalyzerScratch, Segmentation,
    segment::Dictionary,
};

const SAMPLES: [(AccentScript, &str, &str); 5] = [
    (AccentScript::Latin, "é", "e"),
    (AccentScript::Greek, "ά", "α"),
    (AccentScript::Cyrillic, "й", "и"),
    (AccentScript::Arabic, "بَ", "ب"),
    (AccentScript::Hebrew, "בּ", "ב"),
];

pub(crate) fn every_accent_subset_is_independent_and_identified() {
    let mut profile_ids = BTreeSet::new();
    let mut analyzer_ids = BTreeSet::new();
    for subset in 0..32 {
        let members: Vec<_> = SAMPLES
            .iter()
            .enumerate()
            .filter_map(|(at, &(script, _, _))| (subset & (1 << at) != 0).then_some(script))
            .collect();
        let scripts = AccentScripts::from_scripts(members.iter().copied());
        let reversed = AccentScripts::from_scripts(members.iter().rev().chain(&members).copied());
        assert_eq!(scripts, reversed);
        assert_eq!(scripts.iter().collect::<Vec<_>>(), members);
        assert_eq!(scripts.is_empty(), subset == 0);
        let profile =
            AnalyzerProfile::empty_lexicon().with_accent_fold(AccentFold::Selected(scripts));
        let repeated =
            AnalyzerProfile::empty_lexicon().with_accent_fold(AccentFold::Selected(reversed));
        assert_eq!(profile, repeated);
        assert_eq!(profile.fingerprint(), repeated.fingerprint());
        assert!(profile_ids.insert(profile.fingerprint()));
        let analyzer = Analyzer::with_profile(profile.clone()).unwrap();
        assert!(analyzer_ids.insert(analyzer.fingerprint()));
        let mut scratch = AnalyzerScratch::default();
        for &(script, input, folded) in &SAMPLES {
            let expected = if scripts.contains(script) {
                folded
            } else {
                input
            };
            assert_eq!(
                analyzer.analysis_form(input).unwrap(),
                expected,
                "{members:?}"
            );
            let projections = analyzer.projections(input).unwrap();
            assert_eq!(projections.normalized.text, expected, "{members:?}");
            assert_eq!(projections.lexical[0].text, expected, "{members:?}");
            assert_eq!(projections.surface[0].text, expected, "{members:?}");
            let mut streamed = Vec::new();
            analyzer
                .analyze_each_with_scratch(input, &mut scratch, |token| {
                    streamed.push(token.text.into_owned());
                })
                .unwrap();
            assert_eq!(streamed, [expected], "{members:?}");

            // Caller dictionaries normalize through the same selected set.
            // A compound entry forces an observable whole-word lattice edge.
            let dictionary = Dictionary::new([format!("{input}中文")]).unwrap();
            let dictionary = Analyzer::with_profile(
                profile
                    .clone()
                    .with_segmentation(Segmentation::Dictionary(Arc::new(dictionary))),
            )
            .unwrap();
            assert_eq!(
                dictionary.terms(&format!("{input}中文")).unwrap(),
                [format!("{expected}中文")]
            );
            if scripts.contains(script) {
                assert_eq!(
                    dictionary.terms(&format!("{folded}中文")).unwrap(),
                    [format!("{folded}中文")]
                );
            } else {
                assert_ne!(
                    dictionary.terms(&format!("{folded}中文")).unwrap(),
                    [format!("{folded}中文")]
                );
            }
        }
        for input in ["ก่", "ກ່", "កិ", "က့", "अं", "का", "\u{301}", "e\u{17c6}"]
        {
            assert_eq!(analyzer.analysis_form(input).unwrap(), input, "{members:?}");
        }
        let arabic = if scripts.contains(AccentScript::Arabic) {
            "ا"
        } else {
            "أ"
        };
        assert_eq!(analyzer.analysis_form("أ").unwrap(), arabic);
        let aligned = analyzer.projections("e\u{301}").unwrap();
        let source = if scripts.contains(AccentScript::Latin) {
            0..1
        } else {
            0..3
        };
        assert_eq!(
            aligned
                .normalized
                .contributors(0..aligned.normalized.text.len()),
            std::slice::from_ref(&source)
        );
        assert_eq!(
            analyzer.analysis_form("éάйبَבּ").unwrap(),
            SAMPLES
                .iter()
                .map(|&(script, original, folded)| if scripts.contains(script) {
                    folded
                } else {
                    original
                })
                .collect::<String>()
        );
    }
    assert_eq!(profile_ids.len(), 32);
    assert_eq!(analyzer_ids.len(), 32);
    presets_keep_their_existing_identity();
}

fn presets_keep_their_existing_identity() {
    // Frozen preimages from the original three-preset profile format. These
    // BLAKE3 answers were independently calculated before adding script sets.
    for (preset, members, expected) in [
        (
            AccentFold::Preserve,
            &[][..],
            "f70268c28cf081386dc5b13e1bd53b112eab2d4f736cf0d5249537fdba752fb0",
        ),
        (
            AccentFold::LatinGreekCyrillic,
            &[
                AccentScript::Latin,
                AccentScript::Greek,
                AccentScript::Cyrillic,
            ][..],
            "aec4363550e18e2d31b353f69f189da465f219b53bf29f3f5444657b04067048",
        ),
        (
            AccentFold::LatinGreekCyrillicArabicHebrew,
            &[
                AccentScript::Latin,
                AccentScript::Greek,
                AccentScript::Cyrillic,
                AccentScript::Arabic,
                AccentScript::Hebrew,
            ][..],
            "13b1fab9ed2689ea0c2ececb7307521d04c6ecec85c090a9433b226c17c72507",
        ),
    ] {
        let preset_profile = AnalyzerProfile::empty_lexicon().with_accent_fold(preset);
        let selected = AnalyzerProfile::empty_lexicon().with_accent_fold(AccentFold::Selected(
            AccentScripts::from_scripts(members.iter().copied()),
        ));
        assert_eq!(preset_profile, selected);
        assert_eq!(selected.accent_fold(), preset);
        assert_eq!(
            purrdf_hash::hex::encode(&preset_profile.fingerprint()),
            expected
        );
        assert_eq!(preset_profile.fingerprint(), selected.fingerprint());
        assert_eq!(
            Analyzer::with_profile(preset_profile).unwrap(),
            Analyzer::with_profile(selected).unwrap()
        );
    }
}
