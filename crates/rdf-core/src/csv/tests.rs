// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Unit tests of the reader and the writer; every refusal is tested beside a
//! valid neighbour one byte (or one setting) away.

use purrdf_testkit::rng::SplitMix64;

use super::{
    CsvErrorKind, Dialect, Encoding, FieldWriter, LineTerminators, QuoteStyle, Reader,
    StringRecord, Trim, Writer, read_table, write_record_text,
};

/// Every data record of `input`, as owned fields.
fn records(dialect: Dialect<'_>, input: &[u8]) -> Result<Vec<Vec<String>>, CsvErrorKind> {
    let mut reader = Reader::new(dialect, input);
    reader
        .records()
        .map(|record| {
            record
                .map(|record| record.iter().map(str::to_owned).collect())
                .map_err(|error| error.kind().clone())
        })
        .collect()
}

fn rows(fields: &[&[&str]]) -> Vec<Vec<String>> {
    fields
        .iter()
        .map(|row| row.iter().map(|&field| field.to_owned()).collect())
        .collect()
}

fn written(dialect: Dialect<'_>, records: &[&[&str]]) -> String {
    let mut writer = Writer::new(dialect, Vec::new());
    for record in records {
        writer.write_record(*record).expect("writable");
    }
    String::from_utf8(writer.into_inner().expect("in memory")).expect("UTF-8")
}

#[test]
fn the_rfc4180_preset_yields_the_header_row_as_a_record() {
    assert_eq!(
        records(Dialect::RFC4180, b"a,b\r\n1,2\n3,\"x,y\"").expect("valid"),
        rows(&[&["a", "b"], &["1", "2"], &["3", "x,y"]])
    );
}

#[test]
fn a_header_row_is_consumed_by_headers_and_not_yielded_by_records() {
    let mut reader = Reader::new(Dialect::LPG_CSV, b"id,name\n1,Ada\n");
    assert_eq!(
        reader.headers().expect("header").iter().collect::<Vec<_>>(),
        ["id", "name"]
    );
    let data: Vec<StringRecord> = reader.records().collect::<Result<_, _>>().expect("rows");
    assert_eq!(data, [["1", "Ada"].into_iter().collect::<StringRecord>()]);
    // `records` alone consumes the header too.
    assert_eq!(
        records(Dialect::LPG_CSV, b"id,name\n1,Ada\n").expect("valid"),
        rows(&[&["1", "Ada"]])
    );
    // No input: an empty header and no records.
    let mut empty = Reader::new(Dialect::LPG_CSV, b"");
    assert!(empty.headers().expect("empty").is_empty());
    assert_eq!(empty.records().count(), 0);
}

#[test]
fn the_sssom_preset_is_tab_delimited_and_flexible() {
    assert_eq!(
        records(
            Dialect::SSSOM_TSV,
            b"s\tp\to\nex:a\tex:b\n\"q\tq\"\tx\ty\tz\n"
        )
        .expect("valid"),
        rows(&[&["ex:a", "ex:b"], &["q\tq", "x", "y", "z"]])
    );
}

#[test]
fn a_byte_order_mark_is_removed_and_nothing_else_is() {
    assert_eq!(
        records(Dialect::RFC4180, b"\xEF\xBB\xBFa,b\n").expect("valid"),
        rows(&[&["a", "b"]])
    );
    // A second mark is text.
    assert_eq!(
        records(Dialect::RFC4180, "\u{FEFF}\u{FEFF}a\n".as_bytes()).expect("valid"),
        rows(&[&["\u{FEFF}a"]])
    );
}

#[test]
fn rows_end_at_crlf_or_lf_and_a_lone_cr_is_text() {
    assert_eq!(
        records(Dialect::RFC4180, b"a\r\nb\nc\rd").expect("valid"),
        rows(&[&["a"], &["b"], &["c\rd"]])
    );
    // An empty line is a record of one empty field (RFC 4180 §2 grammar);
    // the final line break ends the last record and opens no new one.
    assert_eq!(
        records(
            Dialect {
                flexible: true,
                ..Dialect::RFC4180
            },
            b"a,b\n\nc,d\n"
        )
        .expect("valid"),
        rows(&[&["a", "b"], &[""], &["c", "d"]])
    );
    assert_eq!(records(Dialect::RFC4180, b"").expect("valid"), rows(&[]));
    assert_eq!(
        records(Dialect::RFC4180, b"\n").expect("valid"),
        rows(&[&[""]])
    );
}

