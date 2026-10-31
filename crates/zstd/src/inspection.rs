// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc.
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Borrowed inspection of the codec's original native error owners.
//!
//! Exhaustive producer patterns keep fields tied to their actual owners. Nested
//! errors are borrowed causes; temporary I/O views emit balanced records. Byte
//! extents retain their units, and signed bit positions and probability lists
//! retain their original values. No inspection path renders or clones a cause.

use purrdf_hash::error::{ErrorValue, ErrorVisitor, StructuredError};

fn visit_io<'a>(
    visitor: &mut dyn ErrorVisitor<'a>,
    name: Option<&'static str>,
    cause: &'a std::io::Error,
) {
    visitor.begin_record(name, "std-io");
    visitor.begin_record(Some("kind"), "std-io-kind");
    visitor.value(
        Some("name"),
        purrdf_hash::error::io_error_kind_name(cause.kind())
            .map_or(ErrorValue::Absent, ErrorValue::Code),
    );
    visitor.value(
        Some("native_discriminant"),
        ErrorValue::Unsigned(cause.kind() as u128),
    );
    visitor.end_record();
    visitor.value(
        Some("raw_os_error"),
        cause.raw_os_error().map_or(ErrorValue::Absent, |code| {
            ErrorValue::Signed(i128::from(code))
        }),
    );
    visitor.end_record();
}

impl StructuredError for crate::decoding::errors::FrameDescriptorError {
    purrdf_hash::error_visit! {
    InvalidFrameContentSizeFlag { got } => "zstd-frame-descriptor-invalid-frame-content-size-flag" {
    "got" => Unsigned(u128::from(*got)),
    },
    }
}

impl StructuredError for crate::decoding::errors::FrameHeaderError {
    purrdf_hash::error_visit! {
    WindowTooBig { got } => "zstd-frame-header-window-too-big" {
    "got" => ByteCount(u128::from(*got)),
    },
    WindowTooSmall { got } => "zstd-frame-header-window-too-small" {
    "got" => ByteCount(u128::from(*got)),
    },
    FrameDescriptorError(cause) => "zstd-frame-header-frame-descriptor-error" {
    "cause" => Cause(cause),
    },
    DictIdTooSmall { got, expected } => "zstd-frame-header-dict-id-too-small" {
    "got" => ByteCount(*got as u128),
    "expected" => ByteCount(*expected as u128),
    },
    MismatchedFrameSize { got, expected } => "zstd-frame-header-mismatched-frame-size" {
    "got" => ByteCount(*got as u128),
    "expected" => ByteCount(u128::from(*expected)),
    },
    FrameSizeIsZero => "zstd-frame-header-frame-size-is-zero" {},
    InvalidFrameSize { got } => "zstd-frame-header-invalid-frame-size" {
    "got" => ByteCount(u128::from(*got)),
    },
    }
}

impl StructuredError for crate::decoding::errors::ReadFrameHeaderError {
    fn visit<'a>(&'a self, visitor: &mut dyn ErrorVisitor<'a>) {
        match self {
            Self::MagicNumberReadError(cause) => {
                visitor.variant("zstd-read-frame-header-magic-number-read-error");
                visit_io(visitor, Some("cause"), cause);
            }
            Self::BadMagicNumber(cause) => {
                visitor.variant("zstd-read-frame-header-bad-magic-number");
                visitor.value(
                    Some("magic_number"),
                    ErrorValue::Unsigned(u128::from(*cause)),
                );
            }
            Self::FrameDescriptorReadError(cause) => {
                visitor.variant("zstd-read-frame-header-frame-descriptor-read-error");
                visit_io(visitor, Some("cause"), cause);
            }
            Self::InvalidFrameDescriptor(cause) => {
                visitor.variant("zstd-read-frame-header-invalid-frame-descriptor");
                visitor.value(Some("cause"), ErrorValue::Cause(cause));
            }
            Self::WindowDescriptorReadError(cause) => {
                visitor.variant("zstd-read-frame-header-window-descriptor-read-error");
                visit_io(visitor, Some("cause"), cause);
            }
            Self::DictionaryIdReadError(cause) => {
                visitor.variant("zstd-read-frame-header-dictionary-id-read-error");
                visit_io(visitor, Some("cause"), cause);
            }
            Self::FrameContentSizeReadError(cause) => {
                visitor.variant("zstd-read-frame-header-frame-content-size-read-error");
                visit_io(visitor, Some("cause"), cause);
            }
            Self::SkipFrame {
                magic_number,
                length,
            } => {
                visitor.variant("zstd-read-frame-header-skip-frame");
                visitor.value(
                    Some("magic_number"),
                    ErrorValue::Unsigned(u128::from(*magic_number)),
                );
                visitor.value(Some("length"), ErrorValue::ByteCount(u128::from(*length)));
            }
        }
    }
}

