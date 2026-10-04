// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Dictionary lattice vectors and exhaustive checks against the pinned UCD.

use purrdf_text::TextError;
use purrdf_text::segment::{Dictionary, SegmentationScratch};
use purrdf_text::unicode::{self, SegmentationScript};

fn dictionary(words: &[&str]) -> Dictionary {
    Dictionary::new(words.iter().map(|word| (*word).to_owned())).expect("valid test vocabulary")
}

#[test]
fn dictionaries_segment_each_requested_script_and_mixed_japanese() {
    let cases: &[(&str, &[&str])] = &[
        ("中文知识图谱", &["中文", "知识", "图谱"]),
        (
            "私はサンドイッチを食べます",
            &["私", "は", "サンドイッチ", "を", "食べます"],
        ),
        ("한국어전문검색", &["한국어", "전문", "검색"]),
        ("ภาษาไทย", &["ภาษา", "ไทย"]),
        ("ພາສາລາວ", &["ພາສາ", "ລາວ"]),
        ("ភាសាខ្មែរ", &["ភាសា", "ខ្មែរ"]),
        ("မြန်မာဘာသာ", &["မြန်မာ", "ဘာသာ"]),
    ];
    for &(input, expected) in cases {
        assert_eq!(dictionary(expected).segments(input), expected, "{input}");
    }
}

#[test]
fn technical_chinese_words_can_mix_scripts_digits_and_internal_punctuation() {
    let entries = [
        "RDF数据模型",
        "支持",
        "SPARQL查询",
        "和",
        "UTF8编码",
        "也兼容",
        "RDF1.2模型",
    ];
    let dictionary = dictionary(&entries);
    let input = "RDF数据模型支持SPARQL查询和UTF8编码，也兼容RDF1.2模型";
    let mut normalized = String::new();
    unicode::analysis_form(input, &mut normalized);
    assert_eq!(
        dictionary.segments(&normalized),
        [
            "rdf数据模型",
            "支持",
            "sparql查询",
            "和",
            "utf8编码",
            "也兼容",
            "rdf1.2模型"
        ]
    );
    assert_eq!(dictionary.segments("rdf1.2模型"), ["rdf1.2模型"]);
    assert_eq!(dictionary.segments("rdf1.3模型"), ["rdf1.3", "模", "型"]);
}

#[test]
fn complete_coverage_wins_over_a_greedy_longest_match() {
    let dictionary = dictionary(&["研究", "研究生", "生命", "起源"]);
    assert_eq!(
        dictionary.segments("研究生命起源"),
        ["研究", "生命", "起源"]
    );
}

#[test]
fn integer_lexical_costs_resolve_chinese_ambiguity_and_are_fingerprinted() {
    let make = |cost| {
        Dictionary::with_costs(
            [
                ("研究", 1),
                ("研究生", 1),
                ("生命", cost),
                ("命", 3),
                ("起源", 1),
            ]
            .map(|(word, cost)| (word.to_owned(), cost)),
        )
        .expect("weighted vocabulary")
    };
    let research_life = make(1);
    let graduate_life = make(5);
    assert_eq!(
        research_life.segments("研究生命起源"),
        ["研究", "生命", "起源"]
    );
    assert_eq!(
        graduate_life.segments("研究生命起源"),
        ["研究生", "命", "起源"]
    );
    assert_ne!(research_life.fingerprint(), graduate_life.fingerprint());
    assert_eq!(
        research_life.segments("研究"),
        graduate_life.segments("研究")
    );

    let expensive =
        Dictionary::with_costs(["研究", "生命", "起源"].map(|word| (word.to_owned(), u32::MAX)))
            .expect("all u32 costs are valid");
    assert_eq!(expensive.segments("研究生命起源"), ["研究", "生命", "起源"]);
}

