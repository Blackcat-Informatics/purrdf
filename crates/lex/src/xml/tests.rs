// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use std::fmt::Write as _;

use super::{
    Document, Dtd, Event, NodeType, Options, Reader, XML_NAMESPACE, XmlError, XmlErrorKind,
};

fn parse(text: &str) -> Result<Document<'_>, XmlError> {
    Document::parse(text)
}

fn parse_dtd(text: &str) -> Result<Document<'_>, XmlError> {
    Document::parse_with_options(
        text,
        Options {
            dtd: Dtd::internal_subset(),
            ..Options::default()
        },
    )
}

fn kind(result: Result<Document<'_>, XmlError>) -> XmlErrorKind {
    result.expect_err("the document is refused").kind().clone()
}

/// `depth` nested `<e>` elements.
fn nested(depth: usize) -> String {
    let mut text = "<e>".repeat(depth);
    text.push_str(&"</e>".repeat(depth));
    text
}

fn with_depth(depth: usize) -> Options {
    Options {
        max_depth: depth,
        ..Options::default()
    }
}

// ── The contract's paired refusals ─────────────────────────────────────────

#[test]
fn a_doctype_is_refused() {
    let text = "<!DOCTYPE r><r/>";
    assert_eq!(kind(parse(text)), XmlErrorKind::Doctype);
    assert_eq!(parse(text).unwrap_err().offset(), 0);
}

#[test]
fn a_namespaced_document_with_cdata_and_a_character_reference_parses() {
    let text = "<p:r xmlns:p=\"http://example.org/p\"><p:c>x<![CDATA[<y>]]>&#x7A;</p:c></p:r>";
    let document = parse(text).expect("well-formed");
    let root = document.root_element();
    assert_eq!(root.tag_name().namespace(), Some("http://example.org/p"));
    assert_eq!(root.tag_name().name(), "r");
    let child = root.first_element_child().expect("a child");
    assert_eq!(child.text(), Some("x<y>z"));
    assert_eq!(child.prefix(), Some("p"));
}

#[test]
fn the_depth_cap_refuses_one_past_the_cap_and_accepts_the_cap() {
    let cap = 7;
    let (over, at) = (nested(cap + 1), nested(cap));
    let refused = Document::parse_with_options(&over, with_depth(cap));
    assert_eq!(kind(refused), XmlErrorKind::DepthLimit { limit: cap });
    let accepted = Document::parse_with_options(&at, with_depth(cap)).expect("at cap");
    assert_eq!(
        accepted
            .descendants()
            .filter(super::Node::is_element)
            .count(),
        cap
    );
    // An empty-element tag is an element at its depth too.
    let mut text = "<e>".repeat(cap);
    text.push_str("<leaf/>");
    text.push_str(&"</e>".repeat(cap));
    assert_eq!(
        kind(Document::parse_with_options(&text, with_depth(cap))),
        XmlErrorKind::DepthLimit { limit: cap }
    );
}

#[test]
fn the_default_depth_cap_is_the_published_envelope() {
    assert!(parse(&nested(super::DEFAULT_MAX_DEPTH)).is_ok());
    assert_eq!(
        kind(parse(&nested(super::DEFAULT_MAX_DEPTH + 1))),
        XmlErrorKind::DepthLimit {
            limit: super::DEFAULT_MAX_DEPTH
        }
    );
}

#[test]
fn a_deep_document_is_read_without_recursion() {
    let depth = 200_000;
    let text = nested(depth);
    let document = Document::parse_with_options(&text, with_depth(depth)).expect("deep but lawful");
    assert_eq!(document.len(), depth + 1);
    let deepest = document.descendants().last().expect("a node");
    assert_eq!(deepest.ancestors().count(), depth + 1);
}

#[test]
fn a_signed_character_reference_is_refused_and_an_unsigned_one_is_read() {
    assert_eq!(kind(parse("<r>&#x+41;</r>")), XmlErrorKind::InvalidCharRef);
    assert_eq!(
        kind(parse("<r a='&#x+41;'/>")),
        XmlErrorKind::InvalidCharRef
    );
    let document = parse("<r a='&#x41;'>&#x41;</r>").expect("unsigned");
    assert_eq!(document.root_element().text(), Some("A"));
    assert_eq!(document.root_element().attribute("a"), Some("A"));
}

