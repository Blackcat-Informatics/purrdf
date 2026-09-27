// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The first pass over a module: its index spaces, exports, bodies and declared
//! features, read once so the rewrite and the check can reason about indices before
//! either emits or matches a byte.

use std::convert::Infallible;

use wasm_encoder::ValType;
use wasm_encoder::reencode::{Reencode as _, RoundtripReencoder};
use wasmparser::{
    CompositeInnerType, ElementItems, ExternalKind, FunctionBody, Operator, Parser, Payload,
    TypeRef, WasmFeatures,
};

use crate::error::LinkError;
use crate::{SUSPEND_MODULE, SUSPEND_NAME};

/// A core function signature in wasm-encoder's value types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Sig {
    /// Parameter types, in order.
    pub(crate) params: Vec<ValType>,
    /// Result types, in order.
    pub(crate) results: Vec<ValType>,
}

/// One function import, in function-index order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ImportedFunc {
    /// The import's module string.
    pub(crate) module: String,
    /// The import's field name.
    pub(crate) name: String,
    /// Its type index.
    pub(crate) ty: u32,
}

/// One export entry, in export-section order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExportEntry {
    /// The export name.
    pub(crate) name: String,
    /// What kind of item it exports.
    pub(crate) kind: ExternalKind,
    /// The index in that kind's index space.
    pub(crate) index: u32,
}

/// One global, imported or defined, in global-index order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct GlobalEntry {
    /// Whether the module defines it (an imported global cannot be the stack pointer or
    /// a gate counter).
    pub(crate) defined: bool,
    /// Whether it is mutable.
    pub(crate) mutable: bool,
    /// Its value type.
    pub(crate) val_type: ValType,
}

/// Everything the link and the check need to know about a module before touching it.
#[derive(Debug)]
pub(crate) struct Scan<'a> {
    /// Every core function type, by type index.
    pub(crate) types: Vec<Sig>,
    /// The function imports, by function index.
    pub(crate) imports: Vec<ImportedFunc>,
    /// The defined functions' type indices, by function index minus the import count.
    pub(crate) function_types: Vec<u32>,
    /// Every global, imported then defined, by global index.
    pub(crate) globals: Vec<GlobalEntry>,
    /// The export entries, in order.
    pub(crate) exports: Vec<ExportEntry>,
    /// The defined functions' bodies, by function index minus the import count.
    pub(crate) bodies: Vec<FunctionBody<'a>>,
    /// Whether a `name` custom section is present.
    pub(crate) has_name_section: bool,
    /// The features a `target_features` custom section enables (`+` entries).
    pub(crate) declared_features: Vec<String>,
    /// Every function an element segment references, by index or by `ref.func`.
    pub(crate) element_functions: Vec<u32>,
}

