// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The physical-line authoring contract stays distinct from structural slicing.
use purrdf_markdown::syntax::AuthoringLinkForm;
use purrdf_markdown::{SyntaxError, SyntaxLimits, authoring, syntax};

fn limits() -> SyntaxLimits {
    SyntaxLimits {
        source_bytes: 1 << 20,
        auxiliary_bytes: 64 << 20,
        work: 128 << 20,
        containers: 64,
        nesting: 1 << 20,
        link_part_bytes: 4096,
    }
}

#[test]
fn physical_authoring_reads_indent_html_and_line_local_code() {
    let source = "    [indent](i)\r\n<div>\r\n## Title\r\n[a](a)\r\n</div>\r\n`open\r\n[b](b)\r\nclose`\r\n`[hidden](x)` [c](c)\r\n> ## Quoted\r\nsetext\r\n---\r\n> ```\r\n[hidden](x)\r\n```\r\n[d](d)\r\n";
    let doc = authoring(source, limits()).unwrap();
    assert_eq!(
        doc.links()
            .iter()
            .map(|l| l.destination.decoded.as_ref())
            .collect::<Vec<_>>(),
        ["i", "a", "b", "c", "d"]
    );
    assert_eq!(
        doc.headings()
            .iter()
            .map(|h| h.text.as_ref())
            .collect::<Vec<_>>(),
        ["Title"]
    );
    assert_eq!(doc.lines()[0].prose(), "    [indent](i)");
    assert_eq!(doc.spelling(doc.lines()[0].newline), "\r\n");
    assert_eq!(doc.lines()[8].prose(), "  [c](c)");
    assert_eq!(doc.code_spans().len(), 1);
    let bom_heading = authoring("\u{feff}  # Title\r\n\u{feff}# ordinary\n", limits()).unwrap();
    assert_eq!(bom_heading.headings().len(), 1);
    assert_eq!(bom_heading.headings()[0].text, "Title");
    assert_eq!(bom_heading.headings()[0].title.start, 7);
    assert_eq!(
        bom_heading.spelling(bom_heading.lines()[0].source),
        "\u{feff}  # Title"
    );
    assert_eq!(bom_heading.lines()[1].prose(), "\u{feff}# ordinary");
    let bom_fence = authoring("\u{feff}```rust\n[hidden](x)\n```\n[live](y)", limits()).unwrap();
    assert_eq!(bom_fence.links().len(), 1);
    assert_eq!(bom_fence.links()[0].destination.decoded, "y");
    assert_eq!(bom_fence.lines()[0].source.start, 0);
    let fence = &bom_fence.blocks()[0];
    assert!(
        matches!(fence.kind, syntax::BlockKind::FencedCode { marker, closed: true, .. } if marker.start == 3)
    );
    let bom_definition = authoring("\u{feff}[label]: target", limits()).unwrap();
    assert_eq!(bom_definition.links()[0].source.start, 3);
    assert_eq!(bom_definition.links()[0].destination.decoded, "target");
    let listed = authoring("- item\n    ```\n[hidden](x)\n    ```\n[live](y)", limits()).unwrap();
    assert_eq!(
        listed
            .links()
            .iter()
            .map(|link| link.destination.decoded.as_ref())
            .collect::<Vec<_>>(),
        ["y"]
    );
    let hidden_prefix = format!("```\n{}```\n[hidden](x)\n```\n[live](y)", ">".repeat(66));
    let mut tight = limits();
    tight.containers = 1;
    assert_eq!(
        authoring(&hidden_prefix, tight).unwrap().links()[0]
            .destination
            .decoded,
        "y"
    );
    let bare = authoring("`hidden\r[a](x)` [b](<a\rb>) <a href='x\ry'>", limits()).unwrap();
    assert_eq!(bare.lines().len(), 1);
    assert_eq!(
        bare.links()
            .iter()
            .map(|link| link.destination.decoded.as_ref())
            .collect::<Vec<_>>(),
        ["a\rb", "x\ry"]
    );
    assert!(bare.split_links().is_empty());
    assert_eq!(
        bare.position(bare.source().find("[b]").unwrap())
            .unwrap()
            .line,
        1
    );
    let arena = authoring("> - item\n  continuation", limits()).unwrap();
    for block in arena.blocks() {
        if let Some(parent) = block.parent {
            assert!(arena.blocks()[parent].source.contains_span(block.source));
        }
    }
    let standard = syntax(source, limits()).unwrap();
    assert!(
        !standard
            .links()
            .iter()
            .any(|l| l.destination.decoded == "i")
    );
}