// ── Characters ─────────────────────────────────────────────────────────────

#[test]
fn a_character_outside_char_is_refused_and_its_neighbours_are_read() {
    assert_eq!(
        kind(parse("<r>\u{1}</r>")),
        XmlErrorKind::IllegalChar('\u{1}')
    );
    assert_eq!(
        kind(parse("<r>\u{ffff}</r>")),
        XmlErrorKind::IllegalChar('\u{ffff}')
    );
    assert_eq!(
        kind(parse("<r a='\u{fffe}'/>")),
        XmlErrorKind::IllegalChar('\u{fffe}')
    );
    assert_eq!(
        kind(parse("<r><!--\u{8}--></r>")),
        XmlErrorKind::IllegalChar('\u{8}')
    );
    assert_eq!(kind(parse("<r>&#0;</r>")), XmlErrorKind::InvalidCharRef);
    let document = parse("<r a='\t\u{fffd}'>\t\u{fffd}\u{f000}\u{10000}</r>").expect("Chars");
    assert_eq!(
        document.root_element().text(),
        Some("\t\u{fffd}\u{f000}\u{10000}")
    );
}

#[test]
fn line_ends_are_normalized_in_text_and_attributes() {
    let document = parse("<r a='x\r\ny\rz'>1\r\n2\r3</r>").expect("line ends");
    let root = document.root_element();
    assert_eq!(root.text(), Some("1\n2\n3"));
    // CR LF is one line end, so one SPACE.
    assert_eq!(root.attribute("a"), Some("x y z"));
    // A character reference is not a line end and survives normalization.
    let document = parse("<r a='&#13;&#10;'>&#13;</r>").expect("references");
    assert_eq!(document.root_element().attribute("a"), Some("\r\n"));
    assert_eq!(document.root_element().text(), Some("\r"));
}

#[test]
fn attribute_whitespace_becomes_spaces() {
    let document = parse("<r a=\"\tone\ntwo \"/>").expect("attribute");
    assert_eq!(document.root_element().attribute("a"), Some(" one two "));
}

#[test]
fn a_less_than_in_an_attribute_value_is_refused_and_a_greater_than_is_read() {
    assert_eq!(
        kind(parse("<r a='<'/>")),
        XmlErrorKind::LessThanInAttributeValue
    );
    assert_eq!(
        parse("<r a='>'/>").unwrap().root_element().attribute("a"),
        Some(">")
    );
}

#[test]
fn cdata_end_in_text_is_refused_and_two_brackets_are_read() {
    assert_eq!(kind(parse("<r>a]]>b</r>")), XmlErrorKind::CdataEndInText);
    assert_eq!(
        parse("<r>a]]b]</r>").unwrap().root_element().text(),
        Some("a]]b]")
    );
}

#[test]
fn the_five_predefined_entities_expand_and_an_undeclared_one_is_refused() {
    let document = parse("<r a='&lt;&gt;&amp;&apos;&quot;'>&lt;&gt;&amp;&apos;&quot;</r>")
        .expect("predefined");
    assert_eq!(document.root_element().text(), Some("<>&'\""));
    assert_eq!(document.root_element().attribute("a"), Some("<>&'\""));
    assert_eq!(
        kind(parse("<r>&nbsp;</r>")),
        XmlErrorKind::UndeclaredEntity("nbsp".to_owned())
    );
}

// ── Structure ──────────────────────────────────────────────────────────────

#[test]
fn a_mismatched_end_tag_is_refused_with_its_offset() {
    let error = parse("<a><b></a></b>").unwrap_err();
    assert_eq!(
        error.kind(),
        &XmlErrorKind::MismatchedEndTag {
            expected: "b".to_owned(),
            found: "a".to_owned()
        }
    );
    assert_eq!(error.offset(), 6);
    assert_eq!(error.line_column("<a><b></a></b>"), (1, 7));
    assert!(parse("<a><b></b></a>").is_ok());
}

#[test]
fn line_and_column_count_every_line_end_once() {
    let text = "<a>\r\n<b>\r</c></a>";
    let error = parse(text).unwrap_err();
    assert_eq!(error.line_column(text), (3, 1));
}

