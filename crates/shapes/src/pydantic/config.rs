// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Validated caller-owned configuration for deterministic Pydantic packages.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use super::{PydanticError, is_python_identifier, is_python_keyword};

pub(super) const MAX_DEFINITIONS: usize = 65_536;
const MAX_CLASSES: usize = MAX_DEFINITIONS;
const MAX_MODULES: usize = 65_536;
const MAX_DOTTED_PATH_BYTES: usize = 255;
const MAX_DOTTED_PATH_COMPONENTS: usize = 32;
const MAX_ARTIFACT_PATH_BYTES: usize = 4_096;
const MAX_DOCSTRING_BYTES: usize = 1024 * 1024;
const MAX_VERSION_BYTES: usize = 512;
const MAX_CONFIG_BYTES: usize = 16 * 1024 * 1024;
const MAX_METADATA_DEPTH: usize = 128;
const MAX_METADATA_NODES: usize = 1_000_000;
const MAX_ARTIFACTS: usize = 131_072;
pub(super) const MAX_SCHEMA_BYTES: usize = 16 * 1024 * 1024;
pub(super) const MAX_SCHEMA_DEPTH: usize = 128;
pub(super) const MAX_SCHEMA_NODES: usize = 1_000_000;
pub(super) const MAX_SCHEMA_STRING_BYTES: usize = 16 * 1024 * 1024;
pub(super) const MAX_ARTIFACT_BYTES: usize = 256 * 1024 * 1024;
pub(super) const MAX_OUTPUT_BYTES: usize = 512 * 1024 * 1024;
pub(super) const MAX_OUTPUT_ARTIFACTS: usize = MAX_ARTIFACTS;

const GENERATED_ROOT_MODULES: &[&str] = &["_base", "_schema", "__about__", "__init__"];

/// One caller-declared Python module in a routed Pydantic package.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PydanticModuleConfig {
    path: String,
    docstring: String,
}

impl PydanticModuleConfig {
    /// Validate a relative dotted Python module path and its caller documentation.
    ///
    /// # Errors
    ///
    /// Returns [`PydanticError`] when the path is not portable Python syntax,
    /// collides with a generated root module, or the documentation is invalid.
    pub fn new(
        path: impl Into<String>,
        docstring: impl Into<String>,
    ) -> Result<Self, PydanticError> {
        let path = path.into();
        let docstring = docstring.into();
        validate_dotted_path(&path, "Pydantic module path")?;
        let root = path
            .split('.')
            .next()
            .expect("a validated dotted path has one component");
        if GENERATED_ROOT_MODULES
            .iter()
            .any(|generated| root.eq_ignore_ascii_case(generated))
        {
            return Err(PydanticError::new(format!(
                "Pydantic module path {path:?} collides with generated root module {root:?}"
            )));
        }
        validate_docstring(&docstring, "Pydantic module docstring")?;
        Ok(Self { path, docstring })
    }

    /// Relative dotted module path within the configured package.
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Caller-supplied module docstring.
    #[must_use]
    pub fn docstring(&self) -> &str {
        &self.docstring
    }
}

/// One source `$defs` entry's caller-owned route and class metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PydanticClassConfig {
    definition_key: String,
    module_path: String,
    docstring: String,
    json_schema_extra: BTreeMap<String, Value>,
    resource_bytes: usize,
    metadata_nodes: usize,
}

impl PydanticClassConfig {
    /// Validate one definition route, class docstring, and deterministic metadata.
    ///
    /// `json_schema_extra` is deliberately vocabulary-neutral. PurRDF preserves
    /// the caller's sorted JSON map without assigning meaning to any key.
    ///
    /// # Errors
    ///
    /// Returns [`PydanticError`] for an invalid module path, blank or oversized
    /// documentation, or metadata beyond the fixed depth, node, or byte ceilings.
    pub fn new(
        definition_key: impl Into<String>,
        module_path: impl Into<String>,
        docstring: impl Into<String>,
        json_schema_extra: BTreeMap<String, Value>,
    ) -> Result<Self, PydanticError> {
        let definition_key = definition_key.into();
        let module_path = module_path.into();
        let docstring = docstring.into();
        validate_dotted_path(&module_path, "Pydantic class module path")?;
        validate_docstring(&docstring, "Pydantic class docstring")?;

        let (metadata_bytes, metadata_nodes) =
            measure_metadata(&json_schema_extra, &definition_key)?;
        let resource_bytes = checked_sum(
            [
                definition_key.len(),
                module_path.len(),
                docstring.len(),
                metadata_bytes,
            ],
            "Pydantic class configuration byte count",
        )?;
        if resource_bytes > MAX_CONFIG_BYTES {
            return Err(PydanticError::new(format!(
                "Pydantic class route {definition_key:?} uses {resource_bytes} configuration \
                 bytes; limit is {MAX_CONFIG_BYTES}"
            )));
        }

        Ok(Self {
            definition_key,
            module_path,
            docstring,
            json_schema_extra,
            resource_bytes,
            metadata_nodes,
        })
    }

