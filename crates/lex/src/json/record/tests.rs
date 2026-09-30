// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use std::collections::{BTreeMap, BTreeSet};

use super::{
    DecodeError, FromJson, Record, ToJson, Within, from_slice, into_owned, items_with,
    read_document, sorted_last_wins,
};
use crate::json::{self, Value};

#[derive(Debug, PartialEq, Eq)]
struct Pair {
    name: String,
    count: Option<u32>,
}

impl FromJson for Pair {
    fn from_json(value: &Value) -> Result<Self, DecodeError> {
        let mut fields = Record::new(value, "struct Pair")?;
        let pair = Self {
            name: fields.required("name")?,
            count: fields.optional("count")?,
        };
        fields.deny_unknown()?;
        Ok(pair)
    }
}

/// A record that reads `pairs` and ignores every other member.
#[derive(Debug, PartialEq, Eq)]
struct Open {
    pairs: Vec<Pair>,
}

impl FromJson for Open {
    fn from_json(value: &Value) -> Result<Self, DecodeError> {
        let mut fields = Record::new(value, "an object")?;
        Ok(Self {
            pairs: fields.defaulted("pairs")?,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Colour {
    Red,
    DarkBlue,
}

crate::json_string_enum!(Colour { Red => "red", DarkBlue => "dark-blue" });

fn doc(text: &str) -> Value {
    json::read(text).expect("test JSON")
}

fn pair(text: &str) -> Result<Pair, DecodeError> {
    from_slice(text.as_bytes())
}

fn open(text: &str) -> Result<Open, DecodeError> {
    from_slice(text.as_bytes())
}

// ── Records ────────────────────────────────────────────────────────────────

#[test]
fn a_record_reads_declared_members_and_treats_null_as_absent() {
    assert_eq!(
        pair(r#"{"count":3,"name":"a"}"#).expect("record"),
        Pair {
            name: "a".to_owned(),
            count: Some(3)
        }
    );
    assert_eq!(
        pair(r#"{"name":"a","count":null}"#).expect("null optional"),
        pair(r#"{"name":"a"}"#).expect("absent optional")
    );
}

#[test]
fn an_unknown_member_is_refused_naming_the_declared_set_and_a_declared_one_reads() {
    assert_eq!(
        pair(r#"{"name":"a","extra":1}"#)
            .expect_err("unknown")
            .to_string(),
        "unknown field `extra`, expected `name` or `count`"
    );
    assert!(pair(r#"{"name":"a","count":1}"#).is_ok());
}

#[test]
fn a_missing_member_is_refused_and_a_present_one_reads() {
    let error = pair(r#"{"count":1}"#).expect_err("missing");
    assert_eq!(error.to_string(), "missing field `name`");
    assert_eq!(
        error.pointer(),
        "",
        "a missing member is the record's fault"
    );
    assert!(pair(r#"{"name":"a","count":1}"#).is_ok());
}

#[test]
fn a_repeated_member_is_refused_at_the_member_and_a_single_one_reads() {
    let error = pair(r#"{"name":"a","name":"b"}"#).expect_err("duplicate");
    assert_eq!(error.message(), "duplicate field `name`");
    assert_eq!(error.pointer(), "/name");
    assert_eq!(error.to_string(), "duplicate field `name` at /name");
    assert!(pair(r#"{"name":"a"}"#).is_ok());
}

/// A record that never calls `deny_unknown` is open: a member it does not
/// read is ignored, repeated or not, while a member it reads is still refused
/// when repeated.
#[test]
fn an_open_record_ignores_unread_members_but_refuses_a_repeated_read_one() {
    assert_eq!(
        open(r#"{"note":1,"note":2}"#).expect("unread members"),
        Open { pairs: vec![] }
    );
    let error = open(r#"{"pairs":[],"pairs":[]}"#).expect_err("repeated read member");
    assert_eq!(error.message(), "duplicate field `pairs`");
    assert!(open(r#"{"pairs":[]}"#).is_ok());
}

/// A record is an object: the positional array spelling of a record is
/// refused, and the same members as an object are read.
#[test]
fn a_record_is_never_read_from_an_array() {
    assert_eq!(
        pair(r#"["a",1]"#).expect_err("array").to_string(),
        "invalid type: sequence, expected struct Pair"
    );
    assert_eq!(
        pair("7").expect_err("number").to_string(),
        "invalid type: integer `7`, expected struct Pair"
    );
    assert!(pair(r#"{"name":"a","count":1}"#).is_ok());
}

#[test]
fn a_defaulted_member_takes_its_default_only_when_absent() {
    assert_eq!(open("{}").expect("absent"), Open { pairs: vec![] });
    assert_eq!(
        open(r#"{"pairs":null}"#)
            .expect_err("null list")
            .to_string(),
        "invalid type: null, expected a sequence at /pairs"
    );
    assert_eq!(
        open(r#"{"pairs":[{"name":"a"}]}"#).expect("list"),
        Open {
            pairs: vec![Pair {
                name: "a".to_owned(),
                count: None
            }]
        }
    );
}

#[test]
fn a_tag_must_be_present_a_string_and_one_of_its_variants() {
    let read = |text: &str| -> Result<String, DecodeError> {
        let value = doc(text);
        let mut fields = Record::new(&value, "internally tagged enum")?;
        let tag = fields.tag("kind", &["all", "some"])?.to_owned();
        fields.deny_unknown()?;
        Ok(tag)
    };
    assert_eq!(
        read("{}").expect_err("missing").to_string(),
        "missing field `kind`"
    );
    assert_eq!(
        read(r#"{"kind":1}"#).expect_err("number").to_string(),
        "invalid type: integer `1`, expected variant identifier at /kind"
    );
    assert_eq!(
        read(r#"{"kind":"none"}"#).expect_err("unknown").to_string(),
        "unknown variant `none`, expected `all` or `some` at /kind"
    );
    assert_eq!(
        read(r#"{"kind":"all","extra":0}"#)
            .expect_err("member beside a field-less tag")
            .to_string(),
        "unknown field `extra`, expected `kind`"
    );
    assert_eq!(read(r#"{"kind":"some"}"#), Ok("some".to_owned()));
}

// ── Pointers ───────────────────────────────────────────────────────────────

#[test]
fn a_refusal_names_the_pointer_of_the_offending_value() {
    let error = open(r#"{"pairs":[{"name":"a"},{"name":"b","count":"x"}]}"#).expect_err("count");
    assert_eq!(error.pointer(), "/pairs/1/count");
    assert_eq!(
        error.to_string(),
        "invalid type: string \"x\", expected u32 at /pairs/1/count"
    );
    let error = open(r#"{"pairs":[{"count":1}]}"#).expect_err("missing name");
    assert_eq!(error.pointer(), "/pairs/0");
    assert!(open(r#"{"pairs":[{"name":"b","count":1}]}"#).is_ok());
}

#[test]
fn a_pointer_escapes_tilde_and_slash_in_member_names() {
    let map: Result<BTreeMap<String, u8>, _> = from_slice(br#"{"a/b~c":300}"#);
    assert_eq!(map.expect_err("range").pointer(), "/a~1b~0c");
    let map: BTreeMap<String, u8> = from_slice(br#"{"a/b~c":255}"#).expect("in range");
    assert_eq!(map.get("a/b~c"), Some(&255));
}

/// A caller's own error type: a construction-law refusal and a shape refusal.
#[derive(Debug, PartialEq, Eq)]
enum Caller {
    Shape { pointer: String, reason: String },
    Law(String),
}

impl From<DecodeError> for Caller {
    fn from(error: DecodeError) -> Self {
        Self::Shape {
            pointer: error.pointer().to_owned(),
            reason: error.message().to_owned(),
        }
    }
}

impl Within for Caller {
    fn within(self, token: &str) -> Self {
        match self {
            Self::Shape { pointer, reason } => Self::Shape {
                pointer: format!("/{token}{pointer}"),
                reason,
            },
            law @ Self::Law(_) => law,
        }
    }
}

#[test]
fn a_callers_law_refusal_passes_unchanged_while_its_shape_refusal_gains_a_pointer() {
    let read = |text: &str| -> Result<Vec<String>, Caller> {
        let value = doc(text);
        let mut fields = Record::new(&value, "an object")?;
        fields.required_with("names", |names| {
            items_with(names, |name| {
                let name = String::from_json(name)?;
                if name.is_empty() {
                    Err(Caller::Law("an empty name".to_owned()))
                } else {
                    Ok(name)
                }
            })
        })
    };
    assert_eq!(
        read(r#"{"names":["a",""]}"#),
        Err(Caller::Law("an empty name".to_owned()))
    );
    assert_eq!(
        read(r#"{"names":["a",1]}"#),
        Err(Caller::Shape {
            pointer: "/names/1".to_owned(),
            reason: "invalid type: integer `1`, expected a string".to_owned()
        })
    );
    assert_eq!(
        read(r#"{"names":["a","b"]}"#),
        Ok(vec!["a".to_owned(), "b".to_owned()])
    );
}

#[test]
fn optional_with_reads_a_present_member_and_skips_null() {
    let value = doc(r#"{"a":null,"b":[1]}"#);
    let mut fields = Record::new(&value, "an object").expect("object");
    assert_eq!(
        fields.optional_with("a", |value| Ok::<_, DecodeError>(value.clone())),
        Ok(None)
    );
    assert_eq!(
        fields.optional_with("b", |value| Ok::<_, DecodeError>(value.clone())),
        Ok(Some(doc("[1]")))
    );
    assert!(fields.deny_unknown().is_ok());
}

// ── Scalars ────────────────────────────────────────────────────────────────

#[test]
fn integers_refuse_fractions_exponents_and_out_of_range_values() {
    assert_eq!(u32::from_json(&doc("7")), Ok(7));
    assert_eq!(
        u32::from_json(&doc("7.0"))
            .expect_err("fraction")
            .to_string(),
        "invalid type: floating point `7.0`, expected u32"
    );
    assert!(u32::from_json(&doc("1e2")).is_err());
    assert!(u32::from_json(&doc("\"1\"")).is_err());
    assert_eq!(
        u8::from_json(&doc("256")).expect_err("range").to_string(),
        "invalid value: integer `256`, expected u8"
    );
    assert_eq!(u8::from_json(&doc("255")), Ok(255));
    assert_eq!(u32::from_json(&doc("4294967295")), Ok(u32::MAX));
    assert!(u32::from_json(&doc("4294967296")).is_err());
    assert_eq!(u32::from_json(&doc("-0")), Ok(0));
    assert!(u64::from_json(&doc("-1")).is_err());
    assert_eq!(i64::from_json(&doc("-1")), Ok(-1));
    assert_eq!(i32::from_json(&doc("2147483647")), Ok(i32::MAX));
    assert!(i32::from_json(&doc("2147483648")).is_err());
    assert_eq!(
        i128::from_json(&doc("-170141183460469231731687303715884105728")),
        Ok(i128::MIN)
    );
}

#[test]
fn a_float_beyond_its_range_is_refused_and_its_neighbour_reads() {
    assert_eq!(
        f32::from_json(&doc("1e39"))
            .expect_err("f32 range")
            .to_string(),
        "invalid value: floating point `1e39`, expected a finite f32"
    );
    assert!(f32::from_json(&doc("-1e39")).is_err());
    assert_eq!(f32::from_json(&doc("3.4028235e38")), Ok(f32::MAX));
    assert_eq!(f32::from_json(&doc("2")), Ok(2.0));
    assert_eq!(f32::from_json(&doc("0.1")), Ok(0.1_f32));
    assert!(f64::from_json(&doc("1e309")).is_err());
    assert_eq!(f64::from_json(&doc("1.7976931348623157e308")), Ok(f64::MAX));
    assert_eq!(
        f64::from_json(&doc("null")).expect_err("null").to_string(),
        "invalid type: null, expected f64"
    );
}

#[test]
fn a_character_is_exactly_one_scalar_value() {
    assert_eq!(
        char::from_json(&Value::from("ab"))
            .expect_err("two")
            .to_string(),
        "invalid value: string \"ab\", expected a character"
    );
    assert!(char::from_json(&Value::from("")).is_err());
    assert_eq!(char::from_json(&Value::from("é")), Ok('é'));
}

#[test]
fn a_boolean_and_a_string_refuse_every_other_kind() {
    assert_eq!(
        bool::from_json(&doc("1")).expect_err("number").to_string(),
        "invalid type: integer `1`, expected a boolean"
    );
    assert_eq!(bool::from_json(&doc("true")), Ok(true));
    assert_eq!(
        String::from_json(&doc("{}"))
            .expect_err("object")
            .to_string(),
        "invalid type: map, expected a string"
    );
    assert_eq!(String::from_json(&doc("\"x\"")), Ok("x".to_owned()));
}

/// A closed enum is its string spelling: a map spelling of a unit variant
/// is refused, and the string spelling beside it is read.
#[test]
fn a_closed_enum_is_exactly_its_spellings() {
    assert_eq!(
        Colour::from_json(&Value::from("dark-blue")),
        Ok(Colour::DarkBlue)
    );
    assert_eq!(Colour::DarkBlue.to_json(), Value::from("dark-blue"));
    assert_eq!(Colour::JSON_VARIANTS, ["red", "dark-blue"]);
    assert_eq!(
        Colour::from_json(&Value::from("blue"))
            .expect_err("unknown")
            .to_string(),
        "unknown variant `blue`, expected `red` or `dark-blue`"
    );
    assert_eq!(
        Colour::from_json(&doc(r#"{"red":null}"#))
            .expect_err("map spelling")
            .to_string(),
        "invalid type: map, expected enum Colour"
    );
    assert!(Colour::from_json(&doc(r#""red""#)).is_ok());
    let map: BTreeMap<Colour, u8> = from_slice(br#"{"dark-blue":2,"red":1}"#).expect("keyed map");
    assert_eq!(
        json::write_compact(&map.to_json()),
        r#"{"red":1,"dark-blue":2}"#
    );
}

#[test]
fn unknown_variant_and_field_lists_read_as_prose() {
    assert_eq!(
        DecodeError::unknown_variant("x", &[]).to_string(),
        "unknown variant `x`, there are no variants"
    );
    assert_eq!(
        DecodeError::unknown_field("x", &["a"]).to_string(),
        "unknown field `x`, expected `a`"
    );
    assert_eq!(
        DecodeError::unknown_field("x", &["a", "b", "c"]).to_string(),
        "unknown field `x`, expected one of `a`, `b`, `c`"
    );
}

// ── Documents and free-form values ─────────────────────────────────────────

#[test]
fn a_syntax_error_is_a_decode_error_and_a_well_formed_document_reads() {
    let error = from_slice::<Pair>(b"{").expect_err("truncated");
    assert!(error.message().starts_with("JSON byte "), "{error}");
    assert!(from_slice::<Pair>(br#"{"name":"a"}"#).is_ok());
}

#[test]
fn free_form_values_are_sorted_and_a_repeat_keeps_its_last_value() {
    let value =
        read_document(br#"{"b":{"y":1,"x":[{"k":1,"k":2}]},"a":0,"b":3}"#).expect("document");
    assert_eq!(json::write_compact(&value), r#"{"a":0,"b":3}"#);
    let value =
        read_document(br#"{"b":{"y":1,"x":[{"k":1,"j":2,"k":3}]},"a":0}"#).expect("document");
    assert_eq!(
        json::write_compact(&value),
        r#"{"a":0,"b":{"x":[{"j":2,"k":3}],"y":1}}"#
    );
    let mut value = doc(r#"{"z":1,"a":[{"d":1,"c":2}]}"#);
    sorted_last_wins(&mut value);
    assert_eq!(
        Value::from_json(&doc(r#"{"z":1,"a":[{"d":1,"c":2}]}"#)),
        Ok(value)
    );
}

#[test]
fn maps_keep_the_last_repeat_and_sets_collapse_repeats() {
    let map: BTreeMap<String, u8> = from_slice(br#"{"a":1,"a":2}"#).expect("map");
    assert_eq!(map.get("a"), Some(&2));
    let set: BTreeSet<String> = from_slice(br#"["b","a","b"]"#).expect("set");
    assert_eq!(set.len(), 2);
    assert_eq!(
        from_slice::<BTreeSet<String>>(br#"{"a":1}"#)
            .expect_err("map")
            .to_string(),
        "invalid type: map, expected a sequence"
    );
}

#[test]
fn a_value_moves_out_by_kind_and_back() {
    for text in ["null", "true", "1.5", "\"s\"", "[1]", r#"{"a":1}"#] {
        let value = doc(text);
        assert_eq!(Value::from(into_owned(value.clone())), value);
    }
}

#[test]
fn tuples_and_options_write_as_arrays_and_null() {
    assert_eq!(
        json::write_compact(&("a", 1_u8, true).to_json()),
        r#"["a",1,true]"#
    );
    assert_eq!(None::<u8>.to_json(), Value::Null);
    assert_eq!(
        json::write_compact(&vec![1.5_f64, f64::NAN].to_json()),
        "[1.5,null]"
    );
}
