// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Source-preserving lexical admission and native scalar editing boundaries.

use purrdf_lex::yaml::{
    self, Attachment, CollectionStyle, EmitError, EmitLimits, EmitStyle, ErrorKind, NodeKind,
    ScalarContext, ScalarStyle, SyntaxLimits, TriviaKind,
};

#[test]
fn lexical_sample_retains_spelling_comments_styles_and_original_coordinates() {
    let source = "# retained notice\r\ntitle: 'quoted é' # retained tail\r\n\r\nvalues: [0x1F, 007, +007, 1e400, 0x100000000000000000000000000000001, true, null, ~, .inf]\r\nnested:\r\n  item: \"escaped\\\\value\"\r\n";
    let doc = yaml::read_syntax(source, SyntaxLimits::DEFAULT).unwrap();
    assert_eq!(doc.source(), source);
    assert_ne!(
        doc.source_id(),
        yaml::read_syntax(&source.replace("\r\n", "\n"), SyntaxLimits::DEFAULT)
            .unwrap()
            .source_id()
    );
    let scalars: Vec<_> = doc
        .nodes()
        .filter_map(|(id, node)| match &node.kind {
            NodeKind::Scalar(s) => Some((id, s)),
            _ => None,
        })
        .collect();
    for spelling in [
        "0x1F",
        "007",
        "+007",
        "1e400",
        "0x100000000000000000000000000000001",
        "true",
        "null",
        "~",
        ".inf",
    ] {
        let (_, scalar) = scalars
            .iter()
            .find(|(_, scalar)| scalar.text == spelling)
            .unwrap();
        assert_eq!(scalar.style, ScalarStyle::Plain);
        assert_eq!(doc.raw(scalar.raw), Some(spelling));
    }
    let (title, scalar) = scalars
        .iter()
        .find(|(_, scalar)| scalar.text == "quoted é")
        .unwrap();
    assert_eq!(scalar.style, ScalarStyle::SingleQuoted);
    assert_eq!(doc.raw(scalar.raw), Some("'quoted é'"));
    let comments: Vec<_> = doc
        .trivia()
        .iter()
        .filter(|t| t.kind == TriviaKind::Comment)
        .collect();
    assert_eq!(comments.len(), 2);
    assert!(matches!(comments[0].attachment, Attachment::Leading(_)));
    assert_eq!(comments[1].attachment, Attachment::Trailing(*title));
    assert!(doc.trivia().iter().any(|t| t.kind == TriviaKind::BlankLine));
    assert!(doc.nodes().any(|(_, node)| matches!(
        node.kind,
        NodeKind::Sequence {
            style: CollectionStyle::Flow,
            ..
        }
    )));
    let at = source.find(" # retained tail").unwrap();
    let position = doc.position(at).unwrap();
    assert_eq!((position.line, position.column), (2, 18));
    assert_eq!(
        doc.to_core(yaml::ProjectionLimits::DEFAULT)
            .unwrap_err()
            .kind(),
        ErrorKind::NonFinite
    );
    assert_eq!(yaml::read("é: x\rb: [").unwrap_err().line(), 2);
}