#[test]
fn a_quoted_value_holds_delimiters_line_breaks_and_doubled_quotes() {
    assert_eq!(
        records(
            Dialect::RFC4180,
            b"\"a,b\",\"c\nd\",\"e\"\"f\",\"\",\"\"\"\"\n"
        )
        .expect("valid"),
        rows(&[&["a,b", "c\nd", "e\"f", "", "\""]])
    );
}

/// CSVW §8 "parse a row": the escape character followed by the quote
/// character appends a quote, in an unquoted cell too.
#[test]
fn a_doubled_quote_in_an_unquoted_cell_is_one_quote() {
    assert_eq!(
        records(Dialect::RFC4180, b"a\"\"b\n").expect("valid"),
        rows(&[&["a\"b"]])
    );
}

#[test]
fn a_quote_inside_an_unquoted_cell_is_refused_beside_its_doubled_neighbour() {
    assert_eq!(
        records(Dialect::RFC4180, b"a\"b\"c,d\n"),
        Err(CsvErrorKind::QuoteInUnquotedField)
    );
    assert_eq!(
        records(Dialect::RFC4180, b"a\"\"bc,d\n").expect("valid"),
        rows(&[&["a\"bc", "d"]])
    );
}

#[test]
fn text_after_a_closing_quote_is_refused_beside_the_delimiter_neighbour() {
    assert_eq!(
        records(Dialect::RFC4180, b"\"a\"x,c\n"),
        Err(CsvErrorKind::TextAfterClosingQuote)
    );
    assert_eq!(
        records(Dialect::RFC4180, b"\"a\",,c\n").expect("valid"),
        rows(&[&["a", "", "c"]])
    );
}

#[test]
fn an_unterminated_quoted_value_is_refused_beside_the_closed_neighbour() {
    assert_eq!(
        records(Dialect::RFC4180, b"x,\"ab"),
        Err(CsvErrorKind::UnterminatedQuote)
    );
    assert_eq!(
        records(Dialect::RFC4180, b"x,\"a\"").expect("valid"),
        rows(&[&["x", "a"]])
    );
}

#[test]
fn bytes_that_are_not_utf8_are_refused_beside_the_ascii_neighbour() {
    let flexible = Dialect {
        flexible: true,
        ..Dialect::RFC4180
    };
    let mut reader = Reader::new(flexible, b"ok\nab,c\xFFd\n");
    let mut records = reader.records();
    assert_eq!(
        records.next().expect("first").expect("valid").get(0),
        Some("ok")
    );
    let error = records.next().expect("second").expect_err("not UTF-8");
    assert_eq!(
        error.kind(),
        &CsvErrorKind::NotUtf8 {
            field: 1,
            valid_up_to: 1
        }
    );
    let position = error.position().expect("positioned");
    assert_eq!((position.row, position.line, position.byte), (2, 2, 7));
    assert!(records.next().is_none(), "the reader stops after an error");
    assert_eq!(
        self::records(flexible, b"ok\nab,c\x7Fd\n").expect("valid"),
        rows(&[&["ok"], &["ab", "c\x7Fd"]])
    );
}

#[test]
fn an_unequal_record_is_refused_unless_the_dialect_is_flexible() {
    assert_eq!(
        records(Dialect::RFC4180, b"a,b\nc;d\n"),
        Err(CsvErrorKind::UnequalLengths {
            expected: 2,
            found: 1
        })
    );
    assert_eq!(
        records(Dialect::RFC4180, b"a,b\nc,d\n").expect("valid"),
        rows(&[&["a", "b"], &["c", "d"]])
    );
    assert_eq!(
        records(
            Dialect {
                flexible: true,
                ..Dialect::RFC4180
            },
            b"a,b\nc;d\n"
        )
        .expect("valid"),
        rows(&[&["a", "b"], &["c;d"]])
    );
    // The header row counts as the first record.
    let mut reader = Reader::new(Dialect::LPG_CSV, b"a,b\nc\n");
    reader.headers().expect("header");
    let error = reader.records().next().expect("row").expect_err("short");
    assert_eq!(error.position().expect("positioned").row, 2);
}