#[test]
fn lattice_agrees_with_exhaustive_partition_ranking_on_small_chinese_corpora() {
    let vocabulary = [
        ("中", 3),
        ("文", 2),
        ("中文", 4),
        ("文中", 0),
        ("中文中", 6),
        ("文中文", 1),
    ];
    for dictionary_mask in 0..1u32 << vocabulary.len() {
        let entries: Vec<(&str, u32)> = vocabulary
            .iter()
            .copied()
            .enumerate()
            .filter_map(|(at, entry)| (dictionary_mask & (1 << at) != 0).then_some(entry))
            .collect();
        let dictionary =
            Dictionary::with_costs(entries.iter().map(|&(word, cost)| (word.to_owned(), cost)))
                .expect("finite test lexicon");
        for length in 1..=5 {
            for spelling in 0..1u32 << length {
                let input: String = (0..length)
                    .map(|at| {
                        if spelling & (1 << at) == 0 {
                            '中'
                        } else {
                            '文'
                        }
                    })
                    .collect();
                let boundaries: Vec<usize> = input
                    .char_indices()
                    .map(|(at, _)| at)
                    .chain([input.len()])
                    .collect();
                let mut candidates = Vec::new();
                // Enumerate every partition, independently of the trie and DP.
                for breaks in 0..1u32 << (length - 1) {
                    let mut start = 0;
                    let mut tokens = Vec::new();
                    let mut unknown = 0;
                    let mut cost = 0u128;
                    let mut lengths = Vec::new();
                    let mut valid = true;
                    for end in 1..=length {
                        if end != length && breaks & (1 << (end - 1)) == 0 {
                            continue;
                        }
                        let term = &input[boundaries[start]..boundaries[end]];
                        if let Some((_, lexical_cost)) =
                            entries.iter().find(|(word, _)| *word == term)
                        {
                            cost += u128::from(*lexical_cost);
                        } else if end - start == 1 {
                            unknown += 1;
                        } else {
                            valid = false;
                            break;
                        }
                        tokens.push(term);
                        lengths.push(core::cmp::Reverse(end - start));
                        start = end;
                    }
                    if valid {
                        candidates.push(((unknown, cost, tokens.len(), lengths), tokens));
                    }
                }
                let expected = candidates
                    .into_iter()
                    .min_by(|left, right| left.0.cmp(&right.0))
                    .expect("the all-single-unit partition always exists")
                    .1;
                assert_eq!(
                    dictionary.segments(&input),
                    expected,
                    "{entries:?}, {input}"
                );
            }
        }
    }
}

#[test]
fn zero_costs_and_exact_ties_have_a_total_insertion_independent_order() {
    let entries = ["南京", "市长", "南京市", "长"];
    let forward =
        Dictionary::with_costs(entries.map(|word| (word.to_owned(), 0))).expect("zero costs");
    let reverse =
        Dictionary::with_costs(entries.into_iter().rev().map(|word| (word.to_owned(), 0)))
            .expect("zero costs");
    assert_eq!(forward, reverse);
    assert_eq!(forward.fingerprint(), reverse.fingerprint());
    assert_eq!(forward.segments("南京市长"), ["南京市", "长"]);
    assert_eq!(reverse.segments("南京市长"), forward.segments("南京市长"));
}

#[test]
fn canonical_words_and_equal_cost_duplicates_have_one_identity() {
    let canonical = dictionary(&["パーティ", "rdf数据", "strasse模型"]);
    let variants = dictionary(&[
        "ＲＤＦ数据",
        "rdf数据",
        "ﾊﾟｰﾃｨ",
        "パーティ",
        "Straße模型",
        "STRASSE模型",
        "strasse模型",
    ]);
    assert_eq!(canonical, variants);
    assert_eq!(canonical.fingerprint(), variants.fingerprint());
    assert_eq!(canonical.segments("パーティ"), ["パーティ"]);
    assert_ne!(
        canonical.fingerprint(),
        dictionary(&["パーティ"]).fingerprint()
    );
    assert!(matches!(
        Dictionary::with_costs([("RDF模型".to_owned(), 1), ("rdf模型".to_owned(), 2)]),
        Err(TextError::Config(_))
    ));
}

#[test]
fn invalid_dictionary_entries_are_hard_errors() {
    for input in [
        "",
        " ",
        "two words",
        "中\u{A0}文",
        "中\u{200B}文",
        "中\u{202E}文",
        "中\u{2067}文",
        "中\0文",
        "中文\n",
        "中\u{2028}文",
        "中👩‍💻文",
        "中🧑‍🦊文",
        "中1️⃣文",
        "中🇨🇦文",
    ] {
        assert!(
            matches!(
                Dictionary::new([input.to_owned()]),
                Err(TextError::Config(_))
            ),
            "{input:?} must be refused"
        );
    }
}

