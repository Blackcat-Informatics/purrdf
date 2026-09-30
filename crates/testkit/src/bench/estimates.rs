// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! One benchmark's estimates, and the fixed-schema JSON file that carries them.
//!
//! # The estimates file
//!
//! Each measured benchmark writes
//! `<store>/<group>[/<function>][/<parameter>]/<record>/estimates.json`, where
//! `<record>` is `new` for the latest run or the name given to
//! `--save-baseline`. The file names no benchmark: its path is its identity.
//! Its one object has exactly these members, written in this order, and a
//! reader refuses a missing, repeated or unknown member:
//!
//! | Member | JSON type | Meaning |
//! |---|---|---|
//! | `schema` | integer | `1`, this layout. |
//! | `unit` | string | `"ns"`: every time below is nanoseconds per iteration. |
//! | `median` | number | The median of `samples`. |
//! | `mad` | number | The median absolute deviation of `samples` from `median`, unscaled. |
//! | `ci_low`, `ci_high` | number | The bootstrap percentile interval of the median at `confidence`. |
//! | `confidence` | number | `0.95`. |
//! | `resamples` | integer | The bootstrap's resample count. |
//! | `iterations_per_sample` | integer | How many iterations each sample timed. |
//! | `samples` | array of numbers | Each sample's time per iteration, in collection order. |
//! | `outliers` | object | `low_severe`, `low_mild`, `high_mild`, `high_severe`: Tukey-fence counts over `samples`. |
//! | `throughput` | object or `null` | `{"kind": "bytes" \| "elements", "per_iteration": <integer>}`, as the benchmark declared it. |
//!
//! Numbers are written in Rust's shortest round-trip decimal form, so reading
//! a file back yields the same `f64` values bit for bit. The writer refuses a
//! non-finite number rather than write a file JSON cannot express. Strings are
//! only the schema's own constants, so the file never needs an escape; the
//! reader refuses one.

use std::fmt::{self, Write as _};

use super::Throughput;
use super::stats::{self, CONFIDENCE, Outliers, RESAMPLES};

/// The layout version [`Estimates::to_json`] writes and [`Estimates::from_json`] reads.
pub const SCHEMA: u64 = 1;

/// The one time unit of an estimates file.
pub const UNIT: &str = "ns";

/// One benchmark's estimates, in nanoseconds per iteration.
#[derive(Debug, Clone, PartialEq)]
pub struct Estimates {
    /// The median of [`samples`](Self::samples).
    pub median: f64,
    /// The median absolute deviation from the median, unscaled.
    pub mad: f64,
    /// The lower bound of the median's bootstrap interval.
    pub ci_low: f64,
    /// The upper bound of the median's bootstrap interval.
    pub ci_high: f64,
    /// The interval's confidence level.
    pub confidence: f64,
    /// The bootstrap's resample count.
    pub resamples: u64,
    /// How many iterations each sample timed.
    pub iterations_per_sample: u64,
    /// Each sample's time per iteration, in collection order.
    pub samples: Vec<f64>,
    /// Tukey-fence outlier counts over the samples.
    pub outliers: Outliers,
    /// The work one iteration does, when the benchmark declared it.
    pub throughput: Option<Throughput>,
}

impl Estimates {
    /// The estimates of `samples` (nanoseconds per iteration, each timed over
    /// `iterations_per_sample` iterations), with the bootstrap seeded at
    /// `seed`.
    pub fn from_samples(
        samples: Vec<f64>,
        iterations_per_sample: u64,
        throughput: Option<Throughput>,
        seed: u64,
    ) -> Self {
        let median = stats::median(&samples);
        let interval = stats::bootstrap_median(&samples, RESAMPLES, CONFIDENCE, seed);
        Self {
            median,
            mad: stats::median_absolute_deviation(&samples, median),
            ci_low: interval.low,
            ci_high: interval.high,
            confidence: CONFIDENCE,
            resamples: RESAMPLES as u64,
            iterations_per_sample,
            outliers: stats::classify_outliers(&samples),
            samples,
            throughput,
        }
    }

    /// The estimates file's text.
    pub fn to_json(&self) -> Result<String, EstimatesError> {
        let mut out = String::with_capacity(256 + 24 * self.samples.len());
        out.push_str("{\n");
        push_member(&mut out, "schema", &SCHEMA.to_string());
        push_member(&mut out, "unit", &format!("\"{UNIT}\""));
        for (name, value) in [
            ("median", self.median),
            ("mad", self.mad),
            ("ci_low", self.ci_low),
            ("ci_high", self.ci_high),
            ("confidence", self.confidence),
        ] {
            push_member(&mut out, name, &number(name, value)?);
        }
        push_member(&mut out, "resamples", &self.resamples.to_string());
        push_member(
            &mut out,
            "iterations_per_sample",
            &self.iterations_per_sample.to_string(),
        );
        let mut samples = String::from("[");
        for (index, &sample) in self.samples.iter().enumerate() {
            if index > 0 {
                samples.push_str(", ");
            }
            samples.push_str(&number("samples", sample)?);
        }
        samples.push(']');
        push_member(&mut out, "samples", &samples);
        let outliers = &self.outliers;
        push_member(
            &mut out,
            "outliers",
            &format!(
                "{{\"low_severe\": {}, \"low_mild\": {}, \"high_mild\": {}, \"high_severe\": {}}}",
                outliers.low_severe, outliers.low_mild, outliers.high_mild, outliers.high_severe
            ),
        );
        let throughput = match self.throughput {
            None => "null".to_owned(),
            Some(Throughput::Bytes(amount)) => {
                format!("{{\"kind\": \"bytes\", \"per_iteration\": {amount}}}")
            }
            Some(Throughput::Elements(amount)) => {
                format!("{{\"kind\": \"elements\", \"per_iteration\": {amount}}}")
            }
        };
        // The last member: no trailing comma.
        let _ = write!(out, "  \"throughput\": {throughput}\n}}\n");
        Ok(out)
    }

