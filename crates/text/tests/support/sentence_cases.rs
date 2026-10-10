// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Pinned byte answers exercised through the public API natively and on WASM.

use purrdf_text::unicode;

pub(crate) fn the_sentence_boundaries_are_reproduced_on_this_target() {
    let cases: &[(&str, &[&str])] = &[
        ("", &[]),
        ("\r\n", &["\r\n"]),
        ("\r\u{308}\n", &["\r", "\u{308}\n"]),
        ("a.\u{308}B", &["a.\u{308}B"]),
        ("a.2 U.S.", &["a.2 U.S."]),
        ("A. \u{308}文", &["A. \u{308}", "文"]),
        (
            "。！？\r\nÉté. 東京。\u{2029}👩‍💻!",
            &["。！？\r\n", "Été. ", "東京。\u{2029}", "👩‍💻!"],
        ),
        (
            "A!\u{2060}) \u{2029}\u{308}B",
            &["A!\u{2060}) \u{2029}", "\u{308}B"],
        ),
        ("etc.)’ ‘the rest", &["etc.)’ ‘the rest"]),
    ];
    assert_eq!(unicode::UNICODE_VERSION, (17, 0, 0));
    assert_eq!(
        unicode::SENTENCE_BOUNDARY_LAW,
        "uax29-default-sentence/revision-47"
    );
    for &(input, expected) in cases {
        let actual = unicode::sentence_bounds(input).collect::<Vec<_>>();
        assert_eq!(actual, expected, "{input:?}");
        let mut offset = 0;
        for segment in actual {
            assert_eq!(&input[offset..offset + segment.len()], segment);
            assert!(std::ptr::eq(segment.as_ptr(), input[offset..].as_ptr()));
            offset += segment.len();
        }
        assert_eq!(offset, input.len(), "every original byte is retained");
    }
    let input = "。！？\r\nÉté. 東京。\u{2029}👩‍💻!";
    assert_eq!(
        unicode::sentence_indices(input).collect::<Vec<_>>(),
        [(11, "Été. "), (18, "東京。\u{2029}")],
        "filter membership is pinned alphanumeric; offsets are UTF-8 bytes"
    );
    let mut empty = unicode::sentence_bounds("");
    assert_eq!(empty.next(), None);
    assert_eq!(empty.clone().next(), None);
    assert_eq!(empty.next(), None);
}