#[test]
fn empty_dictionary_respects_unicode_units_in_every_script() {
    let dictionary = Dictionary::empty();
    assert_eq!(dictionary, Dictionary::default());
    assert!(dictionary.is_empty());
    assert_eq!(dictionary.segments("中文"), ["中", "文"]);
    assert_eq!(dictionary.segments("กัข"), ["กั", "ข"]);
    assert_eq!(dictionary.segments("ກັຂ"), ["ກັ", "ຂ"]);
    assert_eq!(dictionary.segments("កាខ"), ["កា", "ខ"]);
    assert_eq!(dictionary.segments("ကာခ"), ["က", "ာ", "ခ"]);
    assert_eq!(dictionary.segments("漢\u{E0100}字"), ["漢\u{E0100}", "字"]);
    assert_eq!(dictionary.segments("か\u{3099}き"), ["か\u{3099}", "き"]);
    assert_eq!(
        dictionary.segments("中rust文ไทยກັ"),
        ["中", "rust", "文", "ไ", "ท", "ย", "ກັ"]
    );
}

#[test]
fn dictionary_edges_cannot_split_a_base_from_its_marks() {
    let dictionary = dictionary(&["ก", "ข", "漢"]);
    assert_eq!(dictionary.segments("กัข"), ["กั", "ข"]);
    assert_eq!(dictionary.segments("漢\u{E0100}漢"), ["漢\u{E0100}", "漢"]);
}

#[test]
fn rare_han_extensions_are_script_data_not_a_frozen_block_approximation() {
    // Extension B, G, H, I and Unicode 17's Extension J.
    let text = "\u{20000}\u{30000}\u{31350}\u{2EBF0}\u{323B0}";
    for c in text.chars() {
        assert_eq!(
            unicode::segmentation_script(c),
            Some(SegmentationScript::Han)
        );
    }
    assert_eq!(Dictionary::empty().segments(text).len(), 5);
    let dictionary = dictionary(&[text]);
    assert_eq!(dictionary.segments(text), [text]);
}

#[test]
fn punctuation_and_separators_do_not_fabricate_dictionary_matches() {
    let dictionary = dictionary(&["知识图谱", "RDF1.2模型"]);
    assert_eq!(dictionary.segments("知识 图谱"), ["知", "识", "图", "谱"]);
    assert_eq!(dictionary.segments("知识，图谱"), ["知", "识", "图", "谱"]);
    assert_eq!(
        dictionary.segments("知识\u{200B}图谱"),
        ["知", "识", "图", "谱"]
    );
    assert_eq!(dictionary.segments("rdf1.2模型"), ["rdf1.2模型"]);
    assert_eq!(dictionary.segments("rdf1,2模型"), ["rdf1,2", "模", "型"]);
    assert_eq!(dictionary.segments("👩\u{200D}💻 ... !!!"), [] as [&str; 0]);
}

#[test]
fn other_scripts_keep_native_word_boundaries_and_scratch_can_be_reused() {
    let dictionary = Dictionary::empty();
    let mut scratch = SegmentationScratch::default();
    for input in ["don't 3.14 2,718 foo-bar", "σοφόσ русский", "", "again"] {
        let expected: Vec<&str> = unicode::word_indices(input).map(|(_, word)| word).collect();
        let mut actual = Vec::new();
        dictionary.segment_each_with_scratch(input, &mut scratch, |word| actual.push(word));
        assert_eq!(actual, expected, "{input}");
    }
}

#[test]
fn lexical_weights_are_canonical_and_reusable_for_profile_normalization() {
    let dictionary = Dictionary::with_costs([
        ("RDF模型".to_owned(), 7),
        ("中文".to_owned(), 0),
        ("RDF模型".to_owned(), 7),
    ])
    .expect("dictionary");
    assert_eq!(
        dictionary.weighted_entries().collect::<Vec<_>>(),
        [("rdf模型", 7), ("中文", 0)]
    );
    let reconstructed = Dictionary::with_costs(
        dictionary
            .weighted_entries()
            .map(|(word, cost)| (word.to_owned(), cost)),
    )
    .expect("canonical entries remain valid");
    assert_eq!(dictionary, reconstructed);
}

