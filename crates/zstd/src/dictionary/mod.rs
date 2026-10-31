#![allow(
    clippy::unreadable_literal,
    clippy::wildcard_imports,
    reason = "Preserve the pinned upstream codec and kernel spelling; local modifications and source identity are documented in PROVENANCE.md"
)]
#![allow(
    missing_docs,
    missing_debug_implementations,
    reason = "Inherited codec API; source identity and modifications are documented in PROVENANCE.md"
)]
//! Deterministic FastCOVER training and finalized Zstandard dictionary construction.
//!
//! The trainer buffers the supplied corpus and computes directly on its bytes.
//! No reservoir sampler, ambient random generator or filesystem traversal is used.
mod fastcover;

use crate::bit_io::BitWriter;
use crate::blocks::sequence_section::{
    MAX_LITERAL_LENGTH_CODE, MAX_MATCH_LENGTH_CODE, MAX_OFFSET_CODE,
};
use crate::decoding::dictionary::MAGIC_NUM as DICT_MAGIC_NUM;
use crate::decoding::sequence_section_decoder::{LL_MAX_LOG, ML_MAX_LOG, OF_MAX_LOG};
use crate::fse::fse_encoder::{self, build_table_from_symbol_counts};
use crate::huff0::HuffmanTable as HuffmanDecoderTable;
use crate::huff0::huff0_encoder::{HuffmanEncoder, HuffmanTable as HuffmanEncoderTable};
pub use fastcover::{
    DEFAULT_D_CANDIDATES, DEFAULT_F_CANDIDATES, DEFAULT_K_CANDIDATES, FastCoverParams,
    FastCoverTuned,
};
use std::{
    format,
    io::{self, Read},
    // `vec` import covers the `vec![..]` macro used below: this crate is
    // no_std-with-std-feature, so the std prelude isn't pulled in implicitly
    // for top-level items in this module. Removing this import fails the
    // build with `cannot find macro 'vec' in this scope` — verified.
    vec,
    vec::Vec,
};

const MAX_HUFFMAN_STATS_BYTES: usize = 64 * 1024;

/// Exact floor(index * length / sample count), without overflowing usize.
fn stats_sample_index(index: usize, length: usize) -> usize {
    index * (length / MAX_HUFFMAN_STATS_BYTES)
        + index * (length % MAX_HUFFMAN_STATS_BYTES) / MAX_HUFFMAN_STATS_BYTES
}

/// Tuning knobs for pure-Rust FastCOVER training.
#[derive(Debug, Clone)]
pub struct FastCoverOptions {
    pub optimize: bool,
    pub split_point: f64,
    pub accel: usize,
    pub k: usize,
    pub d: usize,
    pub f: u32,
    pub k_candidates: Vec<usize>,
    pub d_candidates: Vec<usize>,
    pub f_candidates: Vec<u32>,
}