impl StructuredError for crate::decoding::errors::BlockHeaderReadError {
    fn visit<'a>(&'a self, visitor: &mut dyn ErrorVisitor<'a>) {
        match self {
            Self::ReadError(cause) => {
                visitor.variant("zstd-block-header-read-read-error");
                visit_io(visitor, Some("cause"), cause);
            }
            Self::FoundReservedBlock => {
                visitor.variant("zstd-block-header-read-found-reserved-block");
            }
            Self::BlockTypeError(cause) => {
                visitor.variant("zstd-block-header-read-block-type-error");
                visitor.value(Some("cause"), ErrorValue::Cause(cause));
            }
            Self::BlockSizeError(cause) => {
                visitor.variant("zstd-block-header-read-block-size-error");
                visitor.value(Some("cause"), ErrorValue::Cause(cause));
            }
        }
    }
}

impl StructuredError for crate::decoding::errors::BlockTypeError {
    purrdf_hash::error_visit! {
    InvalidBlocktypeNumber { num } => "zstd-block-type-invalid-blocktype-number" {
    "num" => Unsigned(u128::from(*num)),
    },
    }
}

impl StructuredError for crate::decoding::errors::BlockSizeError {
    purrdf_hash::error_visit! {
    BlockSizeTooLarge { size } => "zstd-block-size-block-size-too-large" {
    "size" => ByteCount(u128::from(*size)),
    },
    }
}

impl StructuredError for crate::decoding::errors::DecompressBlockError {
    fn visit<'a>(&'a self, visitor: &mut dyn ErrorVisitor<'a>) {
        match self {
            Self::BlockContentReadError(cause) => {
                visitor.variant("zstd-decompress-block-block-content-read-error");
                visit_io(visitor, Some("cause"), cause);
            }
            Self::MalformedSectionHeader {
                expected_len,
                remaining_bytes,
            } => {
                visitor.variant("zstd-decompress-block-malformed-section-header");
                visitor.value(
                    Some("expected_len"),
                    ErrorValue::ByteCount(*expected_len as u128),
                );
                visitor.value(
                    Some("remaining_bytes"),
                    ErrorValue::ByteCount(*remaining_bytes as u128),
                );
            }
            Self::DecompressLiteralsError(cause) => {
                visitor.variant("zstd-decompress-block-decompress-literals-error");
                visitor.value(Some("cause"), ErrorValue::Cause(cause));
            }
            Self::LiteralsSectionParseError(cause) => {
                visitor.variant("zstd-decompress-block-literals-section-parse-error");
                visitor.value(Some("cause"), ErrorValue::Cause(cause));
            }
            Self::SequencesHeaderParseError(cause) => {
                visitor.variant("zstd-decompress-block-sequences-header-parse-error");
                visitor.value(Some("cause"), ErrorValue::Cause(cause));
            }
            Self::DecodeSequenceError(cause) => {
                visitor.variant("zstd-decompress-block-decode-sequence-error");
                visitor.value(Some("cause"), ErrorValue::Cause(cause));
            }
            Self::ExecuteSequencesError(cause) => {
                visitor.variant("zstd-decompress-block-execute-sequences-error");
                visitor.value(Some("cause"), ErrorValue::Cause(cause));
            }
        }
    }
}