#[test]
fn property_vectors_distinguish_nonspacing_from_combining_class_and_marks() {
    assert!(unicode::is_nonspacing_mark('\u{900}'));
    assert_eq!(purrdf_lex::unicode::ccc('\u{900}'), 0);
    assert!(unicode::is_nonspacing_mark('\u{301}'));
    assert!(unicode::is_combining_mark('\u{17B6}'));
    assert!(!unicode::is_nonspacing_mark('\u{17B6}'));
    for c in [
        '\u{200B}', '\u{200D}', '\u{200E}', '\u{202E}', '\u{2066}', '\u{FE0F}',
    ] {
        assert!(unicode::is_default_ignorable(c), "U+{:04X}", u32::from(c));
    }
    assert!(unicode::is_bidi_control('\u{2069}'));
    assert!(!unicode::is_bidi_control('\u{200D}'));
    assert!(unicode::is_extended_pictographic('👩'));
    assert!(unicode::is_word_extend('\u{FE0F}'));
    assert_eq!(unicode::segmentation_script('ー'), None);
    assert!(unicode::has_segmentation_script(
        'ー',
        SegmentationScript::Hiragana
    ));
    assert!(unicode::has_segmentation_script(
        'ー',
        SegmentationScript::Katakana
    ));
    assert!(!unicode::has_segmentation_script(
        'ー',
        SegmentationScript::Han
    ));
}

#[test]
fn generated_analysis_properties_match_the_ucd_for_every_scalar() {
    use purrdf_testkit::ucd::{code_point, unicode_data};

    const NONSPACING: u32 = 1;
    const MARK: u32 = 2;
    const IGNORABLE: u32 = 4;
    const BIDI: u32 = 8;
    const SEPARATOR: u32 = 16;
    const PICTOGRAPHIC: u32 = 32;
    const EXTEND: u32 = 64;
    let scripts = [
        ("Han", "Hani", SegmentationScript::Han),
        ("Hiragana", "Hira", SegmentationScript::Hiragana),
        ("Katakana", "Kana", SegmentationScript::Katakana),
        ("Hangul", "Hang", SegmentationScript::Hangul),
        ("Thai", "Thai", SegmentationScript::Thai),
        ("Lao", "Laoo", SegmentationScript::Lao),
        ("Khmer", "Khmr", SegmentationScript::Khmer),
        ("Myanmar", "Mymr", SegmentationScript::Myanmar),
    ];
    let root = purrdf_testkit::paths::workspace_root().join("crates/iri/unicode/17.0.0");
    let read = |name| std::fs::read_to_string(root.join(name)).expect("vendored UCD file");
    let mut expected = vec![0u32; 0x11_0000];
    for (point, fields) in unicode_data(&read("UnicodeData.txt")) {
        let flags = match fields[2].as_str() {
            "Mn" => NONSPACING | MARK,
            "Mc" | "Me" => MARK,
            "Cc" | "Cf" => SEPARATOR,
            _ => 0,
        };
        expected[point as usize] |= flags;
    }
    for file in [
        "DerivedCoreProperties.txt",
        "PropList.txt",
        "emoji-data.txt",
        "WordBreakProperty.txt",
        "Scripts.txt",
        "ScriptExtensions.txt",
    ] {
        for line in read(file).lines() {
            let Some((range, property)) =
                line.split('#').next().unwrap_or_default().split_once(';')
            else {
                continue;
            };
            let property = property.trim();
            let flag = match (file, property) {
                ("DerivedCoreProperties.txt", "Default_Ignorable_Code_Point") => IGNORABLE,
                ("PropList.txt", "Bidi_Control") => BIDI,
                ("PropList.txt", "White_Space") => SEPARATOR,
                ("emoji-data.txt", "Extended_Pictographic") => PICTOGRAPHIC,
                ("WordBreakProperty.txt", "Extend") => EXTEND,
                ("Scripts.txt", _) => scripts
                    .iter()
                    .position(|(name, _, _)| *name == property)
                    .map_or(0, |at| ((at + 1) as u32) << 8),
                ("ScriptExtensions.txt", _) => {
                    property.split_whitespace().fold(0, |bits, alias| {
                        bits | scripts
                            .iter()
                            .position(|(_, short, _)| *short == alias)
                            .map_or(0, |at| 1 << (at + 16))
                    })
                }
                _ => 0,
            };
            if flag == 0 {
                continue;
            }
            let (start, end) = range.trim().split_once("..").map_or_else(
                || (code_point(range), code_point(range)),
                |(start, end)| (code_point(start), code_point(end)),
            );
            for point in start..=end {
                expected[point as usize] |= flag;
            }
        }
    }
    for (point, &flags) in expected.iter().enumerate() {
        let Some(c) = char::from_u32(point as u32) else {
            continue;
        };
        for (actual, bit) in [
            (unicode::is_nonspacing_mark(c), NONSPACING),
            (unicode::is_combining_mark(c), MARK),
            (unicode::is_default_ignorable(c), IGNORABLE),
            (unicode::is_bidi_control(c), BIDI),
            (unicode::is_extended_pictographic(c), PICTOGRAPHIC),
            (unicode::is_word_extend(c), EXTEND),
        ] {
            assert_eq!(actual, flags & bit != 0, "U+{point:04X}, bit {bit}");
        }
        let script = (flags >> 8) & 0xF;
        assert_eq!(
            unicode::segmentation_script(c),
            script.checked_sub(1).map(|index| scripts[index as usize].2),
            "Script of U+{point:04X}"
        );
        for (at, &(_, _, value)) in scripts.iter().enumerate() {
            assert_eq!(
                unicode::has_segmentation_script(c, value),
                script == (at + 1) as u32 || flags & (1 << (at + 16)) != 0,
                "Script/Script_Extensions {value:?} of U+{point:04X}"
            );
        }
    }
}