#[test]
fn native_properties_alias_identity_and_scalar_metadata_need_no_local_grammar() {
    let source = "%YAML 1.2\n%TAG !e! tag:example.org,2026:\n---\nbase: &a !e!record {x: y}\ncopy: *a\n<<: *a\n? explicit\n: |2- # header\n  # body content\n  text\nquoted: \"a\\n\\u00E9\"\nsingle: 'it''s'\n";
    let doc = yaml::read_syntax(source, SyntaxLimits::DEFAULT).unwrap();
    assert_eq!(doc.directives().len(), 2);
    let (target, anchored) = doc.nodes().find(|(_, node)| node.anchor.is_some()).unwrap();
    assert_eq!(doc.raw(anchored.anchor.unwrap()), Some("&a"));
    assert_eq!(doc.raw(anchored.tag.unwrap()), Some("!e!record"));
    let aliases: Vec<_> = doc
        .nodes()
        .filter_map(|(_, node)| match node.kind {
            NodeKind::Alias { target, raw } => Some((target, raw)),
            _ => None,
        })
        .collect();
    assert_eq!(aliases.len(), 2);
    assert!(
        aliases
            .iter()
            .all(|(id, raw)| *id == target && doc.raw(*raw) == Some("*a"))
    );
    assert!(doc.nodes().any(|(id, _)| doc.is_merge_key(id)));
    assert!(doc.nodes().any(|(_, node)| node.explicit_key.is_some()));
    let block = doc
        .nodes()
        .find_map(|(_, node)| match &node.kind {
            NodeKind::Scalar(s) if s.style == ScalarStyle::Literal => Some(s),
            _ => None,
        })
        .unwrap();
    assert_eq!(block.text, "# body content\ntext");
    let header = block.block.unwrap();
    assert_eq!(header.indent_indicator, Some(2));
    assert_eq!(header.content_indent, 2);
    assert_eq!(header.chomping, yaml::Chomping::Strip);
    assert_eq!(
        doc.trivia()
            .iter()
            .filter(|t| t.kind == TriviaKind::BlockHeaderComment)
            .count(),
        1
    );
    assert!(
        !doc.trivia()
            .iter()
            .any(|t| doc.raw(t.span) == Some("# body content"))
    );
    assert_eq!(
        doc.escapes().iter().map(|e| e.decoded).collect::<Vec<_>>(),
        vec![Some('\n'), Some('é'), Some('\'')]
    );
    let block_id = doc.nodes().find(|(_, node)| matches!(&node.kind, NodeKind::Scalar(scalar) if scalar.style == ScalarStyle::Literal)).unwrap().0;
    assert_eq!(
        doc.trivia()
            .iter()
            .find(|trivia| trivia.kind == TriviaKind::BlockHeaderComment)
            .unwrap()
            .attachment,
        Attachment::Trailing(block_id)
    );
    let collections = yaml::read_syntax(
        "empty: [] # empty tail\nvalues: [a, b] # values tail\n",
        SyntaxLimits::DEFAULT,
    )
    .unwrap();
    for comment in collections
        .trivia()
        .iter()
        .filter(|trivia| trivia.kind == TriviaKind::Comment)
    {
        let Attachment::Trailing(id) = comment.attachment else {
            panic!("collection comment has no trailing owner");
        };
        assert!(matches!(
            collections.node(id).unwrap().kind,
            NodeKind::Sequence { .. }
        ));
    }
    let propertied_key =
        yaml::read_syntax("&mapping\n&key [a, b]: value\n", SyntaxLimits::DEFAULT).unwrap();
    let mapping = propertied_key.node(propertied_key.root()).unwrap();
    assert_eq!(
        propertied_key.raw(mapping.anchor.unwrap()),
        Some("&mapping")
    );
    let NodeKind::Mapping { entries, .. } = &mapping.kind else {
        panic!("a mapping");
    };
    assert_eq!(
        propertied_key.raw(propertied_key.node(entries[0].key).unwrap().anchor.unwrap()),
        Some("&key")
    );
    let boundaries = yaml::read_syntax(
        "a: value   # tail\nflow: [value   , next]\nouter:\n  a: |\n    text\n  b: y\n",
        SyntaxLimits::DEFAULT,
    )
    .unwrap();
    for (_, node) in boundaries.nodes() {
        if let NodeKind::Scalar(scalar) = &node.kind {
            if scalar.text == "value" {
                assert_eq!(boundaries.raw(scalar.raw), Some("value"));
            }
            if scalar.style == ScalarStyle::Literal {
                assert_eq!(boundaries.raw(scalar.raw), Some("|\n    text\n"));
            }
        }
    }
    let complex = yaml::read_syntax("? [a, b]\n: value\n", SyntaxLimits::DEFAULT).unwrap();
    assert!(matches!(
        complex.node(complex.root()).unwrap().kind,
        NodeKind::Mapping { .. }
    ));
}