    /// Read an estimates file's text.
    pub fn from_json(text: &str) -> Result<Self, EstimatesError> {
        Reader::new(text).estimates()
    }
}

fn push_member(out: &mut String, name: &str, value: &str) {
    let _ = writeln!(out, "  \"{name}\": {value},");
}

/// `value` in shortest round-trip form, or the refusal of a non-finite one.
fn number(member: &str, value: f64) -> Result<String, EstimatesError> {
    if value.is_finite() {
        Ok(value.to_string())
    } else {
        Err(EstimatesError(format!(
            "`{member}` is {value}, which JSON cannot express"
        )))
    }
}

/// A refused estimates file, or a value no file can carry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EstimatesError(String);

impl fmt::Display for EstimatesError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "estimates file: {}", self.0)
    }
}

impl std::error::Error for EstimatesError {}

/// The members of the top-level object, in written order.
const MEMBERS: [&str; 12] = [
    "schema",
    "unit",
    "median",
    "mad",
    "ci_low",
    "ci_high",
    "confidence",
    "resamples",
    "iterations_per_sample",
    "samples",
    "outliers",
    "throughput",
];

/// A cursor over the one fixed schema: objects with known members, numbers,
/// escape-free strings, number arrays and `null`.
struct Reader<'a> {
    text: &'a str,
    at: usize,
}

impl<'a> Reader<'a> {
    const fn new(text: &'a str) -> Self {
        Self { text, at: 0 }
    }

    fn error<T>(&self, message: impl fmt::Display) -> Result<T, EstimatesError> {
        Err(EstimatesError(format!("{message} at byte {}", self.at)))
    }

    fn skip_whitespace(&mut self) {
        let rest = &self.text.as_bytes()[self.at..];
        self.at += rest
            .iter()
            .take_while(|byte| matches!(byte, b' ' | b'\t' | b'\n' | b'\r'))
            .count();
    }

    fn peek(&mut self) -> Option<u8> {
        self.skip_whitespace();
        self.text.as_bytes().get(self.at).copied()
    }

    fn expect(&mut self, byte: u8) -> Result<(), EstimatesError> {
        if self.peek() == Some(byte) {
            self.at += 1;
            Ok(())
        } else {
            self.error(format_args!("expected `{}`", char::from(byte)))
        }
    }

