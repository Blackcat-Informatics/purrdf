// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The **build identity's text**: the compiler and codegen facts, outside the source's
//! `cfg`, that decide what the [`Reassociated`](super::Reassociated) body compiles to.
//!
//! This one file is compiled twice. The crate's build script includes it (`#[path]`) and
//! writes this build's text into the `PURRDF_BUILD_IDENTITY` compile-time variable; the
//! library includes it as the pure encoder behind
//! [`BuildIdentity::encode`](super::BuildIdentity::encode) and tests it. So the text a
//! build records and the text the encoder computes cannot drift apart. It uses nothing but
//! `std`, because the build script has nothing else.

/// What a build script observes about the compilation of the crate holding the
/// reassociated body.
///
/// Every field is a value `cargo` hands the crate's build script, or the output of a
/// command it runs against the `rustc` `cargo` names; see
/// [`BuildIdentity::encode`](super::BuildIdentity::encode) for which and why.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuildInputs<'a> {
    /// The output of `rustc -vV`: its `release`, `commit-hash` and `LLVM version` lines
    /// are read.
    pub rustc_version: &'a str,
    /// The output of `rustc --print target-cpus --target <target>`, or `None` where it
    /// could not be obtained: it names the target's default CPU and what `native` is on
    /// the building host.
    pub target_cpus: Option<&'a str>,
    /// The target triple (`cargo`'s `TARGET`), named only when the target's default CPU
    /// cannot be resolved from [`BuildInputs::target_cpus`].
    pub target: &'a str,
    /// The flags `rustc` compiled the crate with beyond the profile, one argument per
    /// element: `cargo`'s `CARGO_ENCODED_RUSTFLAGS`, split on `0x1f`.
    pub rustflags: &'a [&'a str],
    /// The profile's optimisation level (`cargo`'s `OPT_LEVEL`): `0`, `1`, `2`, `3`, `s`
    /// or `z`.
    pub opt_level: &'a str,
}

/// The codegen options a flag list sets, each as the last flag that set it, or
/// accumulated where `rustc` accumulates.
#[derive(Default)]
struct Flags<'a> {
    /// `-C target-cpu`.
    target_cpu: Option<&'a str>,
    /// `-C target-feature`, every entry in order.
    target_features: Vec<&'a str>,
    /// `-C llvm-args`, every value in order.
    llvm_args: Vec<&'a str>,
    /// `-C opt-level`, or `-O`.
    opt_level: Option<&'a str>,
    /// `-C codegen-units`.
    codegen_units: Option<&'a str>,
    /// `-C lto`.
    lto: Option<&'a str>,
    /// `-C overflow-checks`.
    overflow_checks: Option<&'a str>,
}

impl<'a> Flags<'a> {
    /// The codegen options `flags` set; every other flag is ignored, because it does not
    /// change the code the body compiles to (`-D warnings` among them).
    fn of(flags: &[&'a str]) -> Self {
        let mut codegen = Self::default();
        let mut rest = flags.iter();
        while let Some(&flag) = rest.next() {
            let option = match flag {
                "-C" | "--codegen" => rest.next().copied(),
                "-O" => {
                    codegen.opt_level = Some("-O");
                    None
                }
                _ => flag
                    .strip_prefix("--codegen=")
                    .or_else(|| flag.strip_prefix("-C")),
            };
            if let Some(option) = option {
                codegen.set(option);
            }
        }
        codegen
    }

    /// Record one `-C key=value` option.
    fn set(&mut self, option: &'a str) {
        let (key, value) = option.split_once('=').unwrap_or((option, ""));
        match key {
            "target-cpu" => self.target_cpu = Some(value),
            "target-feature" => self.target_features.extend(value.split(',')),
            "llvm-args" => self.llvm_args.push(value),
            "opt-level" => self.opt_level = Some(value),
            "codegen-units" => self.codegen_units = Some(value),
            "lto" => self.lto = Some(value),
            "overflow-checks" => self.overflow_checks = Some(value),
            _ => {}
        }
    }

    /// The target features as `rustc` applies them -- the last `+` or `-` of each name
    /// wins -- sorted by name, so two spellings of one feature set are one text. The C
    /// runtime's static linking (`crt-static`) is a link choice, not a codegen one, and is
    /// left out.
    fn features(&self) -> String {
        let mut last: Vec<(&str, char)> = Vec::new();
        for entry in &self.target_features {
            let entry = entry.trim();
            let (sign, name) = match entry.chars().next() {
                Some(sign @ ('+' | '-')) => (sign, &entry[1..]),
                _ => ('+', entry),
            };
            if name.is_empty() || name == "crt-static" {
                continue;
            }
            last.retain(|(seen, _)| *seen != name);
            last.push((name, sign));
        }
        last.sort_unstable();
        last.iter()
            .map(|(name, sign)| format!("{sign}{name}"))
            .collect::<Vec<_>>()
            .join(",")
    }
}

/// The value of the `rustc -vV` line starting `key`, or `unknown`.
fn version_line<'a>(version: &'a str, key: &str) -> &'a str {
    version
        .lines()
        .find_map(|line| line.strip_prefix(key))
        .map_or("unknown", str::trim)
}

