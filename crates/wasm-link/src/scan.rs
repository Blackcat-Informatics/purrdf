// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The first pass over a module: its index spaces, exports, bodies and declared
//! features, read once so the rewrite and the check can reason about indices before
//! either emits or matches a byte.

use crate::binary::{ExternalKind, FunctionBody, Reader, Section, sections};
use crate::error::LinkError;
use crate::template::Op;
use crate::{SUSPEND_MODULE, SUSPEND_NAME};
use wasm_encoder::ValType;

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
    /// References outside functions, including initializer expressions and start.
    pub(crate) external_functions: Vec<(u32, &'static str)>,
    /// Original section payloads and function-index immediate ranges within them.
    pub(crate) sections: Vec<Section<'a>>,
    pub(crate) references: Vec<(u8, usize, usize, u32, bool)>,
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
            external_functions: Vec::new(),
            sections: sections(bytes)?,
            references: Vec::new(),
        };
        for section in scan.sections.clone() {
            let mut r = Reader::new(section.data);
            match section.id {
                0 => {
                    let name = r.name()?;
                    if name == "name" {
                        scan.has_name_section = true;
                    }
                    if name == "target_features" {
                        scan.declared_features = declared_features(&r.data[r.pos..])?;
                    }
                    continue;
                }
                1 => {
                    for _ in 0..r.u32()? {
                        if r.data.get(r.pos) == Some(&0x4e) {
                            r.byte()?;
                            for _ in 0..r.u32()? {
                                scan.types.push(signature(&mut r)?);
                            }
                        } else {
                            scan.types.push(signature(&mut r)?);
                        }
                    }
                }
                2 => {
                    for _ in 0..r.u32()? {
                        let module = r.name()?.to_owned();
                        let name = r.name()?.to_owned();
                        match ExternalKind::read(&mut r)? {
                            ExternalKind::Func => {
                                let ty = r.u32()?;
                                scan.imports.push(ImportedFunc { module, name, ty });
                            }
                            ExternalKind::Table => {
                                r.val()?;
                                r.limits()?;
                            }
                            ExternalKind::Memory => r.limits()?,
                            ExternalKind::Global => {
                                let val_type = r.val()?;
                                let mutable = r.byte()? & 1 != 0;
                                scan.globals.push(GlobalEntry {
                                    defined: false,
                                    mutable,
                                    val_type,
                                });
                            }
                            ExternalKind::Tag => {
                                r.byte()?;
                                r.u32()?;
                            }
                        }
                    }
                }
                3 => {
                    for _ in 0..r.u32()? {
                        scan.function_types.push(r.u32()?);
                    }
                }
                4 => {
                    for _ in 0..r.u32()? {
                        let explicit = r.data.get(r.pos) == Some(&0x40);
                        if explicit {
                            r.byte()?;
                            if r.byte()? != 0 {
                                return Err(LinkError::Parse(
                                    "invalid table initializer prefix".into(),
                                ));
                            }
                        }
                        r.val()?;
                        r.limits()?;
                        if explicit {
                            scan.expression(&mut r, 4, "table initializer", false)?;
                        }
                    }
                }
                5 => {
                    for _ in 0..r.u32()? {
                        r.limits()?;
                    }
                }
                6 => {
                    for _ in 0..r.u32()? {
                        let val_type = r.val()?;
                        let mutable = r.byte()? & 1 != 0;
                        scan.globals.push(GlobalEntry {
                            defined: true,
                            mutable,
                            val_type,
                        });
                        scan.expression(&mut r, 6, "global initializer", false)?;
                    }
                }
                7 => {
                    for _ in 0..r.u32()? {
                        let name = r.name()?.to_owned();
                        let kind = ExternalKind::read(&mut r)?;
                        let index = r.u32()?;
                        scan.exports.push(ExportEntry { name, kind, index });
                    }
                }
                8 => {
                    let start = r.pos;
                    let index = r.u32()?;
                    scan.references.push((8, start, r.pos, index, false));
                    scan.external_functions.push((index, "start section"));
                }
                9 => {
                    for _ in 0..r.u32()? {
                        let flags = r.u32()?;
                        if flags > 7 {
                            return Err(LinkError::Parse("unknown element mode".into()));
                        }
                        if flags & 1 == 0 {
                            if flags & 2 != 0 {
                                r.u32()?;
                            }
                            scan.expression(&mut r, 9, "element offset", false)?;
                        }
                        if flags & 4 == 0 {
                            if flags & 3 != 0 && r.byte()? != 0 {
                                return Err(LinkError::Parse("invalid element kind".into()));
                            }
                            for _ in 0..r.u32()? {
                                let start = r.pos;
                                let index = r.u32()?;
                                scan.references.push((9, start, r.pos, index, false));
                                scan.element_functions.push(index);
                            }
                        } else {
                            if flags & 3 != 0 {
                                r.val()?;
                            }
                            for _ in 0..r.u32()? {
                                scan.expression(&mut r, 9, "element segment", true)?;
                            }
                        }
                    }
                }
                10 => {
                    for _ in 0..r.u32()? {
                        let len = r.u32()? as usize;
                        scan.bodies.push(FunctionBody { data: r.take(len)? });
                    }
                }
                11 => {
                    for _ in 0..r.u32()? {
                        match r.u32()? {
                            0 => scan.expression(&mut r, 11, "data offset", false)?,
                            1 => {}
                            2 => {
                                r.u32()?;
                                scan.expression(&mut r, 11, "data offset", false)?;
                            }
                            _ => return Err(LinkError::Parse("invalid data mode".into())),
                        }
                        let len = r.u32()? as usize;
                        r.take(len)?;
                    }
                }
                12 => {
                    r.u32()?;
                }
                13 => {
                    for _ in 0..r.u32()? {
                        r.byte()?;
                        r.u32()?;
                    }
                }
                id => return Err(LinkError::Parse(format!("unknown core section {id}"))),
            }
            r.finish()?;
        }
        if scan.function_types.len() != scan.bodies.len() {
            return Err(LinkError::Shape("function and code counts differ".into()));
        }
        Ok(scan)
    }
    fn expression(
        &mut self,
        r: &mut Reader<'_>,
        section: u8,
        origin: &'static str,
        element: bool,
    ) -> Result<(), LinkError> {
        loop {
            let (op, reference) = r.op()?;
            if let Some((start, end, index)) = reference {
                self.references.push((section, start, end, index, false));
                if element {
                    self.element_functions.push(index);
                } else {
                    self.external_functions.push((index, origin));
                }
            }
            if op == Op::End {
                return Ok(());
            }
        }
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

    /// Binaryen flags corresponding exactly to the package baseline and LLVM declarations.
    pub(crate) fn features(&self) -> Result<Vec<String>, LinkError> {
        let mut features = vec![
            "mutable-globals".into(),
            "nontrapping-float-to-int".into(),
            "sign-ext".into(),
            "bulk-memory".into(),
            "simd".into(),
        ];
        for name in &self.declared_features {
            let flag = match name.as_str() {
                "nontrapping-fptoint" => "nontrapping-float-to-int",
                "simd128" => "simd",
                "atomics" => "threads",
                "mutable-globals"
                | "sign-ext"
                | "bulk-memory"
                | "bulk-memory-opt"
                | "relaxed-simd"
                | "reference-types"
                | "call-indirect-overlong"
                | "multivalue"
                | "tail-call"
                | "exception-handling"
                | "extended-const"
                | "multimemory"
                | "memory64"
                | "gc"
                | "wide-arithmetic" => name,
                other => {
                    return Err(LinkError::Shape(format!(
                        "the module declares target feature {other:?}, which this tool has no validator flag for"
                    )));
                }
            };
            if !features.iter().any(|x| x == flag) {
                features.push(flag.into());
            }
        }
        Ok(features)
    }
}
fn declared_features(data: &[u8]) -> Result<Vec<String>, LinkError> {
    let mut r = Reader::new(data);
    let mut enabled = Vec::new();
    for _ in 0..r.u32()? {
        let prefix = r.byte()?;
        let name = r.name()?;
        match prefix {
            b'+' => enabled.push(name.into()),
            b'-' => {}
            _ => return Err(LinkError::Parse("invalid target feature prefix".into())),
        }
    }
    r.finish()?;
    Ok(enabled)
}

/// Plain function types, including explicit recursive groups and subtype headers.
fn signature(r: &mut Reader<'_>) -> Result<Sig, LinkError> {
    let mut code = r.byte()?;
    if matches!(code, 0x4f | 0x50) {
        for _ in 0..r.u32()? {
            r.u32()?;
        }
        code = r.byte()?;
    }
    if code == 0x65 {
        code = r.byte()?;
    }
    if code != 0x60 {
        return Err(LinkError::Shape("type is not a plain function type".into()));
    }
    let mut params = Vec::new();
    for _ in 0..r.u32()? {
        params.push(r.val()?);
    }
    let mut results = Vec::new();
    for _ in 0..r.u32()? {
        results.push(r.val()?);
    }
    Ok(Sig { params, results })
}