    /// Exact source `$defs` key routed by this declaration.
    #[must_use]
    pub fn definition_key(&self) -> &str {
        &self.definition_key
    }

    /// Relative dotted module path for the generated class.
    #[must_use]
    pub fn module_path(&self) -> &str {
        &self.module_path
    }

    /// Caller-supplied generated-class docstring.
    #[must_use]
    pub fn docstring(&self) -> &str {
        &self.docstring
    }

    /// Sorted caller-owned Pydantic `json_schema_extra` map.
    #[must_use]
    pub fn json_schema_extra(&self) -> &BTreeMap<String, Value> {
        &self.json_schema_extra
    }
}

/// A total, deterministic caller-owned partition of schema definitions into modules.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PydanticPackageTopology {
    modules: BTreeMap<String, PydanticModuleConfig>,
    classes: BTreeMap<String, PydanticClassConfig>,
    resource_bytes: usize,
    metadata_nodes: usize,
}

impl PydanticPackageTopology {
    /// Validate and canonicalize module declarations and definition routes.
    ///
    /// Constructors accept iterators so duplicate declarations are diagnosed
    /// before storage in sorted maps. Every declared leaf module must own at
    /// least one class, and every class must name a declared module. Exact
    /// coverage of a concrete schema's `$defs` is checked by the emitter.
    ///
    /// # Errors
    ///
    /// Returns [`PydanticError`] for duplicates, missing/unused modules,
    /// case-folded or file/package collisions, or resource-limit exhaustion.
    pub fn new(
        modules: impl IntoIterator<Item = PydanticModuleConfig>,
        classes: impl IntoIterator<Item = PydanticClassConfig>,
    ) -> Result<Self, PydanticError> {
        let mut module_map = BTreeMap::new();
        let mut folded_modules = BTreeMap::<String, String>::new();
        for module in modules {
            if module_map.len() == MAX_MODULES {
                return Err(PydanticError::new(format!(
                    "Pydantic topology exceeds the {MAX_MODULES}-module limit"
                )));
            }
            let path = module.path.clone();
            if module_map.insert(path.clone(), module).is_some() {
                return Err(PydanticError::new(format!(
                    "Pydantic topology declares module {path:?} more than once"
                )));
            }
            let folded = path.to_ascii_lowercase();
            if let Some(previous) = folded_modules.insert(folded, path.clone()) {
                return Err(PydanticError::new(format!(
                    "Pydantic modules {previous:?} and {path:?} collide on a \
                     case-insensitive filesystem"
                )));
            }
        }
        validate_module_prefixes(&module_map)?;

        let mut class_map = BTreeMap::new();
        let mut used_modules = BTreeSet::new();
        for class in classes {
            if class_map.len() == MAX_CLASSES {
                return Err(PydanticError::new(format!(
                    "Pydantic topology exceeds the {MAX_CLASSES}-class limit"
                )));
            }
            if !module_map.contains_key(class.module_path()) {
                return Err(PydanticError::new(format!(
                    "Pydantic class route {:?} names undeclared module {:?}",
                    class.definition_key(),
                    class.module_path()
                )));
            }
            used_modules.insert(class.module_path.clone());
            let key = class.definition_key.clone();
            if class_map.insert(key.clone(), class).is_some() {
                return Err(PydanticError::new(format!(
                    "Pydantic topology routes $defs key {key:?} more than once"
                )));
            }
        }
        if let Some(unused) = module_map.keys().find(|path| !used_modules.contains(*path)) {
            return Err(PydanticError::new(format!(
                "Pydantic topology declares leaf module {unused:?} with no routed class"
            )));
        }

        let mut resource_bytes = 0usize;
        for module in module_map.values() {
            resource_bytes = checked_add(
                resource_bytes,
                checked_add(
                    module.path.len(),
                    module.docstring.len(),
                    "Pydantic module configuration byte count",
                )?,
                "Pydantic topology byte count",
            )?;
        }
        let mut metadata_nodes = 0usize;
        for class in class_map.values() {
            resource_bytes = checked_add(
                resource_bytes,
                class.resource_bytes,
                "Pydantic topology byte count",
            )?;
            metadata_nodes = checked_add(
                metadata_nodes,
                class.metadata_nodes,
                "Pydantic topology metadata node count",
            )?;
        }
        if resource_bytes > MAX_CONFIG_BYTES {
            return Err(PydanticError::new(format!(
                "Pydantic topology uses {resource_bytes} configuration bytes; limit is \
                 {MAX_CONFIG_BYTES}"
            )));
        }
        if metadata_nodes > MAX_METADATA_NODES {
            return Err(PydanticError::new(format!(
                "Pydantic topology contains {metadata_nodes} metadata nodes; limit is \
                 {MAX_METADATA_NODES}"
            )));
        }

        validate_relative_artifacts(Some(&module_map), false)?;
        Ok(Self {
            modules: module_map,
            classes: class_map,
            resource_bytes,
            metadata_nodes,
        })
    }