impl<'a> Scan<'a> {
    /// Read `bytes` once, section by section.
    pub(crate) fn read(bytes: &'a [u8]) -> Result<Self, LinkError> {
        let mut scan = Self {
            types: Vec::new(),
            imports: Vec::new(),
            function_types: Vec::new(),
            globals: Vec::new(),
            exports: Vec::new(),
            bodies: Vec::new(),
            has_name_section: false,
            declared_features: Vec::new(),
            element_functions: Vec::new(),
        };
        for payload in Parser::new(0).parse_all(bytes) {
            match payload? {
                Payload::TypeSection(reader) => {
                    for group in reader {
                        for sub in group?.into_types() {
                            let CompositeInnerType::Func(func) = sub.composite_type.inner else {
                                return Err(LinkError::Shape(format!(
                                    "type {} is not a plain function type",
                                    scan.types.len()
                                )));
                            };
                            scan.types.push(Sig {
                                params: val_types(func.params())?,
                                results: val_types(func.results())?,
                            });
                        }
                    }
                }
                Payload::ImportSection(reader) => {
                    for imports in reader {
                        for entry in imports? {
                            let (_, import) = entry?;
                            match import.ty {
                                TypeRef::Func(ty) | TypeRef::FuncExact(ty) => {
                                    scan.imports.push(ImportedFunc {
                                        module: import.module.to_owned(),
                                        name: import.name.to_owned(),
                                        ty,
                                    });
                                }
                                TypeRef::Global(global) => scan.globals.push(GlobalEntry {
                                    defined: false,
                                    mutable: global.mutable,
                                    val_type: val_type(global.content_type)?,
                                }),
                                TypeRef::Table(_) | TypeRef::Memory(_) | TypeRef::Tag(_) => {}
                            }
                        }
                    }
                }
                Payload::FunctionSection(reader) => {
                    for ty in reader {
                        scan.function_types.push(ty?);
                    }
                }
                Payload::GlobalSection(reader) => {
                    for global in reader {
                        let global = global?;
                        scan.globals.push(GlobalEntry {
                            defined: true,
                            mutable: global.ty.mutable,
                            val_type: val_type(global.ty.content_type)?,
                        });
                    }
                }
                Payload::ExportSection(reader) => {
                    for export in reader {
                        let export = export?;
                        scan.exports.push(ExportEntry {
                            name: export.name.to_owned(),
                            kind: export.kind,
                            index: export.index,
                        });
                    }
                }
                Payload::ElementSection(reader) => {
                    for element in reader {
                        match element?.items {
                            ElementItems::Functions(functions) => {
                                for function in functions {
                                    scan.element_functions.push(function?);
                                }
                            }
                            ElementItems::Expressions(_, expressions) => {
                                for expression in expressions {
                                    for operator in expression?.get_operators_reader() {
                                        if let Operator::RefFunc { function_index } = operator? {
                                            scan.element_functions.push(function_index);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                Payload::CodeSectionEntry(body) => scan.bodies.push(body),
                Payload::CustomSection(section) => {
                    if section.name() == "name" {
                        scan.has_name_section = true;
                    }
                    if section.name() == "target_features" {
                        scan.declared_features = declared_features(section.data())?;
                    }
                }
                _ => {}
            }
        }
        if scan.function_types.len() != scan.bodies.len() {
            return Err(LinkError::Shape(format!(
                "the function section declares {} functions but the code section holds {} bodies",
                scan.function_types.len(),
                scan.bodies.len()
            )));
        }
        Ok(scan)
    }

    /// The number of imported functions: the first defined function's index.
    pub(crate) fn import_count(&self) -> u32 {
        self.imports.len() as u32
    }

    /// The number of defined functions.
    pub(crate) fn defined_count(&self) -> u32 {
        self.function_types.len() as u32
    }

    /// The number of globals, imported and defined.
    pub(crate) fn global_count(&self) -> u32 {
        self.globals.len() as u32
    }

    /// The type index of function `func`, imported or defined.
    pub(crate) fn type_index_of(&self, func: u32) -> Result<u32, LinkError> {
        let import_count = self.import_count();
        let ty = if func < import_count {
            self.imports[func as usize].ty
        } else {
            *self
                .function_types
                .get((func - import_count) as usize)
                .ok_or_else(|| LinkError::Shape(format!("function {func} does not exist")))?
        };
        Ok(ty)
    }

    /// The signature of function `func`, imported or defined.
    pub(crate) fn sig_of(&self, func: u32) -> Result<&Sig, LinkError> {
        let ty = self.type_index_of(func)?;
        self.types.get(ty as usize).ok_or_else(|| {
            LinkError::Shape(format!(
                "function {func} names type {ty}, which does not exist"
            ))
        })
    }

    /// The body of a defined function.
    pub(crate) fn body_of(&self, func: u32) -> Result<&FunctionBody<'a>, LinkError> {
        let import_count = self.import_count();
        if func < import_count {
            return Err(LinkError::Shape(format!(
                "function {func} is an import and has no body"
            )));
        }
        self.bodies
            .get((func - import_count) as usize)
            .ok_or_else(|| LinkError::Shape(format!("function {func} does not exist")))
    }

    /// The export named `name`, if any.
    pub(crate) fn export(&self, name: &str) -> Option<&ExportEntry> {
        self.exports.iter().find(|export| export.name == name)
    }

    /// The exported function named `name`.
    pub(crate) fn exported_function(&self, name: &str) -> Result<u32, LinkError> {
        match self.export(name) {
            Some(export) if export.kind == ExternalKind::Func => Ok(export.index),
            Some(export) => Err(LinkError::Shape(format!(
                "export {name:?} is a {:?}, not a function",
                export.kind
            ))),
            None => Err(LinkError::Shape(format!("no export named {name:?}"))),
        }
    }

    /// The function index of the one `purrdf_jspi_suspend` import.
    pub(crate) fn suspend_import(&self) -> Result<u32, LinkError> {
        let found: Vec<u32> = self
            .imports
            .iter()
            .enumerate()
            .filter(|(_, import)| import.module == SUSPEND_MODULE && import.name == SUSPEND_NAME)
            .map(|(index, _)| index as u32)
            .collect();
        match found.as_slice() {
            [index] => Ok(*index),
            _ => Err(LinkError::Shape(format!(
                "expected exactly one import of {SUSPEND_MODULE:?} {SUSPEND_NAME:?}, found {}",
                found.len()
            ))),
        }
    }

    /// The feature set the module is validated under: the WebAssembly MVP, the five
    /// post-MVP proposals the package build enables in `wasm-opt` (mutable globals,
    /// non-trapping float-to-int, sign extension, bulk memory, SIMD), and every feature
    /// a `target_features` custom section declares with `+`.
    pub(crate) fn features(&self) -> Result<WasmFeatures, LinkError> {
        let mut features = WasmFeatures::MVP
            | WasmFeatures::MUTABLE_GLOBAL
            | WasmFeatures::SATURATING_FLOAT_TO_INT
            | WasmFeatures::SIGN_EXTENSION
            | WasmFeatures::BULK_MEMORY
            | WasmFeatures::SIMD;
        for name in &self.declared_features {
            features |= feature_named(name)?;
        }
        Ok(features)
    }
}

/// The validator flag for an LLVM `target_features` name.
fn feature_named(name: &str) -> Result<WasmFeatures, LinkError> {
    let feature = match name {
        "mutable-globals" => WasmFeatures::MUTABLE_GLOBAL,
        "nontrapping-fptoint" => WasmFeatures::SATURATING_FLOAT_TO_INT,
        "sign-ext" => WasmFeatures::SIGN_EXTENSION,
        "bulk-memory" | "bulk-memory-opt" => WasmFeatures::BULK_MEMORY,
        "simd128" => WasmFeatures::SIMD,
        "relaxed-simd" => WasmFeatures::RELAXED_SIMD,
        "reference-types" | "call-indirect-overlong" => WasmFeatures::REFERENCE_TYPES,
        "multivalue" => WasmFeatures::MULTI_VALUE,
        "tail-call" => WasmFeatures::TAIL_CALL,
        "exception-handling" => WasmFeatures::EXCEPTIONS,
        "extended-const" => WasmFeatures::EXTENDED_CONST,
        "multimemory" => WasmFeatures::MULTI_MEMORY,
        "atomics" => WasmFeatures::THREADS,
        "memory64" => WasmFeatures::MEMORY64,
        "gc" => WasmFeatures::GC,
        "wide-arithmetic" => WasmFeatures::WIDE_ARITHMETIC,
        other => {
            return Err(LinkError::Shape(format!(
                "the module declares target feature {other:?}, which this tool has no validator flag for"
            )));
        }
    };
    Ok(feature)
}

/// The `+` entries of a `target_features` custom section: a count, then a prefix byte
/// and a length-prefixed name per entry.
fn declared_features(data: &[u8]) -> Result<Vec<String>, LinkError> {
    let malformed =
        || LinkError::Shape("the target_features custom section is malformed".to_owned());
    let mut cursor = 0usize;
    let count = read_leb(data, &mut cursor).ok_or_else(malformed)?;
    let mut enabled = Vec::new();
    for _ in 0..count {
        let prefix = *data.get(cursor).ok_or_else(malformed)?;
        cursor += 1;
        let length = read_leb(data, &mut cursor).ok_or_else(malformed)? as usize;
        let name = data.get(cursor..cursor + length).ok_or_else(malformed)?;
        cursor += length;
        let name = std::str::from_utf8(name).map_err(|_| malformed())?;
        match prefix {
            b'+' => enabled.push(name.to_owned()),
            b'-' => {}
            _ => return Err(malformed()),
        }
    }
    if cursor != data.len() {
        return Err(malformed());
    }
    Ok(enabled)
}

/// One unsigned LEB128 value at `*cursor`, advancing past it.
fn read_leb(data: &[u8], cursor: &mut usize) -> Option<u32> {
    let mut value = 0u32;
    let mut shift = 0u32;
    loop {
        let byte = *data.get(*cursor)?;
        *cursor += 1;
        if shift >= 32 {
            return None;
        }
        value |= u32::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            return Some(value);
        }
        shift += 7;
    }
}

/// wasmparser's value type as wasm-encoder's.
pub(crate) fn val_type(ty: wasmparser::ValType) -> Result<ValType, LinkError> {
    let translated: Result<ValType, wasm_encoder::reencode::Error<Infallible>> =
        RoundtripReencoder.val_type(ty);
    Ok(translated?)
}

fn val_types(types: &[wasmparser::ValType]) -> Result<Vec<ValType>, LinkError> {
    types.iter().map(|ty| val_type(*ty)).collect()
}
