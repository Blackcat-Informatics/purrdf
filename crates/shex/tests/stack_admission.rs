// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Physical refusal and recovery through the actual public parser entry points.

use purrdf_shex::{ShexError, parse_shexc, parse_shexj};

#[test]
fn parser_entry_points_refuse_stack_exhaustion_and_recover() {
    purrdf_stack::on_stack(1024 * 1024, || {
        let compact = "<http://example.org/S> { <http://example.org/p> . }";
        let json = r#"{"type":"Schema","shapes":[{"type":"Shape","id":"http://example.org/S"}]}"#;
        let reserve = purrdf_stack::reserve(purrdf_stack::remaining());
        for (result, expected) in [
            (parse_shexc(compact, None), "ShExC document"),
            (parse_shexj(json, None), "ShExJ document"),
        ] {
            let error = result.expect_err("exhaustion cannot produce a partial schema");
            assert!(matches!(error, ShexError::StackMargin { construct } if construct == expected));
            assert_eq!(error.byte_offset(), None);
            assert!(
                error
                    .to_string()
                    .contains("physical stack margin exhausted")
            );
        }
        drop(reserve);
        assert_eq!(parse_shexc(compact, None).unwrap().shapes.len(), 1);
        assert_eq!(parse_shexj(json, None).unwrap().shapes.len(), 1);
    })
    .unwrap();
}

#[test]
fn deep_compact_expressions_are_bounded_on_a_small_stack() {
    purrdf_stack::on_stack(256 * 1024, || {
        let source = format!(
            "<http://example.org/S> {}.{}",
            "(".repeat(4000),
            ")".repeat(4000)
        );
        let error = parse_shexc(&source, None).unwrap_err();
        match error {
            ShexError::StackMargin { .. } => {}
            ShexError::Syntax { reason, .. } => {
                assert_eq!(reason, "expression nesting too deep");
            }
            other => panic!("the deep valid expression has an unrelated error: {other}"),
        }
        // The failure is scoped to this attempt, not a poisoned parser context.
        assert!(parse_shexc("<http://example.org/T> .", None).is_ok());
    })
    .unwrap();
}