    /// Sorted relative module path to declaration map.
    #[must_use]
    pub fn modules(&self) -> &BTreeMap<String, PydanticModuleConfig> {
        &self.modules
    }

    /// Sorted source `$defs` key to class route map.
    #[must_use]
    pub fn classes(&self) -> &BTreeMap<String, PydanticClassConfig> {
        &self.classes
    }
}

/// A caller-owned version source for an emitted Pydantic package.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PydanticVersionStamp {
    version: String,
    module_docstring: String,
    local: bool,
}

impl PydanticVersionStamp {
    /// Validate an exact PEP 440 package version and `__about__.py` docstring.
    ///
    /// The version is retained byte-for-byte, including accepted leading/trailing
    /// ASCII whitespace and spelling aliases. Consumers that require a normalized
    /// distribution identifier can apply their own release policy.
    ///
    /// # Errors
    ///
    /// Returns [`PydanticError`] when the version exceeds 512 ASCII bytes, does
    /// not match the complete PEP 440 grammar, or the docstring is invalid.
    pub fn new(
        version: impl Into<String>,
        module_docstring: impl Into<String>,
    ) -> Result<Self, PydanticError> {
        let version = version.into();
        let module_docstring = module_docstring.into();
        if version.len() > MAX_VERSION_BYTES {
            return Err(PydanticError::new(format!(
                "Pydantic package version uses {} bytes; limit is {MAX_VERSION_BYTES}",
                version.len()
            )));
        }
        if !version.is_ascii() {
            return Err(PydanticError::new(
                "Pydantic package version must contain only ASCII PEP 440 syntax",
            ));
        }
        let local = validate_pep440(&version).ok_or_else(|| {
            PydanticError::new(format!(
                "Pydantic package version {version:?} is not a PEP 440 version"
            ))
        })?;
        validate_docstring(&module_docstring, "Pydantic version-module docstring")?;
        Ok(Self {
            version,
            module_docstring,
            local,
        })
    }

    /// Exact caller-supplied PEP 440 version text.
    #[must_use]
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Caller-supplied `__about__.py` docstring.
    #[must_use]
    pub fn module_docstring(&self) -> &str {
        &self.module_docstring
    }

    /// Whether the version contains a PEP 440 local-version identifier.
    #[must_use]
    pub const fn is_local(&self) -> bool {
        self.local
    }
}

pub(super) fn validate_base_config(
    package_name: &str,
    package_docstring: &str,
    models_docstring: &str,
) -> Result<(), PydanticError> {
    validate_dotted_path(package_name, "Pydantic package name")?;
    validate_docstring(package_docstring, "Pydantic package docstring")?;
    validate_docstring(models_docstring, "Pydantic models-module docstring")?;
    validate_full_config(
        package_name,
        package_docstring,
        models_docstring,
        None,
        None,
    )
}