#[test]
fn an_empty_line_terminator_is_refused_beside_a_one_byte_terminator() {
    let empty = Dialect {
        line_terminators: LineTerminators::Strings(&["\n", ""]),
        ..Dialect::RFC4180
    };
    assert_eq!(
        records(empty, b"a\nb"),
        Err(CsvErrorKind::EmptyLineTerminator)
    );
    assert!(matches!(
        read_table(&empty, b"a").map_err(|error| error.kind().clone()),
        Err(CsvErrorKind::EmptyLineTerminator)
    ));
    let one = Dialect {
        line_terminators: LineTerminators::Strings(&["\n", ";"]),
        ..Dialect::RFC4180
    };
    assert_eq!(
        records(one, b"a\nb;c").expect("valid"),
        rows(&[&["a"], &["b"], &["c"]])
    );
}

#[test]
fn dialect_bytes_that_the_scanner_cannot_test_are_refused_beside_valid_ones() {
    let refusals = [
        Dialect {
            delimiter: 0xAC,
            ..Dialect::RFC4180
        },
        Dialect {
            quote_char: Some(0xA2),
            ..Dialect::RFC4180
        },
        Dialect {
            quote_char: Some(b','),
            ..Dialect::RFC4180
        },
        Dialect {
            escape: Some(0xDC),
            ..Dialect::RFC4180
        },
        Dialect {
            escape: Some(b','),
            ..Dialect::RFC4180
        },
    ];
    for dialect in refusals {
        assert!(
            matches!(
                records(dialect, b"a"),
                Err(CsvErrorKind::InvalidDialect { .. })
            ),
            "{dialect:?}"
        );
        let mut writer = Writer::new(dialect, Vec::new());
        assert!(matches!(
            writer
                .write_record(["a"])
                .map_err(|error| error.kind().clone()),
            Err(CsvErrorKind::InvalidDialect { .. })
        ));
    }
    let neighbours = [
        Dialect {
            delimiter: b',',
            ..Dialect::RFC4180
        },
        Dialect {
            quote_char: Some(b'"'),
            ..Dialect::RFC4180
        },
        Dialect {
            quote_char: Some(b'\''),
            ..Dialect::RFC4180
        },
        Dialect {
            escape: Some(b'\\'),
            ..Dialect::RFC4180
        },
        Dialect {
            escape: Some(b'"'),
            ..Dialect::RFC4180
        },
    ];
    for dialect in neighbours {
        assert_eq!(
            records(dialect, b"a").expect("valid"),
            rows(&[&["a"]]),
            "{dialect:?}"
        );
        assert_eq!(written(dialect, &[&["a"]]), "a\n");
    }
}

#[test]
fn an_escape_character_applies_inside_and_outside_quotes() {
    let escaped = Dialect {
        double_quote: false,
        escape: Some(b'\\'),
        flexible: true,
        ..Dialect::RFC4180
    };
    assert_eq!(
        records(escaped, b"a\\,b,\"c\\\"d\",\\\\\ne\\\nf,g\n").expect("valid"),
        rows(&[&["a,b", "c\"d", "\\"], &["e\nf", "g"]])
    );
    // A doubled quote is not an escape here: it closes and re-opens.
    assert_eq!(
        records(escaped, b"\"a\"\"b\""),
        Err(CsvErrorKind::TextAfterClosingQuote)
    );
    // An escape character with nothing after it is kept, not dropped.
    assert_eq!(records(escaped, b"a\\").expect("valid"), rows(&[&["a\\"]]));
    // It escapes a whole character, not a byte.
    assert_eq!(
        records(escaped, "\\é,x".as_bytes()).expect("valid"),
        rows(&[&["é", "x"]])
    );
}

#[test]
fn without_a_quote_character_quotes_are_text() {
    let unquoted = Dialect {
        quote_char: None,
        ..Dialect::RFC4180
    };
    assert_eq!(
        records(unquoted, b"\"a,b\"\n").expect("valid"),
        rows(&[&["\"a", "b\""]])
    );
}

