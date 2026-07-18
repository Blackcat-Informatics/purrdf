// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Shared deterministic fixtures for Pydantic emission instruments.

use std::collections::BTreeMap;

use purrdf::loss::LossLedger;
use purrdf_shapes::json_schema::CompiledSchema;
use purrdf_shapes::{
    PydanticClassConfig, PydanticConfig, PydanticModuleConfig, PydanticPackage,
    PydanticPackageTopology, PydanticVersionStamp, emit_pydantic,
};
use serde_json::{Map, Value, json};

pub(crate) const SIZES: [usize; 3] = [32, 1_024, 16_384];
pub(crate) const MAXIMUM_DEFINITIONS: usize = 65_536;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Mode {
    Flat,
    RichSingleModule,
    GroupedFortyModules,
    HighFanout,
}

impl Mode {
    pub(crate) const ALL: [Self; 4] = [
        Self::Flat,
        Self::RichSingleModule,
        Self::GroupedFortyModules,
        Self::HighFanout,
    ];

    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Flat => "flat",
            Self::RichSingleModule => "rich_single_module",
            Self::GroupedFortyModules => "grouped_40_modules",
            Self::HighFanout => "high_fanout",
        }
    }
}

pub(crate) struct Fixture {
    pub(crate) definitions: usize,
    pub(crate) mode: Mode,
    compiled: CompiledSchema,
    config: PydanticConfig,
}

impl Fixture {
    pub(crate) fn new(definitions: usize, mode: Mode) -> Self {
        assert!(definitions >= 4, "fixture needs every schema shape");
        Self::from_parts(
            definitions,
            mode,
            compiled_schema(definitions),
            config(definitions, mode),
        )
    }

    pub(crate) fn maximum_high_fanout() -> Self {
        Self::from_parts(
            MAXIMUM_DEFINITIONS,
            Mode::HighFanout,
            compact_high_fanout_schema(MAXIMUM_DEFINITIONS),
            compact_high_fanout_config(MAXIMUM_DEFINITIONS),
        )
    }

    fn from_parts(
        definitions: usize,
        mode: Mode,
        compiled: CompiledSchema,
        config: PydanticConfig,
    ) -> Self {
        let fixture = Self {
            definitions,
            mode,
            compiled,
            config,
        };
        let package = fixture.emit();
        assert_eq!(package.model_paths.len(), definitions);
        assert_eq!(package.source_schema_json(), fixture.compiled.schema_json);
        match mode {
            Mode::Flat => assert_eq!(package.artifacts.len(), 4),
            Mode::RichSingleModule => assert_eq!(package.artifacts.len(), 6),
            Mode::GroupedFortyModules => {
                assert_eq!(package.artifacts.len(), definitions.min(40) + 6);
            }
            Mode::HighFanout => assert_eq!(package.artifacts.len(), definitions + 6),
        }
        fixture
    }

    pub(crate) fn emit(&self) -> PydanticPackage {
        emit_pydantic(&self.compiled, &self.config).expect("validated benchmark fixture emits")
    }
}

fn compact_high_fanout_schema(definitions: usize) -> CompiledSchema {
    let mut defs = Map::new();
    for index in 0..definitions {
        defs.insert(
            definition_name(index),
            if index == 3 {
                high_fanout_definition(definitions)
            } else {
                json!({})
            },
        );
    }
    compiled(defs)
}

fn compiled_schema(definitions: usize) -> CompiledSchema {
    let mut defs = Map::new();
    for index in 0..definitions {
        let name = definition_name(index);
        let next = definition_name((index + 1) % definitions);
        let definition = if index == 3 {
            high_fanout_definition(definitions)
        } else {
            match index % 4 {
                0 => json!({
                    "type": "object",
                    "additionalProperties": false,
                    "properties": {
                        "inline": {
                            "type": "object",
                            "additionalProperties": false,
                            "properties": {
                                "label": { "type": "string", "minLength": 1 }
                            },
                            "required": ["label"]
                        },
                        "next": { "$ref": format!("#/$defs/{next}") },
                        "self": { "$ref": format!("#/$defs/{name}") }
                    }
                }),
                1 => json!({
                    "enum": [format!("value:{index}:a"), format!("value:{index}:b")]
                }),
                2 => json!({ "$ref": format!("#/$defs/{next}") }),
                _ => json!({
                    "type": "object",
                    "properties": {
                        "members": {
                            "type": "array",
                            "items": { "$ref": format!("#/$defs/{next}") },
                            "minItems": 1
                        },
                        "name": { "type": "string", "pattern": "^[A-Z]" }
                    },
                    "required": ["name"]
                }),
            }
        };
        defs.insert(name, definition);
    }
    compiled(defs)
}

