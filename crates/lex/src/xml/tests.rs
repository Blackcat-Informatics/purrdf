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
fn element_children_skip_text_comments_and_instructions_in_both_directions() {
    let text = "<r>a<x/><!--c--><?pi d?>b<y><z/></y>c</r>";
    let document = parse(text).expect("well-formed");
    let root = document.root_element();
    let names: Vec<&str> = root
        .element_children()
        .map(|child| child.tag_name().name())
        .collect();
    assert_eq!(names, ["x", "y"]);
    let reversed: Vec<&str> = root
        .element_children()
        .rev()
        .map(|child| child.tag_name().name())
        .collect();
    assert_eq!(reversed, ["y", "x"]);
    let leaf = root.first_element_child().expect("x");
    assert_eq!(leaf.element_children().count(), 0);
}

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
    // An XML 1.1 document is refused, never read as though it were 1.0; the
    // neighbouring 1.0 declaration is read.
    assert_eq!(
        kind(parse("<?xml version='1.1'?><a/>")),
        XmlErrorKind::UnsupportedVersion("1.1".to_owned())
    );
    let mut reader = Reader::new("<?xml version='1.0'?><a/>");
    match reader.next().expect("declaration") {
        Event::Declaration(declaration) => {
            assert_eq!(declaration.version, "1.0");
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
fn an_external_entity_reference_is_refused_and_an_internal_one_is_read() {
    // A reference the reader cannot expand is refused, never dropped.
    assert_eq!(
        kind(parse_dtd(
            "<!DOCTYPE r [<!ENTITY e SYSTEM 'e.xml'>]><r>&e;</r>"
        )),
        XmlErrorKind::ExternalEntity
    );
    assert_eq!(
        kind(parse_dtd(
            "<!DOCTYPE r [<!ENTITY e PUBLIC 'p' 'e.xml'>]><r a='&e;'/>"
        )),
        XmlErrorKind::ExternalEntity
    );
    // The neighbours: declaring an external entity, or naming an external
    // subset, is well-formed and is read without fetching anything.
    assert!(parse_dtd("<!DOCTYPE r [<!ENTITY e SYSTEM 'e.xml'>]><r/>").is_ok());
    assert!(parse_dtd("<!DOCTYPE r PUBLIC 'p' 'e.xml'><r/>").is_ok());
    let document = parse_dtd("<!DOCTYPE r [<!ENTITY e 'inside'>]><r>&e;</r>").expect("internal");
    assert_eq!(document.root_element().text(), Some("inside"));
    assert!(!document.declarations_unread());
}

#[test]
fn an_external_subset_is_not_read_and_says_so() {
    let document = parse_dtd("<!DOCTYPE r SYSTEM 'r.dtd'><r/>").expect("named, not fetched");
    assert!(document.declarations_unread());
    // A reference only the unread subset could bind is refused as such; with
    // no external subset the same reference is plainly undeclared.
    assert_eq!(
        kind(parse_dtd("<!DOCTYPE r SYSTEM 'r.dtd'><r>&x;</r>")),
        XmlErrorKind::UnexpandedEntity("x".to_owned())
    );
    assert_eq!(
        kind(parse_dtd("<!DOCTYPE r><r>&x;</r>")),
        XmlErrorKind::UndeclaredEntity("x".to_owned())
    );
    // `standalone="yes"` makes the declaration mandatory again.
    assert_eq!(
        kind(parse_dtd(
            "<?xml version='1.0' standalone='yes'?><!DOCTYPE r SYSTEM 'r.dtd'><r>&x;</r>"
        )),
        XmlErrorKind::UndeclaredEntity("x".to_owned())
    );
    // A malformed external identifier is still refused.
    assert!(parse_dtd("<!DOCTYPE r SYSTEM><r/>").is_err());
    assert!(parse_dtd("<!DOCTYPE r PUBLIC 'p'><r/>").is_err());
    assert!(parse_dtd("<!DOCTYPE r PUBLIC 'a\u{1}b' 's'><r/>").is_err());
}

#[test]
fn an_unparsed_entity_may_be_declared_and_is_refused_when_referenced() {
    let declared = "<!DOCTYPE r [<!NOTATION n SYSTEM 'n'><!ENTITY u SYSTEM 'u.gif' NDATA n>]>";
    assert!(parse_dtd(&format!("{declared}<r/>")).is_ok());
    assert_eq!(
        kind(parse_dtd(&format!("{declared}<r>&u;</r>"))),
        XmlErrorKind::UnparsedEntity("u".to_owned())
    );
    assert_eq!(
        kind(parse_dtd(&format!("{declared}<r a='&u;'/>"))),
        XmlErrorKind::UnparsedEntity("u".to_owned())
    );
}

// ── Parameter entities ─────────────────────────────────────────────────────

#[test]
fn an_internal_parameter_entity_declares_entities_between_declarations() {
    // A parameter entity whose replacement text is a declaration.
    let document =
        parse_dtd("<!DOCTYPE r [<!ENTITY % d \"<!ENTITY e 'declared by a PE'>\"> %d;]><r>&e;</r>")
            .expect("a parameter-entity declaration and reference");
    assert_eq!(document.root_element().text(), Some("declared by a PE"));
    // Several declarations, comments, processing instructions and ATTLIST
    // defaults, produced by one replacement text.
    let document = parse_dtd(
        "<!DOCTYPE r [<!ENTITY % all \"<!ELEMENT r ANY><!ATTLIST r a CDATA 'dflt'><!-- c -->\
         <?p d?><!ENTITY e 'x'>\"> %all;]><r>&e;</r>",
    )
    .expect("declarations from a parameter entity");
    assert_eq!(document.root_element().attribute("a"), Some("dflt"));
    assert_eq!(document.root_element().text(), Some("x"));
    // Appendix D: the character reference in the value is resolved when `xx`
    // is declared, so `%xx;` is itself a reference to `zz`.
    let document = parse_dtd(
        "<!DOCTYPE t [<!ELEMENT t (#PCDATA)><!ENTITY % xx '&#37;zz;'>\
         <!ENTITY % zz '&#60;!ENTITY tricky \"error-prone\" >' > %xx;]>\
         <t>a &tricky; method</t>",
    )
    .expect("Appendix D example 2");
    assert_eq!(document.root_element().text(), Some("a error-prone method"));
    // A reference may sit anywhere between declarations, and follow the
    // entity's own text.
    assert!(parse_dtd("<!DOCTYPE r [<!ENTITY % e '<!---->'>%e;<!---->%e;]><r/>").is_ok());
    // The declaration alone, unreferenced, is well-formed; the first
    // declaration of a name binds.
    let document = parse_dtd(
        "<!DOCTYPE r [<!ENTITY % e '<!ENTITY g \"first\">'><!ENTITY % e '<!ENTITY g \"second\">'>%e;]>\
         <r>&g;</r>",
    )
    .expect("first binds");
    assert_eq!(document.root_element().text(), Some("first"));
}

#[test]
fn parameter_entity_syntax_and_placement_are_checked() {
    // Malformed references: no `;`, no name.
    assert!(matches!(
        kind(parse_dtd("<!DOCTYPE r [<!ENTITY % e ''>%e]><r/>")),
        XmlErrorKind::Expected(_)
    ));
    assert_eq!(
        kind(parse_dtd("<!DOCTYPE r [%;]><r/>")),
        XmlErrorKind::InvalidName
    );
    assert_eq!(
        kind(parse_dtd("<!DOCTYPE r [<!ENTITY % e ''>% e;]><r/>")),
        XmlErrorKind::InvalidName
    );
    // Malformed declarations: no space after `%`, no value.
    assert!(parse_dtd("<!DOCTYPE r [<!ENTITY %e 'x'>]><r/>").is_err());
    assert!(parse_dtd("<!DOCTYPE r [<!ENTITY % e>]><r/>").is_err());
    // Well-formedness constraint "PEs in Internal Subset": a reference within a
    // markup declaration is refused, in each declaration kind.
    for text in [
        "<!DOCTYPE r [<!ENTITY % p 'x'><!ENTITY g '%p;'>]><r/>",
        "<!DOCTYPE r [<!ENTITY % p 'x'><!ELEMENT r (%p;)>]><r/>",
        "<!DOCTYPE r [<!ENTITY % p 'x'><!ATTLIST r %p; CDATA #IMPLIED>]><r/>",
        "<!DOCTYPE r [<!ENTITY % p 'x'><!ENTITY %p; 'v'>]><r/>",
        "<!DOCTYPE r [<!ENTITY % p 'x'><!NOTATION n %p;>]><r/>",
    ] {
        assert_eq!(
            kind(parse_dtd(text)),
            XmlErrorKind::ParameterEntity,
            "{text}"
        );
    }
    // Between declarations the same reference is fine, and inside a comment or
    // an ATTLIST default it is text.
    assert!(
        parse_dtd("<!DOCTYPE r [<!ENTITY % p 'x'><!-- %p; --><!ATTLIST r a CDATA '%p;'>%p;]><r/>")
            .is_err(),
        "`x` is not a declaration"
    );
    let document = parse_dtd(
        "<!DOCTYPE r [<!ENTITY % p ''><!-- %nope; --><!ATTLIST r a CDATA '%p;'>%p;]><r/>",
    )
    .expect("inert references");
    assert_eq!(document.root_element().attribute("a"), Some("%p;"));
    // An undeclared parameter entity is refused.
    assert_eq!(
        kind(parse_dtd("<!DOCTYPE r [%nope;]><r/>")),
        XmlErrorKind::UndeclaredParameterEntity("nope".to_owned())
    );
    // A conditional section belongs to the external subset.
    assert!(parse_dtd("<!DOCTYPE r [<![INCLUDE[ ]]>]><r/>").is_err());
    // Text that is not a declaration.
    assert!(parse_dtd("<!DOCTYPE r [<!ENTITY % p 'stray text'>%p;]><r/>").is_err());
}

#[test]
fn a_declaration_closes_in_the_entity_it_opens_in() {
    // "Proper Declaration/PE Nesting".
    let open = "<!DOCTYPE r [<!ENTITY % e \"<!ENTITY g 'x'\">%e;>]><r/>";
    assert_eq!(
        kind(parse_dtd(open)),
        XmlErrorKind::UnbalancedEntity("%e".to_owned())
    );
    let closed = "<!DOCTYPE r [<!ENTITY % e \"<!ENTITY g 'x'>\">%e;]><r>&g;</r>";
    assert!(parse_dtd(closed).is_ok());
    // An error inside an entity is reported at the reference to it.
    let error = parse_dtd("<!DOCTYPE r [<!ENTITY % e '<!ELEMENT>'>%e;]><r/>").unwrap_err();
    assert_eq!(
        error.offset(),
        "<!DOCTYPE r [<!ENTITY % e '<!ELEMENT>'>".len()
    );
}

#[test]
fn recursive_parameter_entities_are_refused_and_nested_ones_are_read() {
    assert_eq!(
        kind(parse_dtd(
            "<!DOCTYPE r [<!ENTITY % a '&#37;b;'><!ENTITY % b '&#37;a;'>%a;]><r/>"
        )),
        XmlErrorKind::RecursiveEntity("%a".to_owned())
    );
    assert_eq!(
        kind(parse_dtd("<!DOCTYPE r [<!ENTITY % a '&#37;a;'>%a;]><r/>")),
        XmlErrorKind::RecursiveEntity("%a".to_owned())
    );
    let document = parse_dtd(
        "<!DOCTYPE r [<!ENTITY % a '<!ENTITY g \"deep\">'><!ENTITY % b '&#37;a;'>\
         <!ENTITY % c '&#37;b;&#37;b;'>%c;]><r>&g;</r>",
    )
    .expect("the same entity twice is not recursion");
    assert_eq!(document.root_element().text(), Some("deep"));
}

#[test]
fn a_parameter_entity_bomb_is_refused_and_a_small_expansion_is_read() {
    // Each level doubles the last: 2^25 comments from a few hundred bytes.
    let mut subset = String::from("<!ENTITY % l0 '<!---->'>");
    for level in 1..=25 {
        let _ = write!(
            subset,
            "<!ENTITY % l{level} '&#37;l{};&#37;l{};'>",
            level - 1,
            level - 1
        );
    }
    let bomb = format!("<!DOCTYPE r [{subset}%l25;]><r/>");
    assert!(matches!(
        kind(parse_dtd(&bomb)),
        XmlErrorKind::EntityExpansionLimit { .. }
    ));
    let small = format!("<!DOCTYPE r [{subset}%l4;]><r/>");
    assert!(parse_dtd(&small).is_ok());
    // The budget is the caller's: the same small document under a tiny one.
    let options = Options {
        dtd: Dtd::InternalSubset { max_expansion: 8 },
        ..Options::default()
    };
    assert!(matches!(
        kind(Document::parse_with_options(&small, options)),
        XmlErrorKind::EntityExpansionLimit { limit: 8 }
    ));
    // Parameter and general entities draw on one budget.
    let mixed = "<!DOCTYPE r [<!ENTITY % p '<!ENTITY g \"0123456789\">'>%p;]><r>&g;</r>";
    let tight = Options {
        dtd: Dtd::InternalSubset { max_expansion: 40 },
        ..Options::default()
    };
    assert!(Document::parse_with_options(mixed, tight).is_ok());
    let tighter = Options {
        dtd: Dtd::InternalSubset { max_expansion: 20 },
        ..Options::default()
    };
    assert!(matches!(
        kind(Document::parse_with_options(mixed, tighter)),
        XmlErrorKind::EntityExpansionLimit { .. }
    ));
}

#[test]
fn an_external_parameter_entity_is_not_read_and_later_declarations_bind_nothing() {
    let text = "<!DOCTYPE r [<!ENTITY early 'e'><!ENTITY % x SYSTEM 'x.ent'>%x;\
                <!ENTITY late 'l'><!ATTLIST r a CDATA 'dflt'>]>";
    let source = format!("{text}<r>&early;</r>");
    let document = parse_dtd(&source).expect("§5.1");
    assert!(document.declarations_unread());
    assert_eq!(document.root_element().text(), Some("e"));
    // The default after the unread reference does not apply, and the entity
    // it might have declared is refused, not guessed.
    assert_eq!(document.root_element().attribute("a"), None);
    assert_eq!(
        kind(parse_dtd(&format!("{text}<r>&late;</r>"))),
        XmlErrorKind::UnexpandedEntity("late".to_owned())
    );
    // What follows is still checked for well-formedness.
    assert!(parse_dtd("<!DOCTYPE r [<!ENTITY % x SYSTEM 'x.ent'>%x;<!ELEMENT>]><r/>").is_err());
    // A reference to a parameter entity that only the unread one could have
    // declared is passed over.
    assert!(parse_dtd("<!DOCTYPE r [<!ENTITY % x SYSTEM 'x.ent'>%x;%y;]><r/>").is_ok());
    // A general entity reference is refused wherever declarations were left
    // unread, including because of an internal parameter entity (§4.4.3: the
    // reader cannot report a skipped entity).
    assert_eq!(
        kind(parse_dtd(
            "<!DOCTYPE r [<!ENTITY % p '<!ENTITY a \"x\">'>%p;]><r>&b;</r>"
        )),
        XmlErrorKind::UnexpandedEntity("b".to_owned())
    );
}

#[test]
fn parameter_entity_text_is_read_by_the_same_grammar() {
    // The processing instruction inside a replacement text reaches the caller.
    let mut reader = Reader::with_options(
        "<!DOCTYPE r [<!ENTITY % p '<?inside data?>'><?before x?>%p;]><r/>",
        Options {
            dtd: Dtd::internal_subset(),
            ..Options::default()
        },
    );
    let mut targets = Vec::new();
    loop {
        match reader.next().expect("well-formed") {
            Event::Pi { target, .. } => targets.push(target.into_owned()),
            Event::Eof => break,
            _ => {}
        }
    }
    assert_eq!(targets, ["before", "inside"]);
}

// ── Declarations: the grammar a non-validating processor checks ────────────

#[test]
fn element_content_models_are_checked_and_deep_ones_do_not_overflow() {
    for good in [
        "<!ELEMENT r EMPTY>",
        "<!ELEMENT r ANY>",
        "<!ELEMENT r (#PCDATA)>",
        "<!ELEMENT r (#PCDATA)*>",
        "<!ELEMENT r (#PCDATA|a|b)*>",
        "<!ELEMENT r ( #PCDATA | a )* >",
        "<!ELEMENT r (a)>",
        "<!ELEMENT r (a,b?,(c|d)+,(e,f)*)+>",
        "<!ELEMENT r ( a , ( b | c ) )?>",
    ] {
        assert!(
            parse_dtd(&format!("<!DOCTYPE r [{good}]><r/>")).is_ok(),
            "{good}"
        );
    }
    for bad in [
        "<!ELEMENT r>",
        "<!ELEMENT r a>",
        "<!ELEMENT r (a|b,c)>",
        "<!ELEMENT r (a,b|c)>",
        "<!ELEMENT r (a|)>",
        "<!ELEMENT r ()>",
        "<!ELEMENT r (a b)>",
        "<!ELEMENT r (#PCDATA|a)>",
        "<!ELEMENT r (a|#PCDATA)*>",
        "<!ELEMENT r (a)??>",
        "<!ELEMENT r ( a , ( b | c ) ) ?>",
        "<!ELEMENT r EMPTY x>",
        "<!ELEMENTr EMPTY>",
        "<!ELEMENT r (a",
    ] {
        assert!(
            parse_dtd(&format!("<!DOCTYPE r [{bad}]><r/>")).is_err(),
            "{bad}"
        );
    }
    // Groups nest on the heap: a hundred thousand deep is read (and a
    // truncated one refused) without exhausting the stack.
    let depth = 100_000;
    let deep = format!("{}a{}", "(".repeat(depth), ")".repeat(depth));
    assert!(parse_dtd(&format!("<!DOCTYPE r [<!ELEMENT r {deep}>]><r/>")).is_ok());
    let truncated = format!("{}a{}", "(".repeat(depth), ")".repeat(depth - 1));
    assert!(parse_dtd(&format!("<!DOCTYPE r [<!ELEMENT r {truncated}>]><r/>")).is_err());
}

#[test]
fn attlist_and_notation_and_entity_declarations_are_checked() {
    for good in [
        "<!ATTLIST r a CDATA #IMPLIED b ID #REQUIRED c IDREF #IMPLIED d IDREFS #IMPLIED>",
        "<!ATTLIST r a (x|y|z) 'x' b NOTATION (n|m) #IMPLIED c NMTOKENS #FIXED 'a b'>",
        "<!ATTLIST r>",
        "<!ATTLIST r a ENTITY #IMPLIED b ENTITIES #IMPLIED c NMTOKEN '1x'>",
        "<!NOTATION n SYSTEM 's'><!NOTATION m PUBLIC 'p'><!NOTATION o PUBLIC 'p' 's'>",
        "<!ENTITY e SYSTEM 's'><!ENTITY f PUBLIC 'p' 's'><!ENTITY g 'v'>",
    ] {
        assert!(
            parse_dtd(&format!("<!DOCTYPE r [{good}]><r/>")).is_ok(),
            "{good}"
        );
    }
    for bad in [
        "<!ATTLIST r a CDATA>",
        "<!ATTLIST r a CDATA #IMPLIED b>",
        "<!ATTLIST r a STRING #IMPLIED>",
        "<!ATTLIST r a (x|) #IMPLIED>",
        "<!ATTLIST r a () #IMPLIED>",
        "<!ATTLIST r a NOTATION #IMPLIED>",
        "<!ATTLIST r a NOTATION (1n) #IMPLIED>",
        "<!ATTLIST r a CDATA #FIXED>",
        "<!ATTLIST r a CDATA 'x<y'>",
        "<!ATTLIST r a CDATA '&#0;'>",
        "<!ATTLIST r aCDATA #IMPLIED>",
        "<!NOTATION n>",
        "<!NOTATION n a:b SYSTEM 's'>",
        "<!NOTATION n PUBLIC 'a\u{1}' 's'>",
        "<!ENTITY e SYSTEM>",
        "<!ENTITY e PUBLIC 'p'>",
        "<!ENTITY e SYSTEM 's' NDATA>",
        "<!ENTITY % e SYSTEM 's' NDATA n>",
        "<!ENTITY e 'v' NDATA n>",
        "<!ENTITY a:b 'v'>",
        "<!ENTITY e 'v'",
    ] {
        assert!(
            parse_dtd(&format!("<!DOCTYPE r [{bad}]><r/>")).is_err(),
            "{bad}"
        );
    }
}

#[test]
fn a_default_refers_only_to_entities_declared_before_it() {
    // "Entity Declared": the reference is checked where the default is
    // declared.
    let before = "<!DOCTYPE r [<!ENTITY e 'v'><!ATTLIST r a CDATA 'x&e;y'>]><r/>";
    assert_eq!(
        parse_dtd(before).unwrap().root_element().attribute("a"),
        Some("xvy")
    );
    assert_eq!(
        kind(parse_dtd(
            "<!DOCTYPE r [<!ATTLIST r a CDATA '&e;'><!ENTITY e 'v'>]><r/>"
        )),
        XmlErrorKind::UndeclaredEntity("e".to_owned())
    );
    // An external entity in a default is a well-formedness error.
    assert_eq!(
        kind(parse_dtd(
            "<!DOCTYPE r [<!ENTITY e SYSTEM 's'><!ATTLIST r a CDATA '&e;'>]><r/>"
        )),
        XmlErrorKind::ExternalEntity
    );
}

#[test]
fn the_predefined_entities_may_be_declared_only_as_themselves() {
    for good in [
        "<!ENTITY lt '&#38;#60;'>",
        "<!ENTITY lt '&#38;#x3C;'>",
        "<!ENTITY amp '&#38;#38;'>",
        "<!ENTITY gt '>'>",
        "<!ENTITY gt '&#62;'>",
        "<!ENTITY apos \"'\">",
        "<!ENTITY quot '&#34;'>",
        "<!ENTITY quot '&#38;#34;'>",
    ] {
        assert!(
            parse_dtd(&format!("<!DOCTYPE r [{good}]><r/>")).is_ok(),
            "{good}"
        );
    }
    for bad in [
        "<!ENTITY lt '<'>",
        "<!ENTITY lt '&#60;'>",
        "<!ENTITY amp '&#38;'>",
        "<!ENTITY gt 'x'>",
        "<!ENTITY lt SYSTEM 's'>",
    ] {
        assert!(
            matches!(
                kind(parse_dtd(&format!("<!DOCTYPE r [{bad}]><r/>"))),
                XmlErrorKind::PredefinedEntity(_)
            ),
            "{bad}"
        );
    }
}

// ── Versions and encodings ─────────────────────────────────────────────────

#[test]
fn only_xml_1_0_is_read_and_a_later_1_x_reads_as_1_0() {
    assert_eq!(
        kind(parse("<?xml version='1.1'?><r/>")),
        XmlErrorKind::UnsupportedVersion("1.1".to_owned())
    );
    assert!(parse("<?xml version='1.0'?><r/>").is_ok());
    // XML 1.0 Fifth Edition: `1.` and digits are a legal `VersionNum`.
    assert!(parse("<?xml version='1.7'?><r/>").is_ok());
    assert!(parse("<?xml version='1.10'?><r/>").is_ok());
    assert!(parse("<?xml version='2.0'?><r/>").is_err());
}

#[test]
fn documents_decode_under_appendix_f_and_unsupported_encodings_are_refused() {
    use super::decode;
    let text = "<?xml version='1.0' encoding='X'?><r>é€</r>";
    let with = |encoding: &str| text.replace('X', encoding);
    // UTF-8, declared or not, borrowed.
    assert_eq!(decode(with("UTF-8").as_bytes()).unwrap(), with("UTF-8"));
    assert_eq!(decode(with("utf-8").as_bytes()).unwrap(), with("utf-8"));
    assert_eq!(decode("<r>é</r>".as_bytes()).unwrap(), "<r>é</r>");
    let bom_utf8: Vec<u8> = [&[0xEF, 0xBB, 0xBF][..], with("UTF-8").as_bytes()].concat();
    assert!(Document::parse(&decode(&bom_utf8).unwrap()).is_ok());
    // UTF-16, either byte order, with a byte order mark.
    let utf16 = |big: bool, body: &str| -> Vec<u8> {
        let mut out = if big {
            vec![0xFE, 0xFF]
        } else {
            vec![0xFF, 0xFE]
        };
        for unit in body.encode_utf16() {
            out.extend(if big {
                unit.to_be_bytes()
            } else {
                unit.to_le_bytes()
            });
        }
        out
    };
    for big in [true, false] {
        let bytes = utf16(big, &with("UTF-16"));
        let decoded = decode(&bytes).unwrap();
        let document = Document::parse(&decoded).expect("UTF-16");
        assert_eq!(document.root_element().text(), Some("é€"));
        // The same bytes under a UTF-8 declaration contradict themselves.
        assert!(matches!(
            decode(&utf16(big, &with("UTF-8"))).unwrap_err().kind(),
            XmlErrorKind::Encoding(_)
        ));
    }
    // UTF-16 with no byte order mark is read only when it says so.
    let bare = |body: &str| -> Vec<u8> { body.encode_utf16().flat_map(u16::to_be_bytes).collect() };
    assert!(decode(&bare("<?xml version='1.0'?><r/>")).is_err());
    assert!(decode(&bare("<?xml version='1.0' encoding='UTF-16'?><r/>")).is_ok());
    // An unpaired surrogate and an odd byte count are refused.
    assert!(decode(&[0xFE, 0xFF, 0xD8, 0x00, 0x00, 0x3C]).is_err());
    assert!(decode(&[0xFE, 0xFF, 0x00]).is_err());
    // A UTF-16 declaration on UTF-8 bytes contradicts itself.
    assert!(matches!(
        decode(with("UTF-16").as_bytes()).unwrap_err().kind(),
        XmlErrorKind::Encoding(_)
    ));
    // ISO-8859-1 and US-ASCII.
    let latin1 = b"<?xml version='1.0' encoding='ISO-8859-1'?><r>\xE9</r>";
    let decoded = decode(latin1).unwrap();
    assert_eq!(
        Document::parse(&decoded).unwrap().root_element().text(),
        Some("é")
    );
    assert!(decode(b"<?xml version='1.0' encoding='US-ASCII'?><r>a</r>").is_ok());
    assert!(matches!(
        decode(b"<?xml version='1.0' encoding='US-ASCII'?><r>\xE9</r>")
            .unwrap_err()
            .kind(),
        XmlErrorKind::Encoding(_)
    ));
    // Ill-formed UTF-8 is refused where a neighbouring valid document is read.
    assert!(matches!(
        decode(b"<r>\xFF</r>").unwrap_err().kind(),
        XmlErrorKind::Encoding(_)
    ));
    assert!(decode(b"<r>\xC3\xA9</r>").is_ok());
    // Anything else is refused by name, never read as though it were UTF-8.
    for name in [
        "Shift_JIS",
        "EUC-JP",
        "ISO-2022-JP",
        "windows-1252",
        "KOI8-R",
    ] {
        assert_eq!(
            decode(with(name).as_bytes()).unwrap_err().kind(),
            &XmlErrorKind::UnsupportedEncoding(name.to_owned())
        );
    }
    assert!(matches!(
        decode(&[0x00, 0x00, 0x00, 0x3C]).unwrap_err().kind(),
        XmlErrorKind::UnsupportedEncoding(_)
    ));
    assert!(matches!(
        decode(&[0x4C, 0x6F, 0xA7, 0x94]).unwrap_err().kind(),
        XmlErrorKind::UnsupportedEncoding(_)
    ));
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