#[test]
fn line_terminators_are_tried_in_the_declared_order() {
    let dialect = Dialect {
        line_terminators: LineTerminators::Strings(&["<><>", "<>", "\u{241E}"]),
        quote_char: None,
        flexible: true,
        ..Dialect::RFC4180
    };
    assert_eq!(
        records(dialect, "a<><>b<>c\u{241E}d".as_bytes()).expect("valid"),
        rows(&[&["a"], &["b"], &["c"], &["d"]])
    );
    // A terminator inside a quoted value is content.
    let cr = Dialect {
        line_terminators: LineTerminators::Strings(&["\r"]),
        ..Dialect::CSVW_CELLS
    };
    let table = read_table(
        &cr,
        b"name,note\rAlice,\"first\nline\rand second\"\rBob,done\r",
    )
    .expect("valid");
    assert_eq!(table.header_rows[0].cells, ["name", "note"]);
    assert_eq!(table.rows.len(), 2);
    assert_eq!(table.rows[0].cells, ["Alice", "first\nline\rand second"]);
    assert_eq!((table.rows[0].number, table.rows[1].number), (2, 3));
    // An empty list: no row ends before the input does.
    let none = Dialect {
        line_terminators: LineTerminators::Strings(&[]),
        ..Dialect::RFC4180
    };
    assert_eq!(
        records(none, b"a\nb,c\r\n").expect("valid"),
        rows(&[&["a\nb", "c\r\n"]])
    );
}

/// CSVW §8's table algorithm: skipped rows and comment rows become comments,
/// comment rows and (when skipped) blank rows are not header rows, skipped
/// columns are removed, and source row numbers count every row read.
#[test]
fn read_table_walks_the_csvw_table_algorithm() {
    let dialect = Dialect {
        skip_rows: 2,
        header_row_count: 1,
        skip_columns: 1,
        skip_blank_rows: true,
        ..Dialect::CSVW_CELLS
    };
    let input = b"generated by hand\n\n#note\n,\nx,name,age\ny,Ada,36\n,,\nz,Bob,\n#tail\n";
    let table = read_table(&dialect, input).expect("valid");
    assert_eq!(table.comments, ["generated by hand", "note", "tail"]);
    assert_eq!(table.header_rows.len(), 1);
    assert_eq!(table.header_rows[0].number, 5);
    assert_eq!(table.header_rows[0].cells, ["name", "age"]);
    let data: Vec<(usize, Vec<String>)> = table
        .rows
        .iter()
        .map(|row| (row.number, row.cells.clone()))
        .collect();
    assert_eq!(
        data,
        [
            (6, vec!["Ada".to_owned(), "36".to_owned()]),
            (8, vec!["Bob".to_owned(), String::new()]),
        ]
    );
    // Without skip blank rows the blank rows are header and data rows.
    let keep = Dialect {
        skip_blank_rows: false,
        ..dialect
    };
    let kept = read_table(&keep, input).expect("valid");
    assert_eq!(kept.header_rows[0].cells, [""]);
    assert_eq!(kept.rows.len(), 4);
}

#[test]
fn trim_removes_unicode_white_space_as_asked() {
    let cell = "\u{3000} a \u{A0}";
    let input = format!("\"{cell}\",{cell}\n");
    for (trim, skip_initial_space, expected) in [
        (Trim::None, false, cell),
        (Trim::Start, false, "a \u{A0}"),
        (Trim::End, false, "\u{3000} a"),
        (Trim::Both, false, "a"),
        (Trim::None, true, "a \u{A0}"),
        (Trim::End, true, "a"),
    ] {
        let dialect = Dialect {
            skip_initial_space,
            trim,
            ..Dialect::RFC4180
        };
        assert_eq!(
            records(dialect, input.as_bytes()).expect("valid"),
            rows(&[&[expected, expected]]),
            "{trim:?} {skip_initial_space}"
        );
    }
    // ZERO WIDTH SPACE is not White_Space.
    let both = Dialect {
        trim: Trim::Both,
        ..Dialect::RFC4180
    };
    assert_eq!(
        records(both, "\u{200B}a\n".as_bytes()).expect("valid"),
        rows(&[&["\u{200B}a"]])
    );
}

#[test]
fn an_error_names_its_row_line_column_and_byte() {
    let mut reader = Reader::new(Dialect::RFC4180, "é,\"x\nyz\"q\n".as_bytes());
    let error = reader.records().next().expect("row").expect_err("refused");
    assert_eq!(error.kind(), &CsvErrorKind::TextAfterClosingQuote);
    let position = error.position().expect("positioned");
    assert_eq!(
        (position.row, position.line, position.column, position.byte),
        (1, 2, 4, 9)
    );
    assert_eq!(
        error.to_string(),
        "row 1 (line 2, column 4, byte 9): a closing quote is followed by text other than the delimiter"
    );
}