impl StructuredError for crate::decoding::errors::DecodeBlockContentError {
    fn visit<'a>(&'a self, visitor: &mut dyn ErrorVisitor<'a>) {
        match self {
            Self::DecoderStateIsFailed => {
                visitor.variant("zstd-decode-block-content-decoder-state-is-failed");
            }
            Self::ExpectedHeaderOfPreviousBlock => {
                visitor.variant("zstd-decode-block-content-expected-header-of-previous-block");
            }
            Self::ReadError { step, source } => {
                visitor.variant("zstd-decode-block-content-read-error");
                visitor.value(Some("step"), ErrorValue::Record(step));
                visit_io(visitor, Some("source"), source);
            }
            Self::DecompressBlockError(cause) => {
                visitor.variant("zstd-decode-block-content-decompress-block-error");
                visitor.value(Some("cause"), ErrorValue::Cause(cause));
            }
            Self::BackendOverflow { step } => {
                visitor.variant("zstd-decode-block-content-backend-overflow");
                visitor.value(Some("step"), ErrorValue::Record(step));
            }
        }
    }
}

impl StructuredError for crate::decoding::errors::DecodeBufferError {
    purrdf_hash::error_visit! {
    NotEnoughBytesInDictionary { got, need } => "zstd-decode-buffer-not-enough-bytes-in-dictionary" {
    "got" => ByteCount(*got as u128),
    "need" => ByteCount(*need as u128),
    },
    OffsetTooBig { offset, buf_len } => "zstd-decode-buffer-offset-too-big" {
    "offset" => ByteCount(*offset as u128),
    "buf_len" => ByteCount(*buf_len as u128),
    },
    ZeroOffset => "zstd-decode-buffer-zero-offset" {},
    BackendOverflow => "zstd-decode-buffer-backend-overflow" {},
    OutputBufferOverflow { tail, requested, capacity } => "zstd-decode-buffer-output-buffer-overflow" {
    "tail" => ByteCount(*tail as u128),
    "requested" => ByteCount(*requested as u128),
    "capacity" => ByteCount(*capacity as u128),
    },
    }
}

impl StructuredError for crate::decoding::errors::DictionaryDecodeError {
    purrdf_hash::error_visit! {
    BadMagicNum { got } => "zstd-dictionary-decode-bad-magic-num" {
    "got" => Bytes(got),
    },
    DictionaryTooSmall { got, need } => "zstd-dictionary-decode-dictionary-too-small" {
    "got" => ByteCount(*got as u128),
    "need" => ByteCount(*need as u128),
    },
    ZeroDictionaryId => "zstd-dictionary-decode-zero-dictionary-id" {},
    ZeroRepeatOffsetInDictionary { index } => "zstd-dictionary-decode-zero-repeat-offset-in-dictionary" {
    "index" => Unsigned(u128::from(*index)),
    },
    FSETableError(cause) => "zstd-dictionary-decode-fse-table-error" {
    "cause" => Cause(cause),
    },
    HuffmanTableError(cause) => "zstd-dictionary-decode-huffman-table-error" {
    "cause" => Cause(cause),
    },
    }
}