#[test]
fn content_outside_the_root_is_refused_and_whitespace_and_misc_are_read() {
    assert_eq!(kind(parse("<a/><b/>")), XmlErrorKind::ContentOutsideRoot);
    assert_eq!(kind(parse("x<a/>")), XmlErrorKind::ContentOutsideRoot);
    assert_eq!(kind(parse("<a/>x")), XmlErrorKind::ContentOutsideRoot);
    assert_eq!(kind(parse("")), XmlErrorKind::NoRootElement);
    assert_eq!(kind(parse("<!-- only -->")), XmlErrorKind::NoRootElement);
    let document = parse("\n<!-- c --><?pi data?>\n<a/>\n<!-- after -->\n").expect("misc");
    let kinds: Vec<NodeType> = document.root().children().map(|n| n.node_type()).collect();
    assert_eq!(
        kinds,
        [
            NodeType::Comment,
            NodeType::Pi,
            NodeType::Element,
            NodeType::Comment
        ]
    );
}

#[test]
fn an_unterminated_element_is_refused() {
    assert_eq!(kind(parse("<a><b></b>")), XmlErrorKind::UnexpectedEof);
    assert_eq!(kind(parse("<a")), XmlErrorKind::UnexpectedEof);
}

#[test]
fn a_double_hyphen_comment_is_refused_and_a_single_hyphen_is_read() {
    assert_eq!(
        kind(parse("<a><!-- a -- b --></a>")),
        XmlErrorKind::InvalidComment
    );
    assert_eq!(
        kind(parse("<a><!-- a ---></a>")),
        XmlErrorKind::InvalidComment
    );
    let document = parse("<a><!-- a - b --></a>").expect("single hyphen");
    assert_eq!(
        document.root_element().first_child().unwrap().text(),
        Some(" a - b ")
    );
}