#[test]
fn encoding_labels_are_the_utf8_labels_of_the_encoding_standard() {
    for label in [
        "utf-8",
        "UTF8",
        " unicode-1-1-utf-8\n",
        "unicode11utf8",
        "unicode20utf8",
        "x-unicode20utf8",
    ] {
        assert_eq!(
            Encoding::from_label(label),
            Some(Encoding::Utf8),
            "{label:?}"
        );
    }
    for label in ["utf-16", "latin1", "utf-8x", "", "utf\u{A0}8"] {
        assert_eq!(Encoding::from_label(label), None, "{label:?}");
    }
}

#[test]
fn a_field_is_quoted_only_when_it_must_be() {
    assert_eq!(
        written(
            Dialect::RFC4180,
            &[&[
                "plain",
                "a,b",
                "say \"hi\"",
                "line\nbreak",
                "cr\r",
                " spaced "
            ]]
        ),
        "plain,\"a,b\",\"say \"\"hi\"\"\",\"line\nbreak\",\"cr\r\", spaced \n"
    );
    // One empty field is written `""`, so it is not read back as a blank row;
    // two empty fields need no quotes.
    assert_eq!(written(Dialect::RFC4180, &[&[""]]), "\"\"\n");
    assert_eq!(written(Dialect::RFC4180, &[&["", ""]]), ",\n");
    assert_eq!(
        written(Dialect::SPARQL_RESULTS, &[&["x"], &["1"]]),
        "x\r\n1\r\n"
    );
    let always = Dialect {
        quote_style: QuoteStyle::Always,
        ..Dialect::RFC4180
    };
    assert_eq!(written(always, &[&["a", ""]]), "\"a\",\"\"\n");
}

#[test]
fn a_first_field_that_begins_with_the_comment_prefix_is_quoted() {
    let dialect = Dialect {
        comment_prefix: Some("#"),
        quote_style: QuoteStyle::Necessary,
        ..Dialect::CSVW_CELLS
    };
    let text = written(dialect, &[&["#x", "#y"], &["z", "#"]]);
    assert_eq!(text, "\"#x\",#y\nz,#\n");
    let table = read_table(
        &Dialect {
            header_row_count: 0,
            ..dialect
        },
        text.as_bytes(),
    )
    .expect("valid");
    assert_eq!(table.comments, Vec::<String>::new());
    assert_eq!(table.rows[0].cells, ["#x", "#y"]);
}

#[test]
fn an_escape_dialect_writes_escapes() {
    let escaped = Dialect {
        double_quote: false,
        escape: Some(b'\\'),
        ..Dialect::RFC4180
    };
    let text = written(escaped, &[&["a\"b", "c\\d", "e"]]);
    assert_eq!(text, "\"a\\\"b\",\"c\\\\d\",e\n");
    assert_eq!(
        records(escaped, text.as_bytes()).expect("valid"),
        rows(&[&["a\"b", "c\\d", "e"]])
    );
}

#[test]
fn writer_refusals_write_nothing_and_their_neighbours_write() {
    let mut writer = Writer::new(Dialect::RFC4180, Vec::new());
    let empty: [&str; 0] = [];
    assert_eq!(
        writer
            .write_record(empty)
            .map_err(|error| error.kind().clone()),
        Err(CsvErrorKind::EmptyRecord)
    );
    writer.write_record(["a", "b"]).expect("first record");
    assert_eq!(
        writer
            .write_record(["c"])
            .map_err(|error| error.kind().clone()),
        Err(CsvErrorKind::UnequalLengths {
            expected: 2,
            found: 1
        })
    );
    writer.write_record(["c", "d"]).expect("same length");
    assert_eq!(writer.into_inner().expect("in memory"), b"a,b\nc,d\n");

    let unquoted = Dialect {
        quote_char: None,
        ..Dialect::RFC4180
    };
    let mut writer = Writer::new(unquoted, Vec::new());
    assert_eq!(
        writer
            .write_record(["x", "a,b"])
            .map_err(|error| error.kind().clone()),
        Err(CsvErrorKind::UnquotableField { field: 1 })
    );
    writer.write_record(["x", "a;b"]).expect("nothing to quote");
    assert_eq!(writer.into_inner().expect("in memory"), b"x,a;b\n");

    let always_unquoted = Dialect {
        quote_style: QuoteStyle::Always,
        ..unquoted
    };
    let mut writer = Writer::new(always_unquoted, Vec::new());
    assert!(matches!(
        writer
            .write_record(["x"])
            .map_err(|error| error.kind().clone()),
        Err(CsvErrorKind::InvalidDialect { .. })
    ));
    let never = Dialect {
        double_quote: false,
        ..Dialect::RFC4180
    };
    let mut writer = Writer::new(never, Vec::new());
    assert_eq!(
        writer
            .write_record(["a\"b"])
            .map_err(|error| error.kind().clone()),
        Err(CsvErrorKind::UnquotableField { field: 0 })
    );
    writer.write_record(["a'b"]).expect("no quote inside");
}