pub(super) fn validate_full_config(
    package_name: &str,
    package_docstring: &str,
    models_docstring: &str,
    topology: Option<&PydanticPackageTopology>,
    version: Option<&PydanticVersionStamp>,
) -> Result<(), PydanticError> {
    let mut config_bytes = checked_sum(
        [
            package_name.len(),
            package_docstring.len(),
            models_docstring.len(),
        ],
        "Pydantic configuration byte count",
    )?;
    if let Some(topology) = topology {
        config_bytes = checked_add(
            config_bytes,
            topology.resource_bytes,
            "Pydantic configuration byte count",
        )?;
        if topology.metadata_nodes > MAX_METADATA_NODES {
            return Err(PydanticError::new(format!(
                "Pydantic configuration contains {} metadata nodes; limit is \
                 {MAX_METADATA_NODES}",
                topology.metadata_nodes
            )));
        }
    }
    if let Some(version) = version {
        config_bytes = checked_sum(
            [
                config_bytes,
                version.version.len(),
                version.module_docstring.len(),
            ],
            "Pydantic configuration byte count",
        )?;
    }
    if config_bytes > MAX_CONFIG_BYTES {
        return Err(PydanticError::new(format!(
            "Pydantic configuration uses {config_bytes} bytes; limit is {MAX_CONFIG_BYTES}"
        )));
    }

    let relative =
        validate_relative_artifacts(topology.map(|value| &value.modules), version.is_some())?;
    let package_path = package_name.replace('.', "/");
    for path in &relative {
        let full_len = checked_sum(
            [package_path.len(), 1, path.len()],
            "Pydantic generated artifact path byte count",
        )?;
        if full_len > MAX_ARTIFACT_PATH_BYTES {
            return Err(PydanticError::new(format!(
                "Pydantic generated artifact path {package_path}/{path} uses {full_len} bytes; \
                 limit is {MAX_ARTIFACT_PATH_BYTES}"
            )));
        }
    }
    Ok(())
}

fn validate_docstring(value: &str, role: &str) -> Result<(), PydanticError> {
    if value.trim().is_empty() {
        return Err(PydanticError::new(format!(
            "{role} must be caller-supplied non-whitespace text"
        )));
    }
    if value.len() > MAX_DOCSTRING_BYTES {
        return Err(PydanticError::new(format!(
            "{role} uses {} bytes; limit is {MAX_DOCSTRING_BYTES}",
            value.len()
        )));
    }
    Ok(())
}

fn validate_dotted_path(value: &str, role: &str) -> Result<(), PydanticError> {
    if value.is_empty() || !value.is_ascii() {
        return Err(PydanticError::new(format!(
            "{role} {value:?} must be a non-empty dotted sequence of ASCII Python identifiers"
        )));
    }
    if value.len() > MAX_DOTTED_PATH_BYTES {
        return Err(PydanticError::new(format!(
            "{role} {value:?} uses {} bytes; limit is {MAX_DOTTED_PATH_BYTES}",
            value.len()
        )));
    }
    let components = value.split('.').collect::<Vec<_>>();
    if components.len() > MAX_DOTTED_PATH_COMPONENTS {
        return Err(PydanticError::new(format!(
            "{role} {value:?} has {} components; limit is {MAX_DOTTED_PATH_COMPONENTS}",
            components.len()
        )));
    }
    for component in components {
        if !is_python_identifier(component) || is_python_keyword(component) {
            return Err(PydanticError::new(format!(
                "{role} {value:?} contains invalid or keyword component {component:?}"
            )));
        }
        if is_windows_device_stem(component) {
            return Err(PydanticError::new(format!(
                "{role} {value:?} contains Windows-reserved device component {component:?}"
            )));
        }
    }
    Ok(())
}

fn is_windows_device_stem(component: &str) -> bool {
    let folded = component.to_ascii_lowercase();
    matches!(folded.as_str(), "con" | "prn" | "aux" | "nul")
        || folded.strip_prefix("com").is_some_and(|suffix| {
            matches!(suffix, "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9")
        })
        || folded.strip_prefix("lpt").is_some_and(|suffix| {
            matches!(suffix, "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9")
        })
}

fn validate_module_prefixes(
    modules: &BTreeMap<String, PydanticModuleConfig>,
) -> Result<(), PydanticError> {
    let folded = modules
        .keys()
        .map(|path| path.to_ascii_lowercase())
        .collect::<BTreeSet<_>>();
    for path in modules.keys() {
        let parts = path
            .split('.')
            .map(str::to_ascii_lowercase)
            .collect::<Vec<_>>();
        for length in 1..parts.len() {
            let prefix = parts[..length].join(".");
            if folded.contains(&prefix) {
                return Err(PydanticError::new(format!(
                    "Pydantic module path {path:?} requires {prefix:?} to be a package, but it \
                     is also declared as a module file"
                )));
            }
        }
    }
    Ok(())
}

