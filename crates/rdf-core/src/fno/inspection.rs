// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Borrowed inspection of original rejected catalog data.

use super::{
    FnBlank, FnFunction, FnImpl, FnMapping, FnOutput, FnParam, FnParamMapping, FnReturnMapping,
    FnoCatalogInput,
};
use purrdf_hash::error::ErrorValue as V;

impl purrdf_hash::error::StructuredError for FnBlank {
    purrdf_hash::error_visit! {
        @record self { label, scope } => "fno-blank" {
            "label" => Text(&self.label), "scope" => Unsigned(u128::from(self.scope.ordinal())),
        }
    }
}

impl purrdf_hash::error::StructuredError for FnoCatalogInput {
    purrdf_hash::error_visit! {
        @record self { ontology_iri, document_iri, doc_label, banner, functions, params, implementations, mappings } => "fno-catalog-input" {
            "ontology_iri" => Iri(&self.ontology_iri), "document_iri" => Iri(&self.document_iri),
            "doc_label" => Term(&self.doc_label), "banner" => Term(&self.banner),
            "functions" => List(self.functions.iter().map(|item| V::Record(item))),
            "params" => List(self.params.iter().map(|item| V::Record(item))),
            "implementations" => List(self.implementations.iter().map(|item| V::Record(item))),
            "mappings" => List(self.mappings.iter().map(|item| V::Record(item))),
        }
    }
}

impl purrdf_hash::error::StructuredError for FnFunction {
    purrdf_hash::error_visit! {
        @record self { iri, label, description, kind_types, see_also, expects, output } => "fno-function" {
            "iri" => Iri(&self.iri), "label" => Term(&self.label),
            "description" => Value(self.description.as_ref().map_or(V::Absent, |value| V::Term(value))),
            "kind_types" => List(self.kind_types.iter().map(|value| V::Iri(value))),
            "see_also" => Value(self.see_also.as_deref().map_or(V::Absent, V::Iri)),
            "expects" => List(self.expects.iter().map(|value| V::Iri(value))), "output" => Record(&self.output),
        }
    }
}

impl purrdf_hash::error::StructuredError for FnOutput {
    purrdf_hash::error_visit! {
        @record self { iri, predicate, r#type, label, description } => "fno-output" {
            "iri" => Iri(&self.iri), "predicate" => Value(self.predicate.as_deref().map_or(V::Absent, V::Iri)),
            "type" => Iri(&self.r#type), "label" => Value(self.label.as_ref().map_or(V::Absent, |value| V::Term(value))),
            "description" => Value(self.description.as_ref().map_or(V::Absent, |value| V::Term(value))),
        }
    }
}

impl purrdf_hash::error::StructuredError for FnParam {
    purrdf_hash::error_visit! {
        @record self { iri, predicate, r#type, required, label, description } => "fno-param" {
            "iri" => Iri(&self.iri), "predicate" => Value(self.predicate.as_deref().map_or(V::Absent, V::Iri)),
            "type" => Iri(&self.r#type), "required" => Boolean(self.required),
            "label" => Value(self.label.as_ref().map_or(V::Absent, |value| V::Term(value))),
            "description" => Value(self.description.as_ref().map_or(V::Absent, |value| V::Term(value))),
        }
    }
}

impl purrdf_hash::error::StructuredError for FnImpl {
    purrdf_hash::error_visit! {
        @record self { iri, format, see_also } => "fno-implementation" {
            "iri" => Iri(&self.iri), "format" => Text(&self.format), "see_also" => Iri(&self.see_also),
        }
    }
}

impl purrdf_hash::error::StructuredError for FnMapping {
    purrdf_hash::error_visit! {
        @record self { node, label, function, implementation, parameter_mappings, return_mappings } => "fno-mapping" {
            "node" => Record(&self.node), "label" => Term(&self.label),
            "function" => Iri(&self.function), "implementation" => Iri(&self.implementation),
            "parameter_mappings" => List(self.parameter_mappings.iter().map(|item| V::Record(item))),
            "return_mappings" => List(self.return_mappings.iter().map(|item| V::Record(item))),
        }
    }
}

impl purrdf_hash::error::StructuredError for FnParamMapping {
    purrdf_hash::error_visit! {
        @record self { node, label, function_parameter, implementation_property } => "fno-param-mapping" {
            "node" => Record(&self.node), "label" => Term(&self.label),
            "function_parameter" => Iri(&self.function_parameter), "implementation_property" => Text(&self.implementation_property),
        }
    }
}

impl purrdf_hash::error::StructuredError for FnReturnMapping {
    purrdf_hash::error_visit! {
        @record self { node, label, function_output, implementation_property } => "fno-return-mapping" {
            "node" => Record(&self.node), "label" => Term(&self.label),
            "function_output" => Iri(&self.function_output), "implementation_property" => Text(&self.implementation_property),
        }
    }
}