impl Default for FastCoverOptions {
    fn default() -> Self {
        Self {
            optimize: true,
            split_point: 0.75,
            accel: 1,
            k: 256,
            d: 8,
            f: 20,
            k_candidates: DEFAULT_K_CANDIDATES.to_vec(),
            d_candidates: DEFAULT_D_CANDIDATES.to_vec(),
            f_candidates: DEFAULT_F_CANDIDATES.to_vec(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct FinalizeOptions {
    pub dict_id: Option<u32>,
}

fn serialize_huffman_table(sample_data: &[u8], raw_content: &[u8]) -> io::Result<Vec<u8>> {
    fn bounded_huffman_stats(data: &[u8]) -> Vec<u8> {
        if data.len() <= MAX_HUFFMAN_STATS_BYTES {
            return data.to_vec();
        }

        let mut stats = Vec::with_capacity(MAX_HUFFMAN_STATS_BYTES);
        for i in 0..MAX_HUFFMAN_STATS_BYTES {
            let idx = stats_sample_index(i, data.len());
            stats.push(data[idx]);
        }
        stats
    }

    let source = if sample_data.len() >= 2 {
        sample_data
    } else {
        raw_content
    };
    let mut stats = bounded_huffman_stats(source);
    if stats.len() < 2 || stats.iter().all(|b| *b == stats[0]) {
        stats = (0u8..=255).collect();
    }

    let table = HuffmanEncoderTable::build_from_data(stats.as_slice());
    let mut writer = BitWriter::new();
    let mut encoder = HuffmanEncoder::new(&table, &mut writer);
    encoder.encode(&[stats[0]], true);
    let encoded = writer.dump();

    let mut decoder = HuffmanDecoderTable::new();
    let table_size = decoder
        .build_decoder(encoded.as_slice())
        .map_err(|e| io::Error::other(format!("failed to decode generated huffman table: {e}")))?;
    Ok(encoded[..table_size as usize].to_vec())
}

fn serialize_fse_table(table: &fse_encoder::FSETable) -> Vec<u8> {
    let mut writer = BitWriter::new();
    table.write_table(&mut writer);
    writer.dump()
}

fn bounded_fse_symbols(data: &[u8], max_symbol: u8) -> Vec<u8> {
    let modulo = u16::from(max_symbol) + 1;
    if data.is_empty() {
        return Vec::from([0u8]);
    }
    if data.len() <= MAX_HUFFMAN_STATS_BYTES {
        return data
            .iter()
            .map(|b| (u16::from(*b) % modulo) as u8)
            .collect();
    }

    let mut out = Vec::with_capacity(MAX_HUFFMAN_STATS_BYTES);
    for i in 0..MAX_HUFFMAN_STATS_BYTES {
        let idx = stats_sample_index(i, data.len());
        out.push((u16::from(data[idx]) % modulo) as u8);
    }
    out
}

fn serialize_fse_table_from_corpus(
    sample_data: &[u8],
    raw_content: &[u8],
    max_symbol: u8,
    max_log: u8,
) -> io::Result<Vec<u8>> {
    fn counts_total_for_source(source: &[u8], max_symbol: u8, counts: &mut [usize]) -> usize {
        counts.fill(0);
        for symbol in bounded_fse_symbols(source, max_symbol) {
            counts[usize::from(symbol)] += 1;
        }
        counts.iter().sum::<usize>()
    }

    let mut counts = vec![0usize; usize::from(max_symbol) + 1];
    let using_sample = !sample_data.is_empty();
    let mut total = counts_total_for_source(
        if using_sample {
            sample_data
        } else {
            raw_content
        },
        max_symbol,
        &mut counts,
    );
    if total <= 1 && using_sample && !raw_content.is_empty() {
        total = counts_total_for_source(raw_content, max_symbol, &mut counts);
    }
    if total <= 1 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "insufficient symbol statistics for FSE table",
        ));
    }
    let table = build_table_from_symbol_counts(&counts, max_log, false);
    Ok(serialize_fse_table(&table))
}

fn finalized_content_budget(
    sample_data: &[u8],
    raw_fallback: &[u8],
    dict_size: usize,
) -> io::Result<usize> {
    let min_content_size = 8usize;
    let huf_len = serialize_huffman_table(sample_data, raw_fallback)?.len();
    let of_len =
        serialize_fse_table_from_corpus(sample_data, raw_fallback, MAX_OFFSET_CODE, OF_MAX_LOG)?
            .len();
    let ml_len = serialize_fse_table_from_corpus(
        sample_data,
        raw_fallback,
        MAX_MATCH_LENGTH_CODE,
        ML_MAX_LOG,
    )?
    .len();
    let ll_len = serialize_fse_table_from_corpus(
        sample_data,
        raw_fallback,
        MAX_LITERAL_LENGTH_CODE,
        LL_MAX_LOG,
    )?
    .len();

    let header_len = DICT_MAGIC_NUM.len() + 4 + huf_len + of_len + ml_len + ll_len + 12;
    let max_content_budget = dict_size.saturating_sub(header_len);
    if max_content_budget < min_content_size {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "dictionary size too small to fit header and offset history",
        ));
    }
    Ok(max_content_budget)
}

fn derive_dict_id(raw_content: &[u8]) -> u32 {
    let mut h = 0xcbf29ce484222325u64;
    for &b in raw_content {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x100000001b3);
    }
    let compliant = (h % ((1u64 << 31) - 32768)) + 32768;
    compliant as u32
}