impl StructuredError for crate::decoding::errors::FrameDecoderError {
    fn visit<'a>(&'a self, visitor: &mut dyn ErrorVisitor<'a>) {
        match self {
            Self::OutputAllocationFailed {
                requested,
                capacity,
                cause,
            } => {
                visitor.variant("zstd-frame-decoder-output-allocation-failed");
                visitor.value(Some("requested"), ErrorValue::ByteCount(*requested as u128));
                visitor.value(Some("capacity"), ErrorValue::ByteCount(*capacity as u128));
                visitor.value(Some("cause"), ErrorValue::Cause(cause));
            }
            Self::ReadFrameHeaderError(cause) => {
                visitor.variant("zstd-frame-decoder-read-frame-header-error");
                visitor.value(Some("cause"), ErrorValue::Cause(cause));
            }
            Self::FrameHeaderError(cause) => {
                visitor.variant("zstd-frame-decoder-frame-header-error");
                visitor.value(Some("cause"), ErrorValue::Cause(cause));
            }
            Self::WindowSizeTooBig { requested } => {
                visitor.variant("zstd-frame-decoder-window-size-too-big");
                visitor.value(
                    Some("requested"),
                    ErrorValue::ByteCount(u128::from(*requested)),
                );
            }
            Self::DictionaryDecodeError(cause) => {
                visitor.variant("zstd-frame-decoder-dictionary-decode-error");
                visitor.value(Some("cause"), ErrorValue::Cause(cause));
            }
            Self::FailedToReadBlockHeader(cause) => {
                visitor.variant("zstd-frame-decoder-failed-to-read-block-header");
                visitor.value(Some("cause"), ErrorValue::Cause(cause));
            }
            Self::FailedToReadBlockBody(cause) => {
                visitor.variant("zstd-frame-decoder-failed-to-read-block-body");
                visitor.value(Some("cause"), ErrorValue::Cause(cause));
            }
            Self::FailedToReadChecksum(cause) => {
                visitor.variant("zstd-frame-decoder-failed-to-read-checksum");
                visit_io(visitor, Some("cause"), cause);
            }
            Self::NotYetInitialized => {
                visitor.variant("zstd-frame-decoder-not-yet-initialized");
            }
            Self::FailedToInitialize(cause) => {
                visitor.variant("zstd-frame-decoder-failed-to-initialize");
                visitor.value(Some("cause"), ErrorValue::Cause(cause));
            }
            Self::FailedToDrainDecodebuffer(cause) => {
                visitor.variant("zstd-frame-decoder-failed-to-drain-decodebuffer");
                visit_io(visitor, Some("cause"), cause);
            }
            Self::FailedToSkipFrame => {
                visitor.variant("zstd-frame-decoder-failed-to-skip-frame");
            }
            Self::TargetTooSmall => {
                visitor.variant("zstd-frame-decoder-target-too-small");
            }
            Self::FrameContentSizeMismatch { declared, produced } => {
                visitor.variant("zstd-frame-decoder-frame-content-size-mismatch");
                visitor.value(
                    Some("declared"),
                    ErrorValue::ByteCount(u128::from(*declared)),
                );
                visitor.value(
                    Some("produced"),
                    ErrorValue::ByteCount(u128::from(*produced)),
                );
            }
            Self::ChecksumMismatch {
                expected,
                calculated,
            } => {
                visitor.variant("zstd-frame-decoder-checksum-mismatch");
                visitor.value(
                    Some("expected"),
                    ErrorValue::Unsigned(u128::from(*expected)),
                );
                visitor.value(
                    Some("calculated"),
                    ErrorValue::Unsigned(u128::from(*calculated)),
                );
            }
            Self::DictNotProvided { dict_id } => {
                visitor.variant("zstd-frame-decoder-dict-not-provided");
                visitor.value(Some("dict_id"), ErrorValue::Unsigned(u128::from(*dict_id)));
            }
            Self::DictIdMismatch { expected, provided } => {
                visitor.variant("zstd-frame-decoder-dict-id-mismatch");
                visitor.value(
                    Some("expected"),
                    ErrorValue::Unsigned(u128::from(*expected)),
                );
                visitor.value(
                    Some("provided"),
                    ErrorValue::Unsigned(u128::from(*provided)),
                );
            }
            Self::DictAlreadyRegistered { dict_id } => {
                visitor.variant("zstd-frame-decoder-dict-already-registered");
                visitor.value(Some("dict_id"), ErrorValue::Unsigned(u128::from(*dict_id)));
            }
        }
    }
}