/// The CPU `rustc` compiles for: the one `-C target-cpu` names, with `native` resolved to
/// the host CPU `rustc` reports, or the target's default when none is named.
///
/// A `native` or default CPU that the `--print target-cpus` output does not resolve is
/// written unresolved, and the default one with its target: an identity that cannot be
/// resolved is never written as a resolved one.
fn target_cpu(inputs: &BuildInputs<'_>, named: Option<&str>) -> String {
    let listing = inputs.target_cpus.unwrap_or("");
    match named {
        Some("native") => listing
            .lines()
            .map(str::trim)
            .filter(|line| line.split_whitespace().next() == Some("native"))
            .find_map(|line| currently(line))
            .map_or_else(|| "native(unresolved)".to_owned(), str::to_owned),
        Some(cpu) => cpu.to_owned(),
        None => listing
            .lines()
            .map(str::trim)
            .find(|line| line.contains("default target CPU"))
            .and_then(|line| line.split_whitespace().next())
            .map_or_else(|| format!("default({})", inputs.target), str::to_owned),
    }
}

/// The text inside a `--print target-cpus` line's `(currently …)`.
fn currently(line: &str) -> Option<&str> {
    let (_, rest) = line.split_once("(currently ")?;
    let (cpu, _) = rest.split_once(')')?;
    let cpu = cpu.trim();
    (!cpu.is_empty()).then_some(cpu)
}

/// The identity text of the compilation `inputs` describe, without the debug-assertion
/// state, which the library appends from its own `cfg`.
///
/// Fields are `key=value`, joined by `"; "`, always in this order: `rustc`, `commit`,
/// `llvm`, `target-cpu`, then `target-feature`, `llvm-args`, `codegen-units`, `lto` and
/// `overflow-checks` only where a flag sets them, then `opt-level`. A control character
/// in a value is written as a space, so the text is always one line.
pub(crate) fn compiler_text(inputs: &BuildInputs<'_>) -> String {
    let codegen = Flags::of(inputs.rustflags);
    let mut fields = vec![
        format!("rustc={}", version_line(inputs.rustc_version, "release:")),
        format!(
            "commit={}",
            version_line(inputs.rustc_version, "commit-hash:")
        ),
        format!(
            "llvm={}",
            version_line(inputs.rustc_version, "LLVM version:")
        ),
        format!("target-cpu={}", target_cpu(inputs, codegen.target_cpu)),
    ];
    let features = codegen.features();
    if !features.is_empty() {
        fields.push(format!("target-feature={features}"));
    }
    if !codegen.llvm_args.is_empty() {
        fields.push(format!("llvm-args={}", codegen.llvm_args.join(" ")));
    }
    if let Some(units) = codegen.codegen_units {
        fields.push(format!("codegen-units={units}"));
    }
    if let Some(lto) = codegen.lto {
        fields.push(format!("lto={lto}"));
    }
    if let Some(checks) = codegen.overflow_checks {
        fields.push(format!("overflow-checks={checks}"));
    }
    fields.push(format!(
        "opt-level={}",
        codegen.opt_level.unwrap_or(inputs.opt_level)
    ));
    fields
        .join("; ")
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect()
}
