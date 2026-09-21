#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

"""Independent integer reference for the complete BM25F rounding law.

No PurRDF import, float, tolerance, or final-only rational rounding. This small
program reproduces every specified truncation, using Python's unbounded
integers. Run with --check to verify the committed corpus, or without arguments
to print the independently computed rows for review.
"""

from pathlib import Path
import sys

S = 10**12
I = 10**18
K1 = 12 * S // 10


def mul(a, b):
    return a * b // S


def div(a, b):
    return a * S // b


def ln(x):
    """The profile's 20-term integer series, for the IDF domain x >= 1."""
    assert x >= S
    exponent = 0
    while x >= S * 2 ** (exponent + 1):
        exponent += 1
    mantissa = x * (I // S) // 2**exponent
    z = (mantissa - I) * I // (mantissa + I)
    square = z * z // I
    power = z
    series = 0
    for denominator in range(1, 40, 2):
        series += power // denominator
        power = power * square // I
    return (exponent * 693147180559945309 + 2 * series) // (I // S)


def score(population, totals, parameters, frequencies, terms):
    result = 0
    for df, fields in zip(frequencies, terms, strict=True):
        idf = ln(S + div((population - df) * S + S // 2, df * S + S // 2))
        pseudo = 0
        for (tf, length), (weight, b), total in zip(fields, parameters, totals, strict=True):
            if tf:
                relative = length * population * S // total
                normalization = S - b + mul(b, relative)
                pseudo += mul(div(tf * S, normalization), weight)
        saturation = div(mul(pseudo, K1 + S), pseudo + K1)
        result += mul(idf, saturation)
    return result


def pairs(value):
    return [tuple(map(int, field.split(":"))) for field in value.split(",")]


def verified_rows():
    path = Path(__file__).with_name("bm25f.tsv")
    for line in path.read_text().splitlines():
        if line.startswith("#"):
            yield line
            continue
        name, population, totals, parameters, frequencies, terms, expected = line.split("\t")
        result = score(
            int(population), list(map(int, totals.split(","))), pairs(parameters),
            list(map(int, frequencies.split(","))), [pairs(term) for term in terms.split(";")],
        )
        if "--check" in sys.argv:
            assert result == int(expected), f"{name}: expected {expected}, reference computed {result}"
        yield "\t".join([name, population, totals, parameters, frequencies, terms, str(result)])


if __name__ == "__main__":
    rows = list(verified_rows())
    if "--check" in sys.argv:
        print(f"{sum(not row.startswith('#') for row in rows)} exact BM25F reference vectors verified")
    else:
        print("\n".join(rows))