#[test]
fn transparent_controls_retain_full_token_coverage_and_punctuation_barriers() {
    let dictionary = dictionary(&["中文", "rdf:type", "ground.logic", "RDF1.2模型", "can't"]);
    for input in ["中\u{200c}文", "中\u{ad}文", "中\u{fe00}文", "中\u{200d}文"] {
        assert_eq!(dictionary.segments(input), [input]);
    }
    assert_eq!(dictionary.segments("rdf:type"), ["rdf", "type"]);
    assert_eq!(dictionary.segments("ground.logic"), ["ground", "logic"]);
    assert_eq!(
        dictionary.segments("rdf\u{ad}:\u{200c}type"),
        ["rdf\u{ad}", "type"]
    );
    assert_eq!(dictionary.segments("rdf1.2模型"), ["rdf1.2模型"]);
    assert_eq!(dictionary.segments("can't"), ["can't"]);
    assert_eq!(dictionary.segments("r\u{301}.t"), ["r\u{301}", "t"]);
}

#[test]
fn ranked_minimal_automaton_preserves_costs_and_all_prefix_matches() {
    use purrdf_text::segment::DictionaryRepresentation;
    let entries = [
        "a", "ab", "abc", "abd", "b", "bc", "bcd", "cb", "cbc", "中", "中文", "文", "文中",
    ];
    let radix = Dictionary::with_costs(
        entries
            .iter()
            .enumerate()
            .map(|(at, word)| ((*word).to_owned(), (at * 13) as u32)),
    )
    .expect("vocabulary");
    let minimal = radix
        .clone()
        .with_representation(DictionaryRepresentation::MinimalAcyclic);
    assert_eq!(radix.fingerprint(), minimal.fingerprint());
    assert_eq!(radix.artifact_bytes(), minimal.artifact_bytes());
    for (word, cost) in radix.weighted_entries() {
        assert_eq!(minimal.cost(word), Some(cost), "{word}");
    }
    for word in ["", "ac", "abcdef", "c", "中字", "文中文中", "abdbcdcbc中文"] {
        assert_eq!(radix.cost(word), minimal.cost(word));
        assert_eq!(radix.segments(word), minimal.segments(word));
    }
    let shared = dictionary(&["abc", "xbc", "ybc", "zbc"])
        .with_representation(DictionaryRepresentation::MinimalAcyclic);
    assert_eq!(
        shared.storage_stats().states,
        4,
        "equivalent suffix languages share one state regardless of prefix"
    );
}

#[test]
fn artifacts_validate_physical_semantic_canonical_and_resource_contracts() {
    use purrdf_hash::blake3;
    use purrdf_lex::cbor::{self, Limits, Value};
    let dictionary =
        Dictionary::with_costs([("中文".to_owned(), 0), ("rdf模型".to_owned(), u32::MAX)])
            .expect("dictionary");
    let bytes = dictionary.artifact_bytes();
    let hash = |bytes: &[u8]| *blake3::hash(bytes).as_bytes();
    assert_eq!(
        Dictionary::from_artifact(hash(&bytes), &bytes).expect("round trip"),
        dictionary
    );
    assert!(Dictionary::from_artifact([0; 32], &bytes).is_err());
    for length in 0..bytes.len() {
        assert!(
            Dictionary::from_artifact(hash(&bytes[..length]), &bytes[..length]).is_err(),
            "truncated at {length}"
        );
    }
    let mut extra = bytes.clone();
    extra.push(0);
    assert!(Dictionary::from_artifact(hash(&extra), &extra).is_err());
    let mut long = vec![0x98, 6];
    long.extend_from_slice(&bytes[1..]);
    assert!(
        Dictionary::from_artifact(hash(&long), &long).is_err(),
        "nonminimal CBOR head"
    );
    for (field, replacement) in [
        (0, Value::from("wrong-law")),
        (1, Value::Bytes(vec![16, 0, 0])),
        (2, Value::from("not the arena")),
        (
            3,
            Value::Array(vec![
                Value::from(0u32),
                Value::from(1u32),
                Value::from(1u32),
            ]),
        ),
        (4, Value::Array(vec![Value::from(0u32)])),
        (5, Value::Bytes(vec![0; 32])),
    ] {
        let mut malformed = cbor::decode(&bytes, Limits::DEFAULT).expect("valid CBOR");
        malformed.as_array_mut().expect("array")[field] = replacement;
        let altered = cbor::canonical(&malformed);
        assert!(
            Dictionary::from_artifact(hash(&altered), &altered).is_err(),
            "field {field}"
        );
    }
    let empty = Dictionary::empty();
    assert_eq!(
        Dictionary::from_artifact(empty.artifact_fingerprint(), &empty.artifact_bytes())
            .expect("empty artifact"),
        empty
    );
}