#[test]
fn malformed_duplicate_alias_and_resource_refusals_have_valid_neighbors() {
    for bad in [
        "a: [",
        "a: \"\\q\"",
        "!<unterminated x",
        "!<tag:example.org,%ZZ> value",
        "!e!record value",
        "!local%ZZ value",
        "!<tag:example.org,a>value",
    ] {
        assert!(
            yaml::read_syntax(bad, SyntaxLimits::DEFAULT).is_err(),
            "{bad}"
        );
    }
    assert!(yaml::read_syntax("a: [x]\nb: \"\\n\"\n", SyntaxLimits::DEFAULT).is_ok());
    for valid in [
        "!<tag:example.org,%20> value",
        "!local%20 value",
        "%TAG !e! tag:example.org,2026:\n---\n!e!record value",
    ] {
        assert!(
            yaml::read_syntax(valid, SyntaxLimits::DEFAULT).is_ok(),
            "{valid}"
        );
    }
    assert_eq!(
        yaml::read_syntax("a: x\n'a': y\n", SyntaxLimits::DEFAULT)
            .unwrap_err()
            .kind(),
        ErrorKind::DuplicateKey
    );
    assert!(yaml::read_syntax("a: x\nb: y\n", SyntaxLimits::DEFAULT).is_ok());
    let source = "a: &a key\n*a : value\nempty:\n";
    let doc = yaml::read_syntax(source, SyntaxLimits::DEFAULT).unwrap();
    let count = doc.nodes().count() as u64;
    assert!(
        yaml::read_syntax(
            source,
            SyntaxLimits {
                max_occurrences: count,
                ..SyntaxLimits::DEFAULT
            }
        )
        .is_ok()
    );
    assert_eq!(
        yaml::read_syntax(
            source,
            SyntaxLimits {
                max_occurrences: count - 1,
                ..SyntaxLimits::DEFAULT
            }
        )
        .unwrap_err()
        .kind(),
        ErrorKind::Nodes { limit: count - 1 }
    );
    assert_eq!(
        yaml::read_syntax(
            source,
            SyntaxLimits {
                aliases: false,
                ..SyntaxLimits::DEFAULT
            }
        )
        .unwrap_err()
        .kind(),
        ErrorKind::Alias
    );
    assert!(
        yaml::read_syntax(
            "a: x",
            SyntaxLimits {
                aliases: false,
                ..SyntaxLimits::DEFAULT
            }
        )
        .is_ok()
    );
    assert!(matches!(
        yaml::read_syntax(
            source,
            SyntaxLimits {
                max_input_bytes: source.len() - 1,
                ..SyntaxLimits::DEFAULT
            }
        )
        .unwrap_err()
        .kind(),
        ErrorKind::Input { .. }
    ));
    assert!(
        yaml::read_syntax(
            source,
            SyntaxLimits {
                max_input_bytes: source.len(),
                ..SyntaxLimits::DEFAULT
            }
        )
        .is_ok()
    );
    assert!(matches!(
        yaml::read_syntax(
            "x",
            SyntaxLimits {
                max_allocation_bytes: 0,
                ..SyntaxLimits::DEFAULT
            }
        )
        .unwrap_err()
        .kind(),
        ErrorKind::Allocation { .. }
    ));
    assert!(yaml::read_syntax("x", SyntaxLimits::DEFAULT).is_ok());
    assert_eq!(
        yaml::read_syntax(
            "[[x]]",
            SyntaxLimits {
                max_depth: 1,
                ..SyntaxLimits::DEFAULT
            }
        )
        .unwrap_err()
        .kind(),
        ErrorKind::Depth { limit: 1 }
    );
    assert!(
        yaml::read_syntax(
            "[x]",
            SyntaxLimits {
                max_depth: 1,
                ..SyntaxLimits::DEFAULT
            }
        )
        .is_ok()
    );
}

#[test]
fn scalar_edits_share_native_escaping_report_fallback_and_preadmit_output() {
    let limits = EmitLimits {
        max_output_bytes: 4096,
        max_allocation_bytes: 4096,
    };
    for (text, request, context) in [
        ("true", EmitStyle::Plain, ScalarContext::Flow),
        ("it's", EmitStyle::SingleQuoted, ScalarContext::Key),
        (
            "a\nb\n",
            EmitStyle::Literal,
            ScalarContext::Block {
                parent_indentation: 0,
                indentation: 2,
            },
        ),
        ("a\nb", EmitStyle::SingleQuoted, ScalarContext::Flow),
        (
            "\0é\\\"\r\u{2028}",
            EmitStyle::DoubleQuoted,
            ScalarContext::Key,
        ),
    ] {
        let emitted = yaml::emit_scalar(text, request, context, limits).unwrap();
        let readback = yaml::read(&emitted.text).unwrap();
        assert_eq!(readback.as_str(), Some(text), "{emitted:?}");
        if request == EmitStyle::SingleQuoted && text.contains('\n') {
            assert_eq!(emitted.style, EmitStyle::DoubleQuoted);
        }
        assert!(matches!(
            yaml::emit_scalar(
                text,
                request,
                context,
                EmitLimits {
                    max_output_bytes: emitted.text.len() - 1,
                    ..limits
                }
            ),
            Err(EmitError::Output { .. })
        ));
        assert!(
            yaml::emit_scalar(
                text,
                request,
                context,
                EmitLimits {
                    max_output_bytes: emitted.text.len(),
                    ..limits
                }
            )
            .is_ok()
        );
    }
    assert!(matches!(
        yaml::emit_scalar(
            "x",
            EmitStyle::DoubleQuoted,
            ScalarContext::Flow,
            EmitLimits {
                max_allocation_bytes: 0,
                ..limits
            }
        ),
        Err(EmitError::Allocation { .. })
    ));
    assert!(matches!(
        yaml::emit_scalar(
            "x",
            EmitStyle::Literal,
            ScalarContext::Block {
                parent_indentation: 0,
                indentation: i32::MAX as usize
            },
            EmitLimits {
                max_output_bytes: 8,
                max_allocation_bytes: 8
            }
        ),
        Err(EmitError::Output { limit: 8 })
    ));
    assert_eq!(
        yaml::emit_scalar(
            "x",
            EmitStyle::Literal,
            ScalarContext::Block {
                parent_indentation: 0,
                indentation: 0
            },
            limits
        ),
        Err(EmitError::Indentation)
    );
    let nested_text = " leading\n\n";
    let nested = yaml::emit_scalar(
        nested_text,
        EmitStyle::Literal,
        ScalarContext::Block {
            parent_indentation: 2,
            indentation: 4,
        },
        limits,
    )
    .unwrap();
    assert!(nested.text.starts_with("|2"));
    assert_eq!(
        yaml::read(&format!("outer:\n  value: {}", nested.text)).unwrap()["outer"]["value"]
            .as_str(),
        Some(nested_text)
    );
    assert_eq!(
        yaml::emit_scalar(
            "x",
            EmitStyle::Literal,
            ScalarContext::Block {
                parent_indentation: 4,
                indentation: 4
            },
            limits
        ),
        Err(EmitError::Indentation)
    );
    let key = yaml::emit_scalar(
        &"é".repeat(1022),
        EmitStyle::DoubleQuoted,
        ScalarContext::Key,
        limits,
    )
    .unwrap();
    assert_eq!(key.text.chars().count(), 1024);
    assert_eq!(
        yaml::emit_scalar(
            &"é".repeat(1023),
            EmitStyle::DoubleQuoted,
            ScalarContext::Key,
            limits
        ),
        Err(EmitError::KeyLength { limit: 1024 })
    );
}