fn compiled(defs: Map<String, Value>) -> CompiledSchema {
    let schema = json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$defs": Value::Object(defs),
    });
    CompiledSchema {
        schema_json: serde_json::to_string(&schema).expect("benchmark schema serializes"),
        openapi_json: "{}\n".to_owned(),
        losses: LossLedger::new(),
    }
}

fn compact_high_fanout_config(definitions: usize) -> PydanticConfig {
    let base = PydanticConfig::new(
        "benchmark_models",
        "Caller-owned benchmark package documentation.",
        "Caller-owned benchmark support documentation.",
    )
    .expect("benchmark base config");
    let modules = (0..definitions).map(|index| {
        PydanticModuleConfig::new(compact_module_path(index), "d").expect("benchmark module")
    });
    let classes = (0..definitions).map(|index| {
        PydanticClassConfig::new(
            definition_name(index),
            compact_module_path(index),
            "d",
            BTreeMap::new(),
        )
        .expect("benchmark class route")
    });
    base.with_topology(PydanticPackageTopology::new(modules, classes).expect("benchmark topology"))
        .expect("benchmark topology config")
        .with_version_stamp(
            PydanticVersionStamp::new(
                "1.2.3+benchmark.1",
                "Caller-owned benchmark version documentation.",
            )
            .expect("benchmark version"),
        )
        .expect("benchmark version config")
}

fn high_fanout_definition(definitions: usize) -> Value {
    let properties = (0..definitions)
        .map(|index| {
            let target = definition_name(index);
            (
                format!("target_{index:05}"),
                json!({ "$ref": format!("#/$defs/{target}") }),
            )
        })
        .collect::<Map<_, _>>();
    json!({
        "type": "object",
        "additionalProperties": false,
        "properties": properties,
    })
}

fn config(definitions: usize, mode: Mode) -> PydanticConfig {
    let base = PydanticConfig::new(
        "benchmark_models",
        "Caller-owned benchmark package documentation.",
        "Caller-owned benchmark support documentation.",
    )
    .expect("benchmark base config");
    if mode == Mode::Flat {
        return base;
    }

    let module_count = match mode {
        Mode::Flat => unreachable!("flat returned above"),
        Mode::RichSingleModule => 1,
        Mode::GroupedFortyModules => definitions.min(40),
        Mode::HighFanout => definitions,
    };
    let modules = (0..module_count).map(|module| {
        let path = module_path(mode, module);
        PydanticModuleConfig::new(
            path.clone(),
            format!("Caller-owned benchmark module {path}."),
        )
        .expect("benchmark module")
    });
    let classes = (0..definitions).map(|index| {
        let key = definition_name(index);
        PydanticClassConfig::new(
            key.clone(),
            module_path(mode, index % module_count),
            format!("Caller-owned documentation for {key}."),
            BTreeMap::from([
                (
                    "definitionDigest".to_owned(),
                    json!(format!("sha256:benchmark-{index:05}")),
                ),
                (
                    "docs".to_owned(),
                    json!(format!("https://example.org/docs/{key}")),
                ),
            ]),
        )
        .expect("benchmark class route")
    });
    base.with_topology(PydanticPackageTopology::new(modules, classes).expect("benchmark topology"))
        .expect("benchmark topology config")
        .with_version_stamp(
            PydanticVersionStamp::new(
                "1.2.3+benchmark.1",
                "Caller-owned benchmark version documentation.",
            )
            .expect("benchmark version"),
        )
        .expect("benchmark version config")
}

fn definition_name(index: usize) -> String {
    format!("Model{index:05}")
}

fn module_path(mode: Mode, module: usize) -> String {
    match mode {
        Mode::Flat => unreachable!("flat packages have no topology"),
        Mode::RichSingleModule => "models".to_owned(),
        Mode::GroupedFortyModules => format!("group.module_{module:03}"),
        Mode::HighFanout => format!("fanout.module_{module:05}"),
    }
}

fn compact_module_path(module: usize) -> String {
    format!("m.m{module:05}")
}