/// Finalize raw dictionary content into a full zstd dictionary binary
/// (`magic + dict_id + entropy tables + offset history + content`).
pub fn finalize_raw_dict(
    raw_content: &[u8],
    sample_data: &[u8],
    dict_size: usize,
    options: FinalizeOptions,
) -> io::Result<Vec<u8>> {
    if raw_content.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "raw dictionary content must not be empty",
        ));
    }
    let mut out = Vec::with_capacity(dict_size.max(256));
    out.extend_from_slice(&DICT_MAGIC_NUM);
    let dict_id = options
        .dict_id
        .unwrap_or_else(|| derive_dict_id(raw_content));
    if dict_id == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "dictionary id must be non-zero",
        ));
    }
    out.extend_from_slice(&dict_id.to_le_bytes());
    out.extend_from_slice(serialize_huffman_table(sample_data, raw_content)?.as_slice());
    out.extend_from_slice(
        serialize_fse_table_from_corpus(sample_data, raw_content, MAX_OFFSET_CODE, OF_MAX_LOG)?
            .as_slice(),
    );
    out.extend_from_slice(
        serialize_fse_table_from_corpus(
            sample_data,
            raw_content,
            MAX_MATCH_LENGTH_CODE,
            ML_MAX_LOG,
        )?
        .as_slice(),
    );
    out.extend_from_slice(
        serialize_fse_table_from_corpus(
            sample_data,
            raw_content,
            MAX_LITERAL_LENGTH_CODE,
            LL_MAX_LOG,
        )?
        .as_slice(),
    );

    // Repeat offsets: keep default bootstrap history.
    out.extend_from_slice(&1u32.to_le_bytes());
    out.extend_from_slice(&4u32.to_le_bytes());
    out.extend_from_slice(&8u32.to_le_bytes());

    let min_content_size = 8usize;
    let max_content_budget = dict_size.saturating_sub(out.len());
    if max_content_budget < min_content_size {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "dictionary size too small to fit header and offset history",
        ));
    }

    let content = if raw_content.len() > max_content_budget {
        &raw_content[raw_content.len() - max_content_budget..]
    } else {
        raw_content
    };
    if content.len() < min_content_size {
        out.resize(out.len() + (min_content_size - content.len()), 0);
    }
    out.extend_from_slice(content);
    Ok(out)
}

/// Train a raw FastCOVER dictionary from a source stream.
fn train_fastcover_internal(
    sample: &[u8],
    dict_size: usize,
    options: &FastCoverOptions,
) -> (Vec<u8>, FastCoverTuned) {
    if options.optimize {
        fastcover::optimize_fastcover_raw(
            sample,
            dict_size,
            options.split_point,
            options.accel,
            options.d_candidates.as_slice(),
            options.f_candidates.as_slice(),
            options.k_candidates.as_slice(),
        )
    } else {
        let params = fastcover::normalize_fastcover_params(FastCoverParams {
            k: options.k,
            d: options.d,
            f: options.f,
            accel: options.accel,
        });
        (
            fastcover::train_fastcover_raw(sample, dict_size, params),
            FastCoverTuned {
                k: params.k,
                d: params.d,
                f: params.f,
                accel: params.accel,
                score: 0,
            },
        )
    }
}

/// Train a raw FastCOVER dictionary directly from an in-memory sample.
pub fn train_fastcover_raw_from_slice(
    sample: &[u8],
    dict_size: usize,
    options: &FastCoverOptions,
) -> io::Result<(Vec<u8>, FastCoverTuned)> {
    if sample.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "source stream is empty",
        ));
    }
    let (dict, tuned) = train_fastcover_internal(sample, dict_size, options);
    if dict.is_empty() && dict_size > 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "training sample is too small for FastCOVER",
        ));
    }
    Ok((dict, tuned))
}

/// Train a raw FastCOVER dictionary from a source stream.
///
/// This function fully buffers the entire training corpus into memory via
/// `read_to_end`, which can consume significant RAM for large inputs.
pub fn create_fastcover_raw_dict_from_source<R: Read, W: io::Write>(
    mut source: R,
    output: &mut W,
    dict_size: usize,
    options: &FastCoverOptions,
) -> io::Result<FastCoverTuned> {
    let mut sample = Vec::new();
    source.read_to_end(&mut sample)?;
    let (dict, tuned) = train_fastcover_raw_from_slice(sample.as_slice(), dict_size, options)?;
    output.write_all(dict.as_slice())?;
    Ok(tuned)
}

/// Train and finalize a FastCOVER dictionary in pure Rust.
///
/// This function fully buffers the entire training corpus into memory via
/// `read_to_end`, which can consume significant RAM for large inputs.
pub fn create_fastcover_dict_from_source<R: Read, W: io::Write>(
    mut source: R,
    output: &mut W,
    dict_size: usize,
    fastcover: &FastCoverOptions,
    finalize: FinalizeOptions,
) -> io::Result<FastCoverTuned> {
    let mut sample = Vec::new();
    source.read_to_end(&mut sample)?;
    if sample.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "source stream is empty",
        ));
    }
    let content_budget = finalized_content_budget(sample.as_slice(), sample.as_slice(), dict_size)?;
    let (raw_dict, tuned) =
        train_fastcover_raw_from_slice(sample.as_slice(), content_budget, fastcover)?;

    let finalized = finalize_raw_dict(raw_dict.as_slice(), sample.as_slice(), dict_size, finalize)?;
    output.write_all(finalized.as_slice())?;
    Ok(tuned)
}