#[test]
fn core_projection_retains_eager_refusal_order_and_scalar_key_law() {
    assert_eq!(
        yaml::read("a: 1\na: .inf\n").unwrap_err().kind(),
        ErrorKind::NonFinite
    );
    assert_eq!(
        yaml::read("a: 1\na: 2\n").unwrap_err().kind(),
        ErrorKind::DuplicateKey
    );
    let source = "0x1F: yes\ntrue: no\n";
    let lexical = yaml::read_syntax(source, SyntaxLimits::DEFAULT).unwrap();
    assert_eq!(
        lexical
            .to_core(yaml::ProjectionLimits::DEFAULT)
            .unwrap_err()
            .kind(),
        ErrorKind::NonStringKey
    );
    let core = lexical
        .to_core(yaml::ProjectionLimits {
            core: yaml::Limits {
                scalar_keys: true,
                ..yaml::Limits::DEFAULT
            },
            ..yaml::ProjectionLimits::DEFAULT
        })
        .unwrap();
    assert_eq!(core["0x1F"], "yes");
    assert_eq!(core["true"], "no");
    let expanded_source = format!("base: &a ['{}']\ncopy: [*a, *a, *a]\n", "x".repeat(4096));
    let expanded = yaml::read_syntax(&expanded_source, SyntaxLimits::DEFAULT).unwrap();
    let limits = yaml::ProjectionLimits {
        max_allocation_bytes: 32 << 10,
        ..yaml::ProjectionLimits::DEFAULT
    };
    assert!(expanded.to_core(limits).is_ok());
    let refused = expanded
        .to_core(yaml::ProjectionLimits {
            max_allocation_bytes: 16 << 10,
            ..limits
        })
        .unwrap_err();
    assert!(matches!(refused.kind(), ErrorKind::Allocation { .. }));
    assert!(refused.offset() >= expanded_source.find("copy:").unwrap());
    let node_limit = yaml::ProjectionLimits {
        core: yaml::Limits {
            max_nodes: 10,
            ..yaml::Limits::DEFAULT
        },
        ..limits
    };
    let refused = expanded.to_core(node_limit).unwrap_err();
    assert_eq!(refused.kind(), ErrorKind::Nodes { limit: 10 });
    assert_eq!(refused.offset(), expanded_source.rfind("*a").unwrap());
    assert!(
        expanded
            .to_core(yaml::ProjectionLimits {
                core: yaml::Limits {
                    max_nodes: 12,
                    ..yaml::Limits::DEFAULT
                },
                ..limits
            })
            .is_ok()
    );
    assert!(matches!(
        expanded
            .to_core(yaml::ProjectionLimits {
                max_input_bytes: expanded_source.len() - 1,
                ..limits
            })
            .unwrap_err()
            .kind(),
        ErrorKind::Input { .. }
    ));
}