#[test]
fn a_sink_failure_is_an_io_error() {
    struct Full;
    impl std::io::Write for Full {
        fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::other("full"))
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut writer = Writer::new(Dialect::RFC4180, Full);
    writer.write_record(["a"]).expect("buffered");
    let error = writer.flush().expect_err("the sink fails");
    assert!(matches!(error.kind(), CsvErrorKind::Io { .. }));
    assert!(error.to_string().contains("full"));
}

#[test]
fn the_field_writer_and_the_text_writer_share_the_quoting_rule() {
    const FIELDS: FieldWriter = FieldWriter::new(Dialect::SPARQL_RESULTS);
    let mut out = String::new();
    let variables = vec!["x".to_owned(), "y z".to_owned()];
    FIELDS.write_record(&variables, &mut out);
    assert_eq!(variables.len(), 2);
    FIELDS.write_field("\"", &mut out);
    FIELDS.write_delimiter(&mut out);
    FIELDS.write_field("", &mut out);
    FIELDS.end_record(&mut out);
    let none: [&str; 0] = [];
    FIELDS.write_record(none, &mut out);
    assert_eq!(out, "x,y z\r\n\"\"\"\",\r\n\r\n");

    let mut text = String::new();
    write_record_text(&Dialect::CSVW_CELLS, &["a", "b\"c"], &mut text);
    assert_eq!(text, "\"a\",\"b\"\"c\"\n");
}

/// Seeded records over an alphabet dense in structure, written under each
/// preset and read back: every field survives.
#[test]
fn every_preset_reads_back_what_it_writes() {
    const ALPHABET: &[&str] = &[
        "a", " ", ",", "\t", ";", "\"", "\r", "\n", "\r\n", "\\", "#", "é", "\u{3000}", "\u{A0}",
    ];
    let presets = [
        Dialect::RFC4180,
        Dialect::LPG_CSV,
        Dialect::SSSOM_TSV,
        Dialect::SPARQL_RESULTS,
        Dialect::CSVW_CELLS,
        Dialect {
            double_quote: false,
            escape: Some(b'\\'),
            ..Dialect::RFC4180
        },
    ];
    let mut rng = SplitMix64::new(0x0C5F_2026_0927);
    for dialect in presets {
        for _ in 0..300 {
            let width = 1 + rng.below_usize(4);
            let table: Vec<Vec<String>> = (0..=rng.below_usize(4))
                .map(|_| {
                    (0..width)
                        .map(|_| {
                            (0..rng.below_usize(6))
                                .map(|_| ALPHABET[rng.below_usize(ALPHABET.len())])
                                .collect()
                        })
                        .collect()
                })
                .collect();
            let mut writer = Writer::new(dialect, Vec::new());
            for record in &table {
                writer.write_record(record).expect("writable");
            }
            let bytes = writer.into_inner().expect("in memory");
            let mut reader = Reader::new(
                Dialect {
                    header_row_count: 0,
                    ..dialect
                },
                &bytes,
            );
            let read: Vec<Vec<String>> = reader
                .records()
                .map(|record| record.map(|r| r.iter().map(str::to_owned).collect()))
                .collect::<Result<_, _>>()
                .unwrap_or_else(|error| panic!("{dialect:?} {bytes:?}: {error}"));
            assert_eq!(
                read,
                table,
                "{dialect:?} {:?}",
                String::from_utf8_lossy(&bytes)
            );
        }
    }
}

#[test]
fn a_string_record_behaves_as_its_list_of_fields() {
    let record: StringRecord = ["a", "", "é"].into_iter().collect();
    assert_eq!(record.len(), 3);
    assert_eq!(record.get(1), Some(""));
    assert_eq!(record.get(3), None);
    assert_eq!(record.iter().rev().collect::<Vec<_>>(), ["é", "", "a"]);
    assert_eq!(format!("{record:?}"), r#"StringRecord(["a", "", "é"])"#);
    assert!(StringRecord::new().is_empty());
    let owned: Vec<String> = record.into_iter().collect();
    assert_eq!(owned, ["a", "", "é"]);
}
