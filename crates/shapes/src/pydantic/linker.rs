// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Crate-private attributed package linker for routed Pydantic emission.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Map, Value};

use super::{
    PydanticClassConfig, PydanticError, PydanticPackageTopology, reference_key,
    schema_array_keywords, schema_map_keywords, schema_single_keywords,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct LinkedHelper {
    pub(super) name: String,
    pub(super) source: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct LinkedDefinition {
    pub(super) key: String,
    pub(super) class_name: String,
    pub(super) config: PydanticClassConfig,
    pub(super) source: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct LinkedModule {
    pub(super) path: String,
    pub(super) docstring: String,
    pub(super) artifact_path: String,
    pub(super) namespace_alias: String,
    pub(super) definitions: BTreeMap<String, LinkedDefinition>,
    pub(super) dependencies: BTreeMap<String, BTreeSet<String>>,
    pub(super) helpers: Vec<LinkedHelper>,
    pub(super) public_symbols: BTreeSet<String>,
    pub(super) private_symbols: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RoutedPackagePlan {
    pub(super) modules: BTreeMap<String, LinkedModule>,
    pub(super) model_paths: BTreeMap<String, String>,
    pub(super) artifact_paths: BTreeSet<String>,
    pub(super) intermediate_packages: BTreeSet<String>,
    symbol_owners: BTreeMap<String, String>,
}

impl RoutedPackagePlan {
    pub(super) fn compile(
        definitions: &Map<String, Value>,
        names: &BTreeMap<String, String>,
        topology: &PydanticPackageTopology,
        package_name: &str,
        include_version: bool,
    ) -> Result<Self, PydanticError> {
        let schema_keys = definitions.keys().cloned().collect::<BTreeSet<_>>();
        let route_keys = topology.classes().keys().cloned().collect::<BTreeSet<_>>();
        let missing = schema_keys
            .difference(&route_keys)
            .cloned()
            .collect::<Vec<_>>();
        let stale = route_keys
            .difference(&schema_keys)
            .cloned()
            .collect::<Vec<_>>();
        if !missing.is_empty() || !stale.is_empty() {
            return Err(PydanticError::new(format!(
                "Pydantic topology must cover $defs exactly; missing routes={missing:?}, stale \
                 routes={stale:?}"
            )));
        }

        let mut modules = BTreeMap::new();
        let mut artifact_paths = BTreeSet::from([
            "_base.py".to_owned(),
            "_schema.py".to_owned(),
            "__init__.py".to_owned(),
            "py.typed".to_owned(),
        ]);
        if include_version {
            artifact_paths.insert("__about__.py".to_owned());
        }
        let mut intermediate_packages = BTreeSet::new();
        let mut current_path = String::new();
        for (index, module) in topology.modules().values().enumerate() {
            current_path.clear();
            current_path.reserve(module.path().len());
            for byte in module.path().bytes() {
                if byte == b'.' {
                    let path = format!("{current_path}/__init__.py");
                    artifact_paths.insert(path.clone());
                    intermediate_packages.insert(path);
                    current_path.push('/');
                } else {
                    current_path.push(char::from(byte));
                }
            }
            let artifact_path = format!("{current_path}.py");
            if !artifact_paths.insert(artifact_path.clone()) {
                return Err(PydanticError::new(format!(
                    "Pydantic linker artifact path {artifact_path:?} is not unique"
                )));
            }
            modules.insert(
                module.path().to_owned(),
                LinkedModule {
                    path: module.path().to_owned(),
                    docstring: module.docstring().to_owned(),
                    artifact_path,
                    namespace_alias: format!("_PURRDF_TYPES_{index}"),
                    definitions: BTreeMap::new(),
                    dependencies: BTreeMap::new(),
                    helpers: Vec::new(),
                    public_symbols: BTreeSet::new(),
                    private_symbols: BTreeSet::new(),
                },
            );
        }

        let mut model_paths = BTreeMap::new();
        let mut symbol_owners = BTreeMap::new();
        for (key, class_config) in topology.classes() {
            let class_name = names.get(key).ok_or_else(|| {
                PydanticError::new(format!(
                    "Pydantic linker has no normalized class name for $defs key {key:?}"
                ))
            })?;
            let module = modules.get_mut(class_config.module_path()).ok_or_else(|| {
                PydanticError::new(format!(
                    "Pydantic linker has no declared module {:?} for $defs key {key:?}",
                    class_config.module_path()
                ))
            })?;
            if !module.public_symbols.insert(class_name.clone()) {
                return Err(PydanticError::new(format!(
                    "Pydantic module {:?} exports class {class_name:?} more than once",
                    module.path
                )));
            }
            if let Some(previous) = symbol_owners.insert(class_name.clone(), key.clone()) {
                return Err(PydanticError::new(format!(
                    "Pydantic definitions {previous:?} and {key:?} both own symbol \
                     {class_name:?}"
                )));
            }
            module.definitions.insert(
                key.clone(),
                LinkedDefinition {
                    key: key.clone(),
                    class_name: class_name.clone(),
                    config: class_config.clone(),
                    source: None,
                },
            );
            model_paths.insert(
                key.clone(),
                format!(
                    "{package_name}.{}.{}",
                    class_config.module_path(),
                    class_name
                ),
            );
        }

        for (key, definition) in definitions {
            let source_config = topology
                .classes()
                .get(key)
                .expect("exact topology coverage was validated");
            let mut references = BTreeSet::new();
            collect_references(definition, &mut references)?;
            for target_key in references {
                let target_config = topology.classes().get(&target_key).ok_or_else(|| {
                    PydanticError::new(format!(
                        "Pydantic $defs key {key:?} references missing routed definition \
                         {target_key:?}"
                    ))
                })?;
                if target_config.module_path() == source_config.module_path() {
                    continue;
                }
                let target_name = names.get(&target_key).ok_or_else(|| {
                    PydanticError::new(format!(
                        "Pydantic linker has no class name for referenced $defs key {target_key:?}"
                    ))
                })?;
                modules
                    .get_mut(source_config.module_path())
                    .expect("topology module exists")
                    .dependencies
                    .entry(target_config.module_path().to_owned())
                    .or_default()
                    .insert(target_name.clone());
            }
        }

        Ok(Self {
            modules,
            model_paths,
            artifact_paths,
            intermediate_packages,
            symbol_owners,
        })
    }

    pub(super) fn attach_definition(
        &mut self,
        key: &str,
        source: String,
        helpers: Vec<LinkedHelper>,
        private_symbols: BTreeSet<String>,
    ) -> Result<(), PydanticError> {
        let module_path = self
            .modules
            .values()
            .find_map(|module| {
                module
                    .definitions
                    .contains_key(key)
                    .then(|| module.path.clone())
            })
            .ok_or_else(|| {
                PydanticError::new(format!(
                    "Pydantic linker cannot attach unknown $defs key {key:?}"
                ))
            })?;

        if self.modules[&module_path].definitions[key].source.is_some() {
            return Err(PydanticError::new(format!(
                "Pydantic linker attached $defs key {key:?} more than once"
            )));
        }

        let mut new_symbols = BTreeSet::new();
        for helper in &helpers {
            if !new_symbols.insert(helper.name.clone()) {
                return Err(PydanticError::new(format!(
                    "Pydantic definition {key:?} declares generated symbol {:?} more than once",
                    helper.name
                )));
            }
        }
        for symbol in &private_symbols {
            if !new_symbols.insert(symbol.clone()) {
                return Err(PydanticError::new(format!(
                    "Pydantic definition {key:?} declares generated symbol {symbol:?} more than \
                     once"
                )));
            }
        }
        for symbol in &new_symbols {
            if let Some(previous) = self.symbol_owners.get(symbol) {
                return Err(PydanticError::new(format!(
                    "Pydantic definitions {previous:?} and {key:?} both own generated symbol \
                     {symbol:?}"
                )));
            }
        }
        for symbol in &new_symbols {
            self.symbol_owners.insert(symbol.clone(), key.to_owned());
        }

        let module = self
            .modules
            .get_mut(&module_path)
            .expect("located module remains present");
        module
            .definitions
            .get_mut(key)
            .expect("located definition remains present")
            .source = Some(source);
        for helper in helpers {
            if !module.private_symbols.insert(helper.name.clone()) {
                return Err(PydanticError::new(format!(
                    "Pydantic module {module_path:?} declares private helper {:?} more than once",
                    helper.name
                )));
            }
            module.helpers.push(helper);
        }
        for symbol in private_symbols {
            if !module.private_symbols.insert(symbol.clone()) {
                return Err(PydanticError::new(format!(
                    "Pydantic module {module_path:?} declares private symbol {symbol:?} more than \
                     once"
                )));
            }
        }
        Ok(())
    }

    pub(super) fn validate_complete(&self) -> Result<(), PydanticError> {
        for module in self.modules.values() {
            if module.definitions.is_empty() {
                return Err(PydanticError::new(format!(
                    "Pydantic linked leaf module {:?} has no definitions",
                    module.path
                )));
            }
            for definition in module.definitions.values() {
                if definition.config.definition_key() != definition.key
                    || definition.config.module_path() != module.path
                    || !module.public_symbols.contains(&definition.class_name)
                {
                    return Err(PydanticError::new(format!(
                        "Pydantic linker attribution drifted for $defs key {:?}",
                        definition.key
                    )));
                }
                if definition.source.is_none() {
                    return Err(PydanticError::new(format!(
                        "Pydantic linker has no rendered source for $defs key {:?}",
                        definition.key
                    )));
                }
            }
            for symbols in module.dependencies.values() {
                for symbol in symbols {
                    if !self.symbol_owners.contains_key(symbol) {
                        return Err(PydanticError::new(format!(
                            "Pydantic linker dependency symbol {symbol:?} has no owner"
                        )));
                    }
                }
            }
        }
        Ok(())
    }
}

pub(super) fn relative_root_import(module_path: &str, target: &str) -> String {
    let depth = module_path.bytes().filter(|byte| *byte == b'.').count() + 1;
    let mut import = String::with_capacity(depth + target.len());
    for _ in 0..depth {
        import.push('.');
    }
    import.push_str(target);
    import
}

fn collect_references(
    value: &Value,
    references: &mut BTreeSet<String>,
) -> Result<(), PydanticError> {
    let Value::Object(object) = value else {
        return Ok(());
    };
    if let Some(reference) = object.get("$ref") {
        let reference = reference.as_str().ok_or_else(|| {
            PydanticError::new("Pydantic linker encountered a non-string schema $ref")
        })?;
        let key = reference_key(reference).ok_or_else(|| {
            PydanticError::new(format!(
                "Pydantic linker cannot route external/non-$defs reference {reference:?}"
            ))
        })?;
        references.insert(key);
    }
    for keyword in schema_map_keywords() {
        if let Some(children) = object.get(*keyword).and_then(Value::as_object) {
            for child in children.values() {
                collect_references(child, references)?;
            }
        }
    }
    for keyword in schema_array_keywords() {
        if let Some(children) = object.get(*keyword).and_then(Value::as_array) {
            for child in children {
                collect_references(child, references)?;
            }
        }
    }
    for keyword in schema_single_keywords() {
        if let Some(child) = object.get(*keyword) {
            collect_references(child, references)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PydanticClassConfig, PydanticModuleConfig};
    use serde_json::json;

    fn topology() -> PydanticPackageTopology {
        PydanticPackageTopology::new(
            [
                PydanticModuleConfig::new("domain.people", "People module.").expect("module"),
                PydanticModuleConfig::new("vocab", "Vocabulary module.").expect("module"),
            ],
            [
                PydanticClassConfig::new(
                    "Person",
                    "domain.people",
                    "Person docs.",
                    BTreeMap::new(),
                )
                .expect("class"),
                PydanticClassConfig::new("Color", "vocab", "Color docs.", BTreeMap::new())
                    .expect("class"),
            ],
        )
        .expect("topology")
    }

    fn definitions() -> Map<String, Value> {
        json!({
            "Person": {
                "type": "object",
                "properties": {
                    "$ref": {"type": "string"},
                    "color": {"$ref": "#/$defs/Color"},
                    "data": {"enum": [{"$ref": "literal-data"}]}
                }
            },
            "Color": {"enum": ["red", "blue"]}
        })
        .as_object()
        .expect("object")
        .clone()
    }

    #[test]
    fn compiles_reference_edges_paths_and_intermediate_packages() {
        let definitions = definitions();
        let names = BTreeMap::from([
            ("Color".to_owned(), "Color".to_owned()),
            ("Person".to_owned(), "Person".to_owned()),
        ]);
        let plan =
            RoutedPackagePlan::compile(&definitions, &names, &topology(), "example_models", true)
                .expect("plan");
        assert_eq!(
            plan.model_paths["Person"],
            "example_models.domain.people.Person"
        );
        assert!(plan.artifact_paths.contains("domain/__init__.py"));
        assert!(plan.artifact_paths.contains("__about__.py"));
        assert_eq!(
            plan.modules["domain.people"].dependencies["vocab"],
            BTreeSet::from(["Color".to_owned()])
        );
        assert!(!plan.modules["domain.people"].dependencies["vocab"].contains("literal-data"));
    }

    #[test]
    fn exact_coverage_and_complete_attachments_fail_closed() {
        let definitions = definitions();
        let names = BTreeMap::from([
            ("Color".to_owned(), "Color".to_owned()),
            ("Person".to_owned(), "Person".to_owned()),
        ]);
        let mut missing = definitions.clone();
        missing.remove("Color");
        assert!(
            RoutedPackagePlan::compile(&missing, &names, &topology(), "example_models", false)
                .is_err()
        );

        let mut plan =
            RoutedPackagePlan::compile(&definitions, &names, &topology(), "example_models", false)
                .expect("plan");
        assert!(plan.validate_complete().is_err());
        plan.attach_definition(
            "Color",
            "class Color: pass\n".to_owned(),
            Vec::new(),
            BTreeSet::new(),
        )
        .expect("attach Color");
        plan.attach_definition(
            "Person",
            "class Person: pass\n".to_owned(),
            vec![LinkedHelper {
                name: "_PersonData".to_owned(),
                source: "_PersonData = object\n".to_owned(),
            }],
            BTreeSet::new(),
        )
        .expect("attach Person");
        plan.validate_complete().expect("complete");
        assert!(
            plan.attach_definition("Person", "again\n".to_owned(), Vec::new(), BTreeSet::new())
                .is_err()
        );
    }

    #[test]
    fn relative_imports_reach_the_package_root() {
        assert_eq!(relative_root_import("people", "vocab"), ".vocab");
        assert_eq!(
            relative_root_import("domain.people", "vocab.colors"),
            "..vocab.colors"
        );
    }
}
