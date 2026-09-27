// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Consumer-side feature unification fixture.

#[cfg(test)]
mod tests {
    use purrdf_jsonschema::{Metaschemas, Schema};
    use serde_json::{Map, Value, json};

    #[test]
    fn unique_items_detects_reverse_key_order_above_hash_threshold() {
        let metaschemas = Metaschemas::new(purrdf_testkit::jsonschema_metaschemas::all().map(
            |(uri, text)| {
                (
                    uri,
                    serde_json::from_str::<Value>(text).expect("JSON meta-schema"),
                )
            },
        ))
        .expect("meta-schemas");
        let schema = Schema::from_document(
            &metaschemas,
            "https://example.org/unique.json",
            json!({"uniqueItems": true}),
        )
        .expect("schema");

        let mut left = Map::new();
        left.insert("a".to_owned(), json!(1));
        left.insert("b".to_owned(), json!(2));
        let mut right = Map::new();
        right.insert("b".to_owned(), json!(2));
        right.insert("a".to_owned(), json!(1));
        assert_eq!(
            right.keys().next().map(String::as_str),
            Some("b"),
            "preserve_order must be enabled"
        );
        let mut items: Vec<Value> = (0..17).map(|n| json!({"n": n})).collect();
        items.push(Value::Object(left));
        items.push(Value::Object(right));
        assert!(!schema.is_valid(&Value::Array(items)).expect("evaluation"));
    }
}