fn validate_relative_artifacts(
    modules: Option<&BTreeMap<String, PydanticModuleConfig>>,
    include_version: bool,
) -> Result<BTreeSet<String>, PydanticError> {
    let mut paths = BTreeSet::new();
    let mut folded = BTreeMap::<String, String>::new();
    let generated = if modules.is_some() {
        ["_base.py", "_schema.py", "__init__.py", "py.typed"]
    } else {
        ["_base.py", "models.py", "__init__.py", "py.typed"]
    };
    for path in generated {
        insert_artifact(path.to_owned(), &mut paths, &mut folded)?;
    }
    if include_version {
        insert_artifact("__about__.py".to_owned(), &mut paths, &mut folded)?;
    }
    if let Some(modules) = modules {
        for module in modules.keys() {
            let parts = module.split('.').collect::<Vec<_>>();
            for length in 1..parts.len() {
                insert_artifact(
                    format!("{}/__init__.py", parts[..length].join("/")),
                    &mut paths,
                    &mut folded,
                )?;
            }
            insert_artifact(format!("{}.py", parts.join("/")), &mut paths, &mut folded)?;
        }
    }
    Ok(paths)
}

fn insert_artifact(
    path: String,
    paths: &mut BTreeSet<String>,
    folded: &mut BTreeMap<String, String>,
) -> Result<(), PydanticError> {
    if paths.contains(&path) {
        return Ok(());
    }
    if paths.len() == MAX_ARTIFACTS {
        return Err(PydanticError::new(format!(
            "Pydantic package exceeds the {MAX_ARTIFACTS}-artifact limit"
        )));
    }
    if path.len() > MAX_ARTIFACT_PATH_BYTES {
        return Err(PydanticError::new(format!(
            "Pydantic relative artifact path {path:?} uses {} bytes; limit is \
             {MAX_ARTIFACT_PATH_BYTES}",
            path.len()
        )));
    }
    let key = path.to_ascii_lowercase();
    if let Some(previous) = folded.insert(key, path.clone()) {
        return Err(PydanticError::new(format!(
            "Pydantic artifact paths {previous:?} and {path:?} collide on a \
             case-insensitive filesystem"
        )));
    }
    paths.insert(path);
    Ok(())
}

fn measure_metadata(
    metadata: &BTreeMap<String, Value>,
    definition_key: &str,
) -> Result<(usize, usize), PydanticError> {
    let mut bytes = 1usize;
    let mut nodes = 1usize;
    let mut stack = metadata
        .iter()
        .rev()
        .map(|(key, value)| (Some(key.as_str()), value, 1usize))
        .collect::<Vec<_>>();
    while let Some((key, value, depth)) = stack.pop() {
        if depth > MAX_METADATA_DEPTH {
            return Err(PydanticError::new(format!(
                "Pydantic class route {definition_key:?} metadata exceeds depth limit \
                 {MAX_METADATA_DEPTH}"
            )));
        }
        nodes = checked_add(nodes, 1, "Pydantic metadata node count")?;
        if nodes > MAX_METADATA_NODES {
            return Err(PydanticError::new(format!(
                "Pydantic class route {definition_key:?} metadata exceeds node limit \
                 {MAX_METADATA_NODES}"
            )));
        }
        if let Some(key) = key {
            bytes = checked_add(bytes, key.len(), "Pydantic metadata byte count")?;
        }
        bytes = checked_add(bytes, 1, "Pydantic metadata byte count")?;
        match value {
            Value::Null | Value::Bool(_) => {}
            Value::Number(number) => {
                bytes = checked_add(
                    bytes,
                    number.to_string().len(),
                    "Pydantic metadata byte count",
                )?;
            }
            Value::String(text) => {
                bytes = checked_add(bytes, text.len(), "Pydantic metadata byte count")?;
            }
            Value::Array(values) => {
                for child in values.iter().rev() {
                    stack.push((None, child, depth + 1));
                }
            }
            Value::Object(object) => {
                for (child_key, child) in object.iter().rev() {
                    stack.push((Some(child_key), child, depth + 1));
                }
            }
        }
        if bytes > MAX_CONFIG_BYTES {
            return Err(PydanticError::new(format!(
                "Pydantic class route {definition_key:?} metadata exceeds byte limit \
                 {MAX_CONFIG_BYTES}"
            )));
        }
    }
    Ok((bytes, nodes))
}

fn checked_sum(
    values: impl IntoIterator<Item = usize>,
    role: &str,
) -> Result<usize, PydanticError> {
    values
        .into_iter()
        .try_fold(0usize, |sum, value| checked_add(sum, value, role))
}