    /// A string with no escapes and no control characters.
    fn string(&mut self) -> Result<&'a str, EstimatesError> {
        self.expect(b'"')?;
        let start = self.at;
        let rest = &self.text[start..];
        let Some(length) = rest.find('"') else {
            return self.error("unterminated string");
        };
        let body = &rest[..length];
        if body.contains('\\') || body.chars().any(char::is_control) {
            return self.error("a string with an escape or a control character");
        }
        self.at = start + length + 1;
        Ok(body)
    }

    /// A JSON number's lexeme, checked against the JSON grammar.
    fn lexeme(&mut self) -> Result<&'a str, EstimatesError> {
        self.skip_whitespace();
        let bytes = self.text.as_bytes();
        let start = self.at;
        let mut at = start;
        let digits = |at: &mut usize| {
            let first = *at;
            while bytes.get(*at).is_some_and(u8::is_ascii_digit) {
                *at += 1;
            }
            *at - first
        };
        if bytes.get(at) == Some(&b'-') {
            at += 1;
        }
        let integer_start = at;
        let integer = digits(&mut at);
        if integer == 0 || (integer > 1 && bytes[integer_start] == b'0') {
            return self.error("a malformed number");
        }
        if bytes.get(at) == Some(&b'.') {
            at += 1;
            if digits(&mut at) == 0 {
                return self.error("a malformed number");
            }
        }
        if matches!(bytes.get(at), Some(b'e' | b'E')) {
            at += 1;
            if matches!(bytes.get(at), Some(b'+' | b'-')) {
                at += 1;
            }
            if digits(&mut at) == 0 {
                return self.error("a malformed number");
            }
        }
        self.at = at;
        Ok(&self.text[start..at])
    }

    fn float(&mut self) -> Result<f64, EstimatesError> {
        let lexeme = self.lexeme()?;
        match lexeme.parse::<f64>() {
            Ok(value) if value.is_finite() => Ok(value),
            _ => self.error(format_args!("`{lexeme}` is not a finite number")),
        }
    }

    fn integer(&mut self) -> Result<u64, EstimatesError> {
        let lexeme = self.lexeme()?;
        match lexeme.parse::<u64>() {
            Ok(value) => Ok(value),
            Err(_) => self.error(format_args!("`{lexeme}` is not a non-negative integer")),
        }
    }

    fn floats(&mut self) -> Result<Vec<f64>, EstimatesError> {
        self.expect(b'[')?;
        let mut values = Vec::new();
        if self.peek() == Some(b']') {
            self.at += 1;
            return Ok(values);
        }
        loop {
            values.push(self.float()?);
            match self.peek() {
                Some(b',') => self.at += 1,
                Some(b']') => {
                    self.at += 1;
                    return Ok(values);
                }
                _ => return self.error("expected `,` or `]`"),
            }
        }
    }

    /// An object whose members are exactly `names`, in any order; `member`
    /// reads each value.
    fn object(
        &mut self,
        names: &[&str],
        mut member: impl FnMut(&mut Self, usize) -> Result<(), EstimatesError>,
    ) -> Result<(), EstimatesError> {
        self.expect(b'{')?;
        let mut seen = vec![false; names.len()];
        if self.peek() != Some(b'}') {
            loop {
                let name = self.string()?;
                let Some(index) = names.iter().position(|known| *known == name) else {
                    return self.error(format_args!("unknown member `{name}`"));
                };
                if seen[index] {
                    return self.error(format_args!("member `{name}` given twice"));
                }
                seen[index] = true;
                self.expect(b':')?;
                member(self, index)?;
                match self.peek() {
                    Some(b',') => self.at += 1,
                    Some(b'}') => break,
                    _ => return self.error("expected `,` or `}`"),
                }
            }
        }
        self.expect(b'}')?;
        if let Some(index) = seen.iter().position(|seen| !seen) {
            return self.error(format_args!("member `{}` is missing", names[index]));
        }
        Ok(())
    }

    fn estimates(mut self) -> Result<Estimates, EstimatesError> {
        let mut estimates = Estimates {
            median: 0.0,
            mad: 0.0,
            ci_low: 0.0,
            ci_high: 0.0,
            confidence: 0.0,
            resamples: 0,
            iterations_per_sample: 0,
            samples: Vec::new(),
            outliers: Outliers::default(),
            throughput: None,
        };
        self.object(&MEMBERS, |reader, index| {
            match MEMBERS[index] {
                "schema" => {
                    let schema = reader.integer()?;
                    if schema != SCHEMA {
                        return reader.error(format_args!(
                            "schema {schema}; this reader reads schema {SCHEMA}"
                        ));
                    }
                }
                "unit" => {
                    let unit = reader.string()?;
                    if unit != UNIT {
                        return reader.error(format_args!("unit `{unit}`; the unit is `{UNIT}`"));
                    }
                }
                "median" => estimates.median = reader.float()?,
                "mad" => estimates.mad = reader.float()?,
                "ci_low" => estimates.ci_low = reader.float()?,
                "ci_high" => estimates.ci_high = reader.float()?,
                "confidence" => estimates.confidence = reader.float()?,
                "resamples" => estimates.resamples = reader.integer()?,
                "iterations_per_sample" => estimates.iterations_per_sample = reader.integer()?,
                "samples" => estimates.samples = reader.floats()?,
                "outliers" => {
                    let outliers = &mut estimates.outliers;
                    reader.object(
                        &["low_severe", "low_mild", "high_mild", "high_severe"],
                        |reader, index| {
                            let count = reader.integer()? as usize;
                            match index {
                                0 => outliers.low_severe = count,
                                1 => outliers.low_mild = count,
                                2 => outliers.high_mild = count,
                                _ => outliers.high_severe = count,
                            }
                            Ok(())
                        },
                    )?;
                }
                _ => estimates.throughput = reader.throughput()?,
            }
            Ok(())
        })?;
        if self.peek().is_some() {
            return self.error("text after the estimates object");
        }
        if estimates.samples.is_empty() {
            return self.error("`samples` is empty");
        }
        if estimates.iterations_per_sample == 0 {
            return self.error("`iterations_per_sample` is zero");
        }
        Ok(estimates)
    }

    fn throughput(&mut self) -> Result<Option<Throughput>, EstimatesError> {
        if self.peek() == Some(b'n') {
            if self.text[self.at..].starts_with("null") {
                self.at += 4;
                return Ok(None);
            }
            return self.error("expected `null` or an object");
        }
        let mut kind = "";
        let mut amount = 0;
        self.object(&["kind", "per_iteration"], |reader, index| {
            if index == 0 {
                kind = reader.string()?;
            } else {
                amount = reader.integer()?;
            }
            Ok(())
        })?;
        match kind {
            "bytes" => Ok(Some(Throughput::Bytes(amount))),
            "elements" => Ok(Some(Throughput::Elements(amount))),
            other => self.error(format_args!(
                "throughput kind `{other}`; the kinds are `bytes` and `elements`"
            )),
        }
    }
}
