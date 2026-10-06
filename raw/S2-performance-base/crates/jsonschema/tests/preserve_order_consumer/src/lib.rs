// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! An external consumer: `uniqueItems` over objects whose members the caller
//! wrote in different orders, built outside the PurRDF workspace's dependency
//! graph.

#[cfg(test)]
mod tests {
    use purrdf_jsonschema::{Metaschemas, Schema};
    use purrdf_lex::json::{self, Object, Value};

    #[test]
    fn unique_items_detects_reverse_key_order_above_hash_threshold() {
        let metaschemas = Metaschemas::new(
            purrdf_testkit::jsonschema_metaschemas::all()
                .map(|(uri, text)| (uri, json::read(text).expect("JSON meta-schema"))),
        )
        .expect("meta-schemas");
        let schema = Schema::from_document(
            &metaschemas,
            "https://example.org/unique.json",
            json::read(r#"{"uniqueItems": true}"#).expect("JSON"),
        )
        .expect("schema");

        let left = Object::new().with("a", 1_u8).with("b", 2_u8);
        let right = Object::new().with("b", 2_u8).with("a", 1_u8);
        assert_eq!(
            right.keys().next().map(String::as_str),
            Some("b"),
            "members keep the order they were written in"
        );
        let mut items: Vec<Value> = (0..17_u8)
            .map(|n| Value::from(Object::new().with("n", n)))
            .collect();
        items.push(Value::from(left.clone()));
        assert!(
            schema
                .is_valid(&Value::Array(items.clone()))
                .expect("evaluation")
        );
        items.push(Value::from(right));
        assert!(!schema.is_valid(&Value::Array(items)).expect("evaluation"));
    }
}