fn checked_add(left: usize, right: usize, role: &str) -> Result<usize, PydanticError> {
    left.checked_add(right)
        .ok_or_else(|| PydanticError::new(format!("{role} exceeds the platform usize range")))
}

/// Validate PEP 440 without normalizing or parsing numeric fields into integers.
///
/// Returns whether the accepted identifier contains a local-version segment.
fn validate_pep440(raw: &str) -> Option<bool> {
    let bytes = raw.as_bytes();
    let mut start = 0usize;
    let mut end = bytes.len();
    while start < end && bytes[start].is_ascii_whitespace() {
        start += 1;
    }
    while end > start && bytes[end - 1].is_ascii_whitespace() {
        end -= 1;
    }
    let bytes = &bytes[start..end];
    if bytes.is_empty() {
        return None;
    }
    let mut index = 0usize;

    if bytes
        .get(index)
        .is_some_and(|byte| byte.eq_ignore_ascii_case(&b'v'))
    {
        index += 1;
    }

    let epoch_start = index;
    consume_digits(bytes, &mut index);
    if bytes.get(index) == Some(&b'!') && index > epoch_start {
        index += 1;
    } else {
        index = epoch_start;
    }

    if !consume_digits(bytes, &mut index) {
        return None;
    }
    loop {
        let checkpoint = index;
        if bytes.get(index) == Some(&b'.') {
            index += 1;
            if consume_digits(bytes, &mut index) {
                continue;
            }
        }
        index = checkpoint;
        break;
    }

    let checkpoint = index;
    let mut candidate = index;
    consume_separator(bytes, &mut candidate);
    if consume_tag(
        bytes,
        &mut candidate,
        &["preview", "alpha", "beta", "pre", "rc", "a", "b", "c"],
    ) {
        consume_optional_number(bytes, &mut candidate);
        index = candidate;
    } else {
        index = checkpoint;
    }

    let checkpoint = index;
    let mut matched_post = false;
    if bytes.get(index) == Some(&b'-') {
        let mut candidate = index + 1;
        if consume_digits(bytes, &mut candidate) {
            index = candidate;
            matched_post = true;
        }
    }
    if !matched_post {
        let mut candidate = checkpoint;
        consume_separator(bytes, &mut candidate);
        if consume_tag(bytes, &mut candidate, &["post", "rev", "r"]) {
            consume_optional_number(bytes, &mut candidate);
            index = candidate;
        } else {
            index = checkpoint;
        }
    }

    let checkpoint = index;
    let mut candidate = index;
    consume_separator(bytes, &mut candidate);
    if consume_tag(bytes, &mut candidate, &["dev"]) {
        consume_optional_number(bytes, &mut candidate);
        index = candidate;
    } else {
        index = checkpoint;
    }

    let mut local = false;
    if bytes.get(index) == Some(&b'+') {
        local = true;
        index += 1;
        if !consume_ascii_alphanumeric(bytes, &mut index) {
            return None;
        }
        loop {
            let checkpoint = index;
            if bytes
                .get(index)
                .is_some_and(|byte| matches!(byte, b'-' | b'_' | b'.'))
            {
                index += 1;
                if consume_ascii_alphanumeric(bytes, &mut index) {
                    continue;
                }
            }
            index = checkpoint;
            break;
        }
    }

    (index == bytes.len()).then_some(local)
}

fn consume_digits(bytes: &[u8], index: &mut usize) -> bool {
    let start = *index;
    while bytes.get(*index).is_some_and(u8::is_ascii_digit) {
        *index += 1;
    }
    *index > start
}

fn consume_ascii_alphanumeric(bytes: &[u8], index: &mut usize) -> bool {
    let start = *index;
    while bytes.get(*index).is_some_and(u8::is_ascii_alphanumeric) {
        *index += 1;
    }
    *index > start
}

fn consume_separator(bytes: &[u8], index: &mut usize) {
    if bytes
        .get(*index)
        .is_some_and(|byte| matches!(byte, b'-' | b'_' | b'.'))
    {
        *index += 1;
    }
}

fn consume_optional_number(bytes: &[u8], index: &mut usize) {
    consume_separator(bytes, index);
    consume_digits(bytes, index);
}