#[test]
fn the_xml_declaration_opens_the_document_or_is_refused() {
    let document = parse("\u{feff}<?xml version=\"1.0\" encoding='UTF-8' standalone=\"yes\"?><a/>");
    assert!(document.is_ok());
    let mut reader = Reader::new("<?xml version='1.1'?><a/>");
    match reader.next().expect("declaration") {
        Event::Declaration(declaration) => {
            assert_eq!(declaration.version, "1.1");
            assert_eq!(declaration.encoding, None);
            assert_eq!(declaration.standalone, None);
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(
        kind(parse(" <?xml version='1.0'?><a/>")),
        XmlErrorKind::MisplacedXmlDeclaration
    );
    assert_eq!(
        kind(parse("<?xml version='2.0'?><a/>")),
        XmlErrorKind::InvalidXmlDeclaration
    );
    assert_eq!(
        kind(parse("<?xml version='1.0' standalone='maybe'?><a/>")),
        XmlErrorKind::InvalidXmlDeclaration
    );
    assert_eq!(
        kind(parse("<a><?XmL x?></a>")),
        XmlErrorKind::ReservedPiTarget
    );
    let document = parse("<?xml-stylesheet href='s'?><a/>").expect("not the declaration");
    assert_eq!(
        document.root().first_child().unwrap().pi(),
        Some(("xml-stylesheet", Some("href='s'")))
    );
}

#[test]
fn names_are_checked_against_the_name_productions() {
    assert_eq!(kind(parse("<1a/>")), XmlErrorKind::InvalidName);
    assert_eq!(kind(parse("<a:b:c/>")), XmlErrorKind::InvalidName);
    assert_eq!(
        kind(parse("<a: xmlns:a='http://example.org/'/>")),
        XmlErrorKind::InvalidName
    );
    let document = parse("<élément-1.x _a='1'/>").expect("Unicode names");
    assert_eq!(document.root_element().tag_name().name(), "élément-1.x");
}

#[test]
fn duplicate_attributes_are_refused_by_name_and_by_expanded_name() {
    assert_eq!(
        kind(parse("<a x='1' x='2'/>")),
        XmlErrorKind::DuplicateAttribute("x".to_owned())
    );
    let same_namespace =
        "<a xmlns:p='http://example.org/n' xmlns:q='http://example.org/n' p:x='1' q:x='2'/>";
    assert_eq!(
        kind(parse(same_namespace)),
        XmlErrorKind::DuplicateAttribute("q:x".to_owned())
    );
    let distinct = "<a xmlns:p='http://example.org/n' p:x='1' x='2'/>";
    assert!(parse(distinct).is_ok());
    // Many attributes take the sorted path.
    let many: String = (0..40).fold(String::new(), |mut out, i| {
        let _ = write!(out, " a{i}='v'");
        out
    });
    assert!(parse(&format!("<r{many}/>")).is_ok());
    assert_eq!(
        kind(parse(&format!("<r{many} a7='again'/>"))),
        XmlErrorKind::DuplicateAttribute("a7".to_owned())
    );
}

// ── Namespaces ─────────────────────────────────────────────────────────────

#[test]
fn an_undeclared_prefix_is_refused_and_a_declared_one_resolves() {
    assert_eq!(
        kind(parse("<p:a/>")),
        XmlErrorKind::UndeclaredPrefix("p".to_owned())
    );
    assert_eq!(
        kind(parse("<a p:x='1'/>")),
        XmlErrorKind::UndeclaredPrefix("p".to_owned())
    );
    let document =
        parse("<p:a xmlns:p='http://example.org/p' p:x='1' xml:lang='en'/>").expect("declared");
    let root = document.root_element();
    assert_eq!(root.attribute(("http://example.org/p", "x")), Some("1"));
    assert_eq!(root.attribute((XML_NAMESPACE, "lang")), Some("en"));
    // A bare local name matches only an attribute with no namespace.
    assert_eq!(root.attribute("x"), None);
    assert_eq!(root.attributes()[0].qname(), "p:x");
    assert_eq!(root.attributes()[0].prefix(), Some("p"));
}

#[test]
fn an_unprefixed_attribute_has_no_namespace_even_under_a_default() {
    let document = parse("<a xmlns='http://example.org/d' x='1'/>").expect("default ns");
    let root = document.root_element();
    assert_eq!(root.tag_name().namespace(), Some("http://example.org/d"));
    assert_eq!(root.attributes()[0].namespace(), None);
    assert_eq!(root.attribute("x"), Some("1"));
}

#[test]
fn a_prefix_cannot_be_undeclared_but_the_default_namespace_can() {
    assert_eq!(
        kind(parse("<a xmlns:p=''/>")),
        XmlErrorKind::EmptyNamespacePrefix("p".to_owned())
    );
    let document =
        parse("<a xmlns='http://example.org/d'><b xmlns=''><c/></b></a>").expect("undeclared");
    let b = document.root_element().first_element_child().unwrap();
    assert_eq!(b.tag_name().namespace(), None);
    assert_eq!(
        b.first_element_child().unwrap().tag_name().namespace(),
        None
    );
    assert_eq!(b.lookup_namespace_uri(None), None);
    assert_eq!(
        document.root_element().lookup_namespace_uri(None),
        Some("http://example.org/d")
    );
}

#[test]
fn the_reserved_prefixes_are_refused_where_namespaces_forbids_them() {
    for text in [
        "<a xmlns:xml='http://example.org/'/>",
        "<a xmlns:x='http://www.w3.org/XML/1998/namespace'/>",
        "<a xmlns='http://www.w3.org/XML/1998/namespace'/>",
        "<a xmlns:xmlns='http://example.org/'/>",
        "<a xmlns:x='http://www.w3.org/2000/xmlns/'/>",
        "<xmlns:a/>",
    ] {
        assert!(
            matches!(kind(parse(text)), XmlErrorKind::ReservedNamespace(_)),
            "{text}"
        );
    }
    // Binding `xml` to its own namespace is lawful.
    assert!(parse("<a xmlns:xml='http://www.w3.org/XML/1998/namespace'/>").is_ok());
}

#[test]
fn in_scope_namespaces_list_the_nearest_binding_first_then_xml() {
    let text = "<a xmlns='http://example.org/d' xmlns:p='http://example.org/p1'>\
                <b xmlns:p='http://example.org/p2' xmlns:q='http://example.org/q'/></a>";
    let document = parse(text).expect("namespaces");
    let b = document.root_element().first_element_child().unwrap();
    let scope: Vec<(Option<&str>, &str)> = b.namespaces().map(|n| (n.name(), n.uri())).collect();
    assert_eq!(
        scope,
        [
            (Some("p"), "http://example.org/p2"),
            (Some("q"), "http://example.org/q"),
            (None, "http://example.org/d"),
            (Some("xml"), XML_NAMESPACE),
        ]
    );
    assert_eq!(b.lookup_prefix("http://example.org/p2"), Some("p"));
    // The outer binding of `p` is shadowed here.
    assert_eq!(b.lookup_prefix("http://example.org/p1"), None);
    assert_eq!(b.lookup_prefix("http://example.org/d"), Some(""));
    assert_eq!(b.lookup_prefix(XML_NAMESPACE), Some("xml"));
    assert_eq!(
        b.lookup_namespace_uri(Some("p")),
        Some("http://example.org/p2")
    );
    assert_eq!(b.lookup_namespace_uri(Some("xml")), Some(XML_NAMESPACE));
    assert_eq!(b.namespace_declarations().len(), 2);
    assert_eq!(document.root_element().attributes().len(), 0);
}

// ── The tree ───────────────────────────────────────────────────────────────

#[test]
fn the_tree_links_parents_siblings_and_descendants_in_document_order() {
    let document = parse("<a><b>1</b><!--c--><d><e/></d>2</a>").expect("tree");
    let a = document.root_element();
    let names: Vec<String> = a
        .descendants()
        .map(|n| match n.node_type() {
            NodeType::Element => n.tag_name().name().to_owned(),
            _ => format!("{:?}", n.node_type()),
        })
        .collect();
    assert_eq!(names, ["a", "b", "Text", "Comment", "d", "e", "Text"]);
    let children: Vec<_> = a.children().collect();
    assert_eq!(children.len(), 4);
    let reversed: Vec<_> = a.children().rev().collect();
    assert_eq!(reversed.first(), children.last());
    assert_eq!(children[0].next_sibling(), Some(children[1]));
    assert_eq!(children[1].prev_sibling(), Some(children[0]));
    assert_eq!(children[0].next_sibling_element(), Some(children[2]));
    assert_eq!(children[2].parent(), Some(a));
    assert_eq!(a.parent().map(|p| p.is_root()), Some(true));
    assert_eq!(a.parent_element(), None);
    assert_eq!(children[3].text(), Some("2"));
    assert!(a.has_children());
    assert!(!children[2].first_child().unwrap().has_children());
    assert!(a.has_tag_name("a"));
    assert!(!children[1].has_tag_name("a"));
    let id = children[2].id();
    assert_eq!(document.get_node(id), Some(children[2]));
    assert_eq!(document.root().descendants().count(), document.len());
}

#[test]
fn offsets_point_at_each_node() {
    let text = "<a>\n  <b x='1'>t</b></a>";
    let document = parse(text).expect("offsets");
    let b = document.root_element().first_element_child().unwrap();
    assert_eq!(b.offset(), 6);
    assert_eq!(b.attributes()[0].offset(), 9);
    assert_eq!(b.first_child().unwrap().offset(), 15);
}

#[test]
fn adjacent_text_cdata_and_references_merge_into_one_node() {
    let document = parse("<a>x&amp;<![CDATA[y]]>&#x7A;<![CDATA[]]></a>").expect("merge");
    let a = document.root_element();
    assert_eq!(a.children().count(), 1);
    assert_eq!(a.text(), Some("x&yz"));
}

#[test]
fn the_reader_streams_events_and_ends_every_empty_element() {
    let mut reader = Reader::new("<a x='1'><b/>t</a>");
    let mut seen = Vec::new();
    loop {
        let event = reader.next().expect("event");
        seen.push(match event {
            Event::Start(tag) => format!(
                "start {} {} {}",
                tag.name.as_str(),
                tag.attributes.len(),
                tag.self_closing
            ),
            Event::End(tag) => format!("end {}", tag.name.as_str()),
            Event::Text { text, .. } => format!("text {text}"),
            Event::Eof => break,
            other => format!("{other:?}"),
        });
    }
    assert_eq!(
        seen,
        [
            "start a 1 false",
            "start b 0 true",
            "end b",
            "text t",
            "end a"
        ]
    );
    assert!(matches!(reader.next(), Ok(Event::Eof)));
}

#[test]
fn text_is_borrowed_where_the_source_spells_it() {
    let text = "<a x='plain'>plain</a>";
    let mut reader = Reader::new(text);
    let Ok(Event::Start(tag)) = reader.next() else {
        panic!("start");
    };
    assert!(matches!(tag.attributes[0].value(), "plain"));
    match reader.next() {
        Ok(Event::Text {
            text: std::borrow::Cow::Borrowed(t),
            ..
        }) => assert_eq!(t, "plain"),
        other => panic!("{other:?}"),
    }
}

// ── The internal subset ────────────────────────────────────────────────────

#[test]
fn internal_entities_expand_in_text_and_attribute_values() {
    let text = r#"<!DOCTYPE Document [
        <!ENTITY rif  "http://www.w3.org/2007/rif#">
        <!ENTITY ex   "http://example.org/ns#">
        ]>
<Document xmlns="&rif;"><payload a="&ex;x">&ex;y</payload></Document>"#;
    let document = parse_dtd(text).expect("internal subset");
    let root = document.root_element();
    assert_eq!(
        root.tag_name().namespace(),
        Some("http://www.w3.org/2007/rif#")
    );
    let payload = root.first_element_child().unwrap();
    assert_eq!(payload.attribute("a"), Some("http://example.org/ns#x"));
    assert_eq!(payload.text(), Some("http://example.org/ns#y"));
    // The same document without the opt-in is refused, not misread.
    assert_eq!(kind(parse(text)), XmlErrorKind::Doctype);
}

#[test]
fn an_external_entity_is_refused_and_an_internal_one_is_read() {
    assert_eq!(
        kind(parse_dtd("<!DOCTYPE r SYSTEM 'r.dtd'><r/>")),
        XmlErrorKind::ExternalEntity
    );
    assert_eq!(
        kind(parse_dtd(
            "<!DOCTYPE r [<!ENTITY e SYSTEM 'e.xml'>]><r>&e;</r>"
        )),
        XmlErrorKind::ExternalEntity
    );
    assert_eq!(
        kind(parse_dtd(
            "<!DOCTYPE r [<!ENTITY e PUBLIC 'p' 'e.xml'>]><r/>"
        )),
        XmlErrorKind::ExternalEntity
    );
    let document = parse_dtd("<!DOCTYPE r [<!ENTITY e 'inside'>]><r>&e;</r>").expect("internal");
    assert_eq!(document.root_element().text(), Some("inside"));
}

#[test]
fn a_parameter_entity_is_refused_and_a_general_one_is_read() {
    assert_eq!(
        kind(parse_dtd("<!DOCTYPE r [<!ENTITY % p 'x'>]><r/>")),
        XmlErrorKind::ParameterEntity
    );
    assert_eq!(
        kind(parse_dtd("<!DOCTYPE r [<!ENTITY e 'a%b'>]><r/>")),
        XmlErrorKind::ParameterEntity
    );
    assert!(parse_dtd("<!DOCTYPE r [<!ENTITY e 'a b'>]><r>&e;</r>").is_ok());
}

#[test]
fn an_entity_with_markup_expands_into_elements() {
    let text = "<!DOCTYPE r [<!ENTITY e '<b x=\"1\">in&amp;side</b>'>]><r>&e;&e;</r>";
    let document = parse_dtd(text).expect("markup entity");
    let children: Vec<_> = document.root_element().children().collect();
    assert_eq!(children.len(), 2);
    assert_eq!(children[0].attribute("x"), Some("1"));
    assert_eq!(children[0].text(), Some("in&side"));
    // Offsets inside the replacement text point at the reference.
    assert_eq!(children[1].offset(), text.rfind("&e;").unwrap());
}

#[test]
fn an_unbalanced_entity_is_refused_and_a_balanced_one_is_read() {
    assert_eq!(
        kind(parse_dtd("<!DOCTYPE r [<!ENTITY e '<b>'>]><r>&e;</b></r>")),
        XmlErrorKind::UnbalancedEntity("e".to_owned())
    );
    assert_eq!(
        kind(parse_dtd("<!DOCTYPE r [<!ENTITY e '</r>'>]><r>&e;")),
        XmlErrorKind::UnbalancedEntity("e".to_owned())
    );
    assert!(parse_dtd("<!DOCTYPE r [<!ENTITY e '<b></b>'>]><r>&e;</r>").is_ok());
}

#[test]
fn a_recursive_entity_is_refused_and_a_nested_one_is_read() {
    assert_eq!(
        kind(parse_dtd(
            "<!DOCTYPE r [<!ENTITY a '&b;'><!ENTITY b '&a;'>]><r>&a;</r>"
        )),
        XmlErrorKind::RecursiveEntity("a".to_owned())
    );
    assert_eq!(
        kind(parse_dtd("<!DOCTYPE r [<!ENTITY a 'x&a;'>]><r v='&a;'/>")),
        XmlErrorKind::RecursiveEntity("a".to_owned())
    );
    let document = parse_dtd("<!DOCTYPE r [<!ENTITY a '1&b;'><!ENTITY b '2'>]><r v='&a;'>&a;</r>")
        .expect("nested");
    assert_eq!(document.root_element().text(), Some("12"));
    assert_eq!(document.root_element().attribute("v"), Some("12"));
}

#[test]
fn an_entity_bomb_is_refused_and_a_small_expansion_is_read() {
    let mut subset = String::from("<!ENTITY l0 'laugh'>");
    for level in 1..=8 {
        let previous = format!("&l{};", level - 1);
        let _ = write!(subset, "<!ENTITY l{level} '{}'>", previous.repeat(10));
    }
    let bomb = format!("<!DOCTYPE r [{subset}]><r>&l8;</r>");
    let budget = Options {
        dtd: Dtd::InternalSubset {
            max_expansion: 1 << 16,
        },
        ..Options::default()
    };
    assert_eq!(
        kind(Document::parse_with_options(&bomb, budget)),
        XmlErrorKind::EntityExpansionLimit { limit: 1 << 16 }
    );
    let small = format!("<!DOCTYPE r [{subset}]><r>&l2;</r>");
    let document = Document::parse_with_options(&small, budget).expect("small expansion");
    assert_eq!(document.root_element().text().map(str::len), Some(500));
}

#[test]
fn attlist_defaults_apply_and_non_cdata_values_collapse() {
    let text = "<!DOCTYPE r [<!ATTLIST r kind CDATA 'plain' id ID #IMPLIED \
                tok NMTOKENS #FIXED ' a  b ' mode (x|y) 'y'>\
                <!ELEMENT r ANY><!NOTATION n SYSTEM 'n>'><!-- c --><?p d?>]>\
                <r id='  i1 ' kind=' k '/>";
    let document = parse_dtd(text).expect("attlist");
    let root = document.root_element();
    assert_eq!(root.attribute("kind"), Some(" k "));
    assert_eq!(root.attribute("id"), Some("i1"));
    assert_eq!(root.attribute("tok"), Some("a b"));
    assert_eq!(root.attribute("mode"), Some("y"));
}

#[test]
fn a_second_doctype_or_one_after_the_root_is_refused() {
    assert_eq!(
        kind(parse_dtd("<!DOCTYPE r><!DOCTYPE r><r/>")),
        XmlErrorKind::MisplacedDoctype
    );
    assert_eq!(
        kind(parse_dtd("<r/><!DOCTYPE r>")),
        XmlErrorKind::MisplacedDoctype
    );
    assert!(parse_dtd("<!DOCTYPE r><r/>").is_ok());
}

#[test]
fn a_redeclared_entity_keeps_its_first_value_and_predefined_ones_keep_their_meaning() {
    let text = "<!DOCTYPE r [<!ENTITY e 'first'><!ENTITY e 'second'>\
                <!ENTITY lt '&#38;#60;'>]><r>&e;&lt;</r>";
    let document = parse_dtd(text).expect("redeclared");
    assert_eq!(document.root_element().text(), Some("first<"));
}
