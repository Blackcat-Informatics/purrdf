// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use super::DATA_COLUMNS;

struct FieldPowers {
    exp: [u8; 255],
    log: [usize; 256],
}

const POWERS: FieldPowers = FieldPowers::new();

impl FieldPowers {
    const fn new() -> Self {
        let mut exp = [0; 255];
        let mut log = [0; 256];
        let mut exponent = 0;
        let mut power = 1_u8;
        while exponent < exp.len() {
            exp[exponent] = power;
            log[power as usize] = exponent;
            power = (power << 1) ^ if power & 0x80 == 0 { 0 } else { 0x1d };
            exponent += 1;
        }
        Self { exp, log }
    }

    const fn inverse(&self, byte: u8) -> u8 {
        self.exp[(255 - self.log[byte as usize]) % 255]
    }

    const fn multiply(&self, left: u8, right: u8) -> u8 {
        if left == 0 || right == 0 {
            0
        } else {
            self.exp[(self.log[left as usize] + self.log[right as usize]) % 255]
        }
    }
}

// All divisors of the Geometry 1 Cauchy matrix are in 192..=255.
static PRODUCTS: [[u8; 256]; DATA_COLUMNS] = inverse_products();

const fn inverse_products() -> [[u8; 256]; DATA_COLUMNS] {
    let mut products = [[0; 256]; DATA_COLUMNS];
    let mut divisor = 0;
    while divisor < products.len() {
        let inverse = POWERS.inverse((192 + divisor) as u8);
        let mut byte = 1;
        while byte < 256 {
            products[divisor][byte] = POWERS.multiply(inverse, byte as u8);
            byte += 1;
        }
        divisor += 1;
    }
    products
}

pub(super) fn add_data_shard(output: &mut [u8], data: &[u8], row: usize, column: usize) {
    let products = cauchy_products(row, column);
    for (destination, &source) in output.iter_mut().zip(data) {
        *destination ^= products[usize::from(source)];
    }
}

pub(super) fn matrix_coefficient(row: usize, column: usize) -> u8 {
    cauchy_products(row, column)[1]
}

fn cauchy_products(row: usize, column: usize) -> &'static [u8; 256] {
    let divisor = (255 - row) ^ column;
    &PRODUCTS[divisor & (DATA_COLUMNS - 1)]
}

pub(super) fn inverse(byte: u8) -> u8 {
    debug_assert_ne!(byte, 0);
    POWERS.inverse(byte)
}

pub(super) fn multiply(left: u8, right: u8) -> u8 {
    POWERS.multiply(left, right)
}