impl StructuredError for crate::decoding::errors::DecompressLiteralsError {
    purrdf_hash::error_visit! {
    MissingCompressedSize => "zstd-decompress-literals-missing-compressed-size" {},
    MissingNumStreams => "zstd-decompress-literals-missing-num-streams" {},
    GetBitsError(cause) => "zstd-decompress-literals-get-bits-error" {
    "cause" => Cause(cause),
    },
    HuffmanTableError(cause) => "zstd-decompress-literals-huffman-table-error" {
    "cause" => Cause(cause),
    },
    HuffmanDecoderError(cause) => "zstd-decompress-literals-huffman-decoder-error" {
    "cause" => Cause(cause),
    },
    UninitializedHuffmanTable => "zstd-decompress-literals-uninitialized-huffman-table" {},
    MissingBytesForJumpHeader { got } => "zstd-decompress-literals-missing-bytes-for-jump-header" {
    "got" => ByteCount(*got as u128),
    },
    MissingBytesForLiterals { got, needed } => "zstd-decompress-literals-missing-bytes-for-literals" {
    "got" => ByteCount(*got as u128),
    "needed" => ByteCount(*needed as u128),
    },
    ExtraPadding { skipped_bits } => "zstd-decompress-literals-extra-padding" {
    "skipped_bits" => Signed(i128::from(*skipped_bits)),
    },
    BitstreamReadMismatch { read_til, expected } => "zstd-decompress-literals-bitstream-read-mismatch" {
    "read_til" => Signed(*read_til as i128),
    "expected" => Signed(*expected as i128),
    },
    DecodedLiteralCountMismatch { decoded, expected } => "zstd-decompress-literals-decoded-literal-count-mismatch" {
    "decoded" => Unsigned(*decoded as u128),
    "expected" => Unsigned(*expected as u128),
    },
    }
}

impl StructuredError for crate::decoding::errors::ExecuteSequencesError {
    purrdf_hash::error_visit! {
    DecodebufferError(cause) => "zstd-execute-sequences-decodebuffer-error" {
    "cause" => Cause(cause),
    },
    NotEnoughBytesForSequence { wanted, have } => "zstd-execute-sequences-not-enough-bytes-for-sequence" {
    "wanted" => ByteCount(*wanted as u128),
    "have" => ByteCount(*have as u128),
    },
    ZeroOffset => "zstd-execute-sequences-zero-offset" {},
    OutputBufferOverflow { tail, requested, capacity } => "zstd-execute-sequences-output-buffer-overflow" {
    "tail" => ByteCount(*tail as u128),
    "requested" => ByteCount(*requested as u128),
    "capacity" => ByteCount(*capacity as u128),
    },
    }
}

impl StructuredError for crate::decoding::errors::DecodeSequenceError {
    purrdf_hash::error_visit! {
    GetBitsError(cause) => "zstd-decode-sequence-get-bits-error" {
    "cause" => Cause(cause),
    },
    FSEDecoderError(cause) => "zstd-decode-sequence-fse-decoder-error" {
    "cause" => Cause(cause),
    },
    FSETableError(cause) => "zstd-decode-sequence-fse-table-error" {
    "cause" => Cause(cause),
    },
    ExtraPadding { skipped_bits } => "zstd-decode-sequence-extra-padding" {
    "skipped_bits" => Signed(i128::from(*skipped_bits)),
    },
    UnsupportedOffset { offset_code } => "zstd-decode-sequence-unsupported-offset" {
    "offset_code" => Unsigned(u128::from(*offset_code)),
    },
    ZeroOffset => "zstd-decode-sequence-zero-offset" {},
    NotEnoughBytesForNumSequences => "zstd-decode-sequence-not-enough-bytes-for-num-sequences" {},
    ExtraBits { bits_remaining } => "zstd-decode-sequence-extra-bits" {
    "bits_remaining" => Signed(*bits_remaining as i128),
    },
    MissingCompressionMode => "zstd-decode-sequence-missing-compression-mode" {},
    MissingByteForRleLlTable => "zstd-decode-sequence-missing-byte-for-rle-ll-table" {},
    MissingByteForRleOfTable => "zstd-decode-sequence-missing-byte-for-rle-of-table" {},
    MissingByteForRleMlTable => "zstd-decode-sequence-missing-byte-for-rle-ml-table" {},
    InvalidRleCode { axis, code } => "zstd-decode-sequence-invalid-rle-code" {
    "axis" => Code(axis),
    "code" => Unsigned(u128::from(*code)),
    },
    }
}