fn consume_tag(bytes: &[u8], index: &mut usize, tags: &[&str]) -> bool {
    for tag in tags {
        let end = *index + tag.len();
        if bytes
            .get(*index..end)
            .is_some_and(|candidate| candidate.eq_ignore_ascii_case(tag.as_bytes()))
        {
            *index = end;
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PydanticConfig;
    use serde_json::json;

    fn module(path: &str) -> PydanticModuleConfig {
        PydanticModuleConfig::new(path, format!("Caller docs for {path}.")).expect("module")
    }

    fn class(key: &str, module: &str) -> PydanticClassConfig {
        PydanticClassConfig::new(
            key,
            module,
            format!("Caller docs for {key}."),
            BTreeMap::from([
                (
                    "definitionDigest".to_owned(),
                    json!(format!("sha256:{key}")),
                ),
                (
                    "docs".to_owned(),
                    json!(format!("https://example.org/docs/{key}")),
                ),
            ]),
        )
        .expect("class")
    }

    #[test]
    fn topology_is_total_over_declared_modules_and_canonical() {
        let topology = PydanticPackageTopology::new(
            [module("zeta"), module("alpha.people")],
            [class("Person", "alpha.people"), class("Thing", "zeta")],
        )
        .expect("topology");
        assert_eq!(
            topology
                .modules()
                .keys()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            ["alpha.people", "zeta"]
        );
        assert_eq!(
            topology
                .classes()
                .keys()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            ["Person", "Thing"]
        );
    }

    #[test]
    fn config_builders_retain_validated_topology_and_version() {
        let topology = PydanticPackageTopology::new(
            [module("domain.people")],
            [class("Person", "domain.people")],
        )
        .expect("topology");
        let version = PydanticVersionStamp::new(
            "2!3.4rc1+portable.7",
            "Caller-owned package version documentation.",
        )
        .expect("version");
        let config = PydanticConfig::new(
            "example_models",
            "Caller package documentation.",
            "Caller base-model documentation.",
        )
        .expect("base config")
        .with_version_stamp(version)
        .expect("version config")
        .with_topology(topology)
        .expect("topology config");

        assert_eq!(
            config.topology().expect("topology").classes()["Person"].json_schema_extra()["definitionDigest"],
            json!("sha256:Person")
        );
        assert_eq!(
            config.version_stamp().expect("version").version(),
            "2!3.4rc1+portable.7"
        );
        assert!(config.version_stamp().expect("version").is_local());
    }

    #[test]
    fn topology_rejects_duplicate_unknown_unused_and_prefix_routes() {
        assert!(PydanticPackageTopology::new([module("a"), module("a")], []).is_err());
        assert!(PydanticPackageTopology::new([module("a")], [class("A", "b")]).is_err());
        assert!(
            PydanticPackageTopology::new([module("a"), module("b")], [class("A", "a")]).is_err()
        );
        assert!(
            PydanticPackageTopology::new(
                [module("a"), module("a.b")],
                [class("A", "a"), class("B", "a.b")]
            )
            .is_err()
        );
        assert!(
            PydanticPackageTopology::new(
                [module("Case"), module("case")],
                [class("A", "Case"), class("B", "case")]
            )
            .is_err()
        );
        assert!(
            PydanticPackageTopology::new([module("a")], [class("A", "a"), class("A", "a")])
                .is_err()
        );
    }

    #[test]
    fn shared_intermediate_package_initializers_are_valid() {
        let topology = PydanticPackageTopology::new(
            [module("domain.people"), module("domain.places")],
            [
                class("Person", "domain.people"),
                class("Place", "domain.places"),
            ],
        )
        .expect("shared intermediate package");
        validate_relative_artifacts(Some(topology.modules()), false)
            .expect("shared initializer is emitted once");
    }

    #[test]
    fn artifact_count_fails_before_inserting_the_first_one_over() {
        let mut paths = (0..MAX_ARTIFACTS)
            .map(|index| format!("path-{index}"))
            .collect::<BTreeSet<_>>();
        let mut folded = paths
            .iter()
            .map(|path| (path.to_ascii_lowercase(), path.clone()))
            .collect::<BTreeMap<_, _>>();
        let existing = paths.first().expect("full path set has an entry").clone();
        insert_artifact(existing, &mut paths, &mut folded).expect("duplicate is not a new file");
        let error = insert_artifact("one-over".to_owned(), &mut paths, &mut folded)
            .expect_err("new artifact beyond the ceiling");
        assert!(error.to_string().contains("131072-artifact limit"));
        assert_eq!(paths.len(), MAX_ARTIFACTS);
        assert!(!paths.contains("one-over"));
    }

    #[test]
    fn paths_reject_keywords_generated_names_devices_and_limits() {
        for path in [
            "",
            "bad-name",
            "class",
            "naïve",
            "con",
            "COM9",
            "_base",
            "_SCHEMA.child",
            "__about__.x",
        ] {
            assert!(PydanticModuleConfig::new(path, "docs").is_err(), "{path}");
        }
        let max = format!("a{}", "b".repeat(MAX_DOTTED_PATH_BYTES - 1));
        assert_eq!(max.len(), MAX_DOTTED_PATH_BYTES);
        assert!(PydanticModuleConfig::new(max, "docs").is_ok());
        let over = format!("a{}", "b".repeat(MAX_DOTTED_PATH_BYTES));
        assert!(PydanticModuleConfig::new(over, "docs").is_err());
        let deep = std::iter::repeat_n("a", MAX_DOTTED_PATH_COMPONENTS + 1)
            .collect::<Vec<_>>()
            .join(".");
        assert!(PydanticModuleConfig::new(deep, "docs").is_err());
        assert!(PydanticConfig::new("con", "package docs", "model docs").is_err());
    }

    #[test]
    fn docs_and_metadata_fail_at_fixed_boundaries() {
        assert!(PydanticModuleConfig::new("models", " ").is_err());
        assert!(PydanticModuleConfig::new("models", "x".repeat(MAX_DOCSTRING_BYTES)).is_ok());
        assert!(PydanticModuleConfig::new("models", "x".repeat(MAX_DOCSTRING_BYTES + 1)).is_err());

        let mut accepted = Value::Null;
        for _ in 1..MAX_METADATA_DEPTH {
            accepted = Value::Array(vec![accepted]);
        }
        assert!(
            PydanticClassConfig::new(
                "Deep",
                "models",
                "docs",
                BTreeMap::from([("value".to_owned(), accepted)]),
            )
            .is_ok()
        );

        let mut rejected = Value::Null;
        for _ in 0..MAX_METADATA_DEPTH {
            rejected = Value::Array(vec![rejected]);
        }
        assert!(
            PydanticClassConfig::new(
                "TooDeep",
                "models",
                "docs",
                BTreeMap::from([("value".to_owned(), rejected)]),
            )
            .is_err()
        );
    }

    #[test]
    fn pep440_accepts_complete_public_and_local_grammar() {
        for version in [
            "0",
            "v1.2",
            "1!2.0",
            "1.0a1",
            "1.0-alpha",
            "1.0beta2",
            "1.0b3",
            "1.0preview2",
            "1.0pre4",
            "1.0c5",
            "1.0RC1",
            "1.0-1",
            "1.0-post2",
            "1.0rev",
            "1.0post-",
            "1.0_post_7",
            "1.0rc_",
            "1.0dev-",
            "1.0_dev_9",
            "1.0.dev3",
            "1.0rc1.post2.dev3",
            "1.0+abc.1",
            "1.0+Ubuntu-1",
            " \tv1.0RC1+LOCAL_1\n",
        ] {
            let stamp = PydanticVersionStamp::new(version, "Caller version docs.")
                .unwrap_or_else(|error| panic!("{version:?}: {error}"));
            assert_eq!(stamp.version(), version);
            assert_eq!(stamp.is_local(), version.contains('+'));
        }
    }

    #[test]
    fn pep440_rejects_malformed_non_ascii_and_over_limit_versions() {
        for version in [
            "",
            "v",
            "1..0",
            "1.0+",
            "1.0+abc+def",
            "1.0++x",
            "1.0+abc_",
            "1.0-",
            "1.0_",
            "1.0..post1",
            "1.0a..1",
            "1!",
            "1.0foo",
            "1.0+naïve",
        ] {
            assert!(
                PydanticVersionStamp::new(version, "Caller version docs.").is_err(),
                "{version:?}"
            );
        }
        let accepted = format!("1.{}", "7".repeat(MAX_VERSION_BYTES - 2));
        assert_eq!(accepted.len(), MAX_VERSION_BYTES);
        assert!(PydanticVersionStamp::new(accepted, "docs").is_ok());
        let rejected = format!("1.{}", "7".repeat(MAX_VERSION_BYTES - 1));
        assert_eq!(rejected.len(), MAX_VERSION_BYTES + 1);
        let error = PydanticVersionStamp::new(rejected, "docs").expect_err("version limit");
        assert!(error.to_string().contains("512"));
    }
}