#[test]
fn unknown_units_obey_extended_graphemes_not_combining_mark_heuristics() {
    let empty = Dictionary::empty();
    // Myanmar virama conjunct and Khmer coeng consonant are indivisible EGCs.
    for input in ["က္က", "ក្ក", "က\u{200d}ိ"] {
        assert_eq!(
            unicode::grapheme_bounds(input).count(),
            1,
            "fixture must be one Unicode EGC"
        );
        assert_eq!(empty.segments(input), [input]);
    }
}

#[test]
fn all_five_full_baseline_artifacts_preserve_every_canonical_key_and_cost() {
    use purrdf_hash::hex;
    use purrdf_lex::json;
    use purrdf_text::segment::DictionaryRepresentation;
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("lexicons/artifacts");
    let manifest = json::read(
        &std::fs::read_to_string(root.join("manifest.json")).expect("generated manifest"),
    )
    .expect("valid manifest");
    let records = manifest["artifacts"].as_array().expect("artifact array");
    assert_eq!(records.len(), 5);
    for record in records {
        let bytes = std::fs::read(root.join(record["artifact"].as_str().expect("filename")))
            .expect("artifact bytes");
        let identity =
            hex::decode_32(record["physical_blake3"].as_str().expect("identity")).expect("hash");
        let radix = Dictionary::from_artifact(identity, &bytes).expect("verified full artifact");
        let minimal = radix
            .clone()
            .with_representation(DictionaryRepresentation::MinimalAcyclic);
        assert!(radix.len() > 20_000);
        for (word, cost) in radix.weighted_entries() {
            assert_eq!(radix.cost(word), Some(cost), "radix {word}");
            assert_eq!(minimal.cost(word), Some(cost), "minimal {word}");
        }
        for word in radix.words().step_by(211) {
            assert_eq!(radix.segments(word), minimal.segments(word), "{word}");
        }
        match record["name"].as_str().expect("name") {
            "cjdict" => {
                assert!(radix.cost("々宮").is_some());
                assert!(radix.cost("〆切").is_some());
            }
            "burmesedict" => {
                assert!(radix.cost("၏").is_some());
                assert!(radix.cost("၍").is_some());
            }
            _ => {}
        }
    }
}

#[test]
fn surface_dictionary_edges_retain_internal_periods_and_colons() {
    let dictionary = Dictionary::with_costs([
        ("ground.logic.ttl".to_owned(), 0),
        ("rdf:type".to_owned(), 0),
        ("ground".to_owned(), 1),
        ("logic".to_owned(), 1),
        ("ttl".to_owned(), 1),
        ("rdf".to_owned(), 1),
        ("type".to_owned(), 1),
    ])
    .expect("costed punctuation vocabulary");
    let mut scratch = SegmentationScratch::default();
    let mut surface = Vec::new();
    dictionary.segment_surface_each_with_scratch(
        "ground.logic.ttl rdf:type",
        &mut scratch,
        |term| surface.push(term),
    );
    assert_eq!(surface, ["ground.logic.ttl", "rdf:type"]);
    assert_eq!(
        dictionary.segments("ground.logic.ttl rdf:type"),
        ["ground", "logic", "ttl", "rdf", "type"]
    );
    surface.clear();
    dictionary.segment_surface_each_with_scratch("rdf:\u{200c}type", &mut scratch, |term| {
        surface.push(term);
    });
    assert_eq!(surface, ["rdf:\u{200c}type"]);
}