impl StructuredError for crate::decoding::errors::LiteralsSectionParseError {
    purrdf_hash::error_visit! {
    IllegalLiteralSectionType { got } => "zstd-literals-section-parse-illegal-literal-section-type" {
    "got" => Unsigned(u128::from(*got)),
    },
    GetBitsError(cause) => "zstd-literals-section-parse-get-bits-error" {
    "cause" => Cause(cause),
    },
    NotEnoughBytes { have, need } => "zstd-literals-section-parse-not-enough-bytes" {
    "have" => ByteCount(*have as u128),
    "need" => ByteCount(u128::from(*need)),
    },
    }
}

impl StructuredError for crate::decoding::errors::SequencesHeaderParseError {
    purrdf_hash::error_visit! {
    NotEnoughBytes { need_at_least, got } => "zstd-sequences-header-parse-not-enough-bytes" {
    "need_at_least" => ByteCount(u128::from(*need_at_least)),
    "got" => ByteCount(*got as u128),
    },
    }
}

impl StructuredError for crate::decoding::errors::FSETableError {
    purrdf_hash::error_visit! {
    AccLogIsZero => "zstd-fse-table-acc-log-is-zero" {},
    AccLogTooBig { got, max } => "zstd-fse-table-acc-log-too-big" {
    "got" => Unsigned(u128::from(*got)),
    "max" => Unsigned(u128::from(*max)),
    },
    GetBitsError(cause) => "zstd-fse-table-get-bits-error" {
    "cause" => Cause(cause),
    },
    ProbabilityCounterMismatch { got, expected_sum, symbol_probabilities } => "zstd-fse-table-probability-counter-mismatch" {
    "got" => Unsigned(u128::from(*got)),
    "expected_sum" => Unsigned(u128::from(*expected_sum)),
    "symbol_probabilities" => List(symbol_probabilities.iter().map(|value| ErrorValue::Signed(i128::from(*value)))),
    },
    TooManySymbols { got } => "zstd-fse-table-too-many-symbols" {
    "got" => Unsigned(*got as u128),
    },
    InvalidProbability { value, table_size, accuracy_log } => "zstd-fse-table-invalid-probability" {
    "value" => Signed(i128::from(*value)),
    "table_size" => Unsigned(u128::from(*table_size)),
    "accuracy_log" => Unsigned(u128::from(*accuracy_log)),
    },
    TableInvariantViolation { prob, symbol, num_bits, accuracy_log } => "zstd-fse-table-table-invariant-violation" {
    "prob" => Signed(i128::from(*prob)),
    "symbol" => Unsigned(u128::from(*symbol)),
    "num_bits" => Unsigned(u128::from(*num_bits)),
    "accuracy_log" => Unsigned(u128::from(*accuracy_log)),
    },
    }
}

impl StructuredError for crate::decoding::errors::FSEDecoderError {
    purrdf_hash::error_visit! {
    GetBitsError(cause) => "zstd-fse-decoder-get-bits-error" {
    "cause" => Cause(cause),
    },
    TableIsUninitialized => "zstd-fse-decoder-table-is-uninitialized" {},
    InvalidTableShape { decode_len, accuracy_log } => "zstd-fse-decoder-invalid-table-shape" {
    "decode_len" => Unsigned(*decode_len as u128),
    "accuracy_log" => Unsigned(u128::from(*accuracy_log)),
    },
    }
}