#[test]
fn comment_holes_code_replacement_and_split_links_keep_original_extents() {
    let source = "## a<!--hide-->b `code` ##\r\n[x](a<!--hide-->b)\r\n[y](<`z`>)\r\n[a\r\n<!--c-->\r\nb](split)\r\n<!-->\r\n```\r\n-->\r\n```\r\n[hidden](x)\r\n-->[live](l)\r\n<!--unfinished";
    let doc = authoring(source, limits()).unwrap();
    assert_eq!(doc.headings()[0].text, "ab `code`");
    assert_eq!(
        doc.spelling(doc.headings()[0].title),
        "a<!--hide-->b `code`"
    );
    assert_eq!(
        doc.links()
            .iter()
            .map(|l| l.destination.decoded.as_ref())
            .collect::<Vec<_>>(),
        ["ab", " ", "l"]
    );
    assert_eq!(
        doc.spelling(doc.links()[0].destination.source),
        "a<!--hide-->b"
    );
    assert_eq!(doc.spelling(doc.links()[1].destination.source), "`z`");
    assert_eq!(doc.split_links().len(), 1);
    assert_eq!(doc.split_links()[0].destination.decoded, "split");
    assert_eq!(
        doc.spelling(doc.split_links()[0].source),
        "[a\r\n<!--c-->\r\nb](split)"
    );
    assert!(doc.comments()[3].closed);
    assert!(
        doc.spelling(doc.comments()[3].source)
            .contains("[hidden](x)")
    );
    assert!(!doc.comments().last().unwrap().closed);
    assert_eq!(
        doc.comments().last().unwrap().source.end as usize,
        source.len()
    );
    for link in doc.links().iter().chain(doc.split_links()) {
        assert!(link.source.end as usize <= source.len());
        assert!(source.is_char_boundary(link.destination.source.start as usize));
        assert!(source.is_char_boundary(link.destination.source.end as usize));
    }
    for source in [
        "``unmatched` [live](x)",
        "\\`` [live](x)`",
        "<!---> [hidden](x)",
    ] {
        let doc = authoring(source, limits()).unwrap();
        assert_eq!(doc.links().len(), usize::from(!source.starts_with("<!--")));
    }
}

#[test]
fn definitions_entities_and_tolerant_html_keep_the_authoring_link_policy() {
    let source = "[use][ref]\n[ref]: a\\(b\\)&amp; \"title &amp;\"\n[^foot]: hidden\n<a HREF='a&amp;b' src=x/> [inside](i)\n<?pi [inside](p)?>\n<a href=\nnext>\n<a href='cross\nline' src='ok'>\n<a href=unfinished\n<a éname='ignored' href='u'>\n<a href\n= excluded>\n<a other=\nhref=nextwanted>";
    let doc = authoring(source, limits()).unwrap();
    assert_eq!(
        doc.links()
            .iter()
            .map(|l| (l.form, l.destination.decoded.as_ref()))
            .collect::<Vec<_>>(),
        [
            (AuthoringLinkForm::ReferenceDefinition, "a(b)&amp;"),
            (AuthoringLinkForm::Href, "a&amp;b"),
            (AuthoringLinkForm::Src, "x"),
            (AuthoringLinkForm::Inline, "i"),
            (AuthoringLinkForm::Inline, "p"),
            (AuthoringLinkForm::Href, "next"),
            (AuthoringLinkForm::Src, "ok"),
            (AuthoringLinkForm::Href, "unfinished"),
            (AuthoringLinkForm::Href, "u"),
            (AuthoringLinkForm::Href, "nextwanted"),
        ]
    );
    assert_eq!(
        doc.links()[0].title.as_ref().unwrap().decoded,
        "title &amp;"
    );
    assert_eq!(doc.spelling(doc.links()[2].destination.source), "x/");
    let long_label = format!("[{}]: target", "a".repeat(1000));
    assert_eq!(
        authoring(&long_label, limits()).unwrap().links()[0]
            .destination
            .decoded,
        "target"
    );
}

#[test]
fn authoring_refuses_original_resources_at_their_exact_boundaries() {
    for (component, suffix) in [("[x](", ")"), ("[x](d '", "')"), ("<a href='", "'>")] {
        let admitted = format!("{component}{}{suffix}", "a".repeat(4096));
        assert_eq!(authoring(&admitted, limits()).unwrap().links().len(), 1);
        let refused = format!("{component}{}{suffix}", "a".repeat(4097));
        assert!(matches!(
            authoring(&refused, limits()),
            Err(SyntaxError::Limit {
                resource: "link part bytes",
                ..
            })
        ));
    }
    let nested = format!("    {}text", "> ".repeat(64));
    assert!(authoring(&nested, limits()).is_ok());
    let nested = format!("    {}text", "> ".repeat(65));
    assert!(matches!(
        authoring(&nested, limits()),
        Err(SyntaxError::Limit {
            resource: "containers",
            actual: 65,
            ..
        })
    ));
    let inline = format!("{}x{}(d)", "[".repeat(65), "]".repeat(65));
    assert_eq!(authoring(&inline, limits()).unwrap().links().len(), 1);
    for (resource, bound) in [("source bytes", 0), ("auxiliary bytes", 1), ("work", 2)] {
        let mut policy = limits();
        match bound {
            0 => policy.source_bytes = 0,
            1 => policy.auxiliary_bytes = 0,
            _ => policy.work = 0,
        }
        assert!(
            matches!(authoring("α [a](b)", policy), Err(SyntaxError::Limit { resource: found, position, .. }) if found == resource && position.byte == 0)
        );
    }
}
