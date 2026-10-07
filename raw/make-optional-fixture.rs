// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use std::io::{BufWriter, Write};

fn main() -> std::io::Result<()> {
    let path = std::env::args().nth(1).expect("output fixture path");
    let mut output = BufWriter::new(std::fs::File::create_new(path)?);
    writeln!(output, "@prefix ex: <http://example.org/> .")?;
    for index in 0..200_000 {
        writeln!(output, "ex:s{index} ex:v {index} ; ex:w {index} .")?;
    }
    output.flush()
}