impl StructuredError for crate::decoding::errors::HuffmanTableError {
    purrdf_hash::error_visit! {
    GetBitsError(cause) => "zstd-huffman-table-get-bits-error" {
    "cause" => Cause(cause),
    },
    FSEDecoderError(cause) => "zstd-huffman-table-fse-decoder-error" {
    "cause" => Cause(cause),
    },
    FSETableError(cause) => "zstd-huffman-table-fse-table-error" {
    "cause" => Cause(cause),
    },
    SourceIsEmpty => "zstd-huffman-table-source-is-empty" {},
    NotEnoughBytesForWeights { got_bytes, expected_bytes } => "zstd-huffman-table-not-enough-bytes-for-weights" {
    "got_bytes" => ByteCount(*got_bytes as u128),
    "expected_bytes" => ByteCount(u128::from(*expected_bytes)),
    },
    ExtraPadding { skipped_bits } => "zstd-huffman-table-extra-padding" {
    "skipped_bits" => Signed(i128::from(*skipped_bits)),
    },
    TooManyWeights { got } => "zstd-huffman-table-too-many-weights" {
    "got" => Unsigned(*got as u128),
    },
    MissingWeights => "zstd-huffman-table-missing-weights" {},
    LeftoverIsNotAPowerOf2 { got } => "zstd-huffman-table-leftover-is-not-a-power-of2" {
    "got" => Unsigned(u128::from(*got)),
    },
    NotEnoughBytesToDecompressWeights { have, need } => "zstd-huffman-table-not-enough-bytes-to-decompress-weights" {
    "have" => ByteCount(*have as u128),
    "need" => ByteCount(*need as u128),
    },
    FSETableUsedTooManyBytes { used, available_bytes } => "zstd-huffman-table-fse-table-used-too-many-bytes" {
    "used" => ByteCount(*used as u128),
    "available_bytes" => ByteCount(u128::from(*available_bytes)),
    },
    NotEnoughBytesInSource { got, need } => "zstd-huffman-table-not-enough-bytes-in-source" {
    "got" => ByteCount(*got as u128),
    "need" => ByteCount(*need as u128),
    },
    WeightBiggerThanMaxNumBits { got } => "zstd-huffman-table-weight-bigger-than-max-num-bits" {
    "got" => Unsigned(u128::from(*got)),
    },
    MaxBitsTooHigh { got } => "zstd-huffman-table-max-bits-too-high" {
    "got" => Unsigned(u128::from(*got)),
    },
    }
}

impl StructuredError for crate::decoding::errors::HuffmanDecoderError {
    purrdf_hash::error_visit! {
    GetBitsError(cause) => "zstd-huffman-decoder-get-bits-error" {
    "cause" => Cause(cause),
    },
    }
}

impl StructuredError for crate::decoding::FrameSizeError {
    purrdf_hash::error_visit! {
    Header(cause) => "zstd-frame-size-header" {
    "cause" => Cause(cause),
    },
    Truncated => "zstd-frame-size-truncated" {},
    ReservedBlock => "zstd-frame-size-reserved-block" {},
    OversizedBlock => "zstd-frame-size-oversized-block" {},
    }
}

impl StructuredError for crate::bit_io::GetBitsError {
    purrdf_hash::error_visit! {
    TooManyBits { num_requested_bits, limit } => "zstd-get-bits-too-many-bits" {
    "num_requested_bits" => Unsigned(*num_requested_bits as u128),
    "limit" => Unsigned(u128::from(*limit)),
    },
    NotEnoughRemainingBits { requested, remaining } => "zstd-get-bits-not-enough-remaining-bits" {
    "requested" => Unsigned(*requested as u128),
    "remaining" => Unsigned(*remaining as u128),
    },
    }
}

impl StructuredError for crate::blocks::block::BlockType {
    purrdf_hash::error_visit! {
    Raw => "zstd-block-type-raw" {},
    RLE => "zstd-block-type-rle" {},
    Compressed => "zstd-block-type-compressed" {},
    Reserved => "zstd-block-type-reserved" {},
    }
}

impl std::fmt::Display for crate::decoding::FrameSizeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for crate::decoding::FrameSizeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Header(cause) => Some(cause),
            Self::Truncated | Self::ReservedBlock | Self::OversizedBlock => None,
        }
    }
}
