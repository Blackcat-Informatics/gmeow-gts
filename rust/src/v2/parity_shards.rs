// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Shard digests and erasure solving of a geometry-1 span (P5 §§6.1–6.4).

use std::{
    collections::{BTreeMap, BTreeSet},
    sync::OnceLock,
};

/// The parity rows of each stripe (P5 6.4). Equal to the row count of `parity::Shape`.
const ROWS: usize = 4;
/// The bytes of a shard digest: the first sixteen of the derived hash.
pub(super) const DIGEST_BYTES: usize = 16;

/// The exponent and logarithm tables of GF(256) under the reducing polynomial `0x11D`.
struct Field {
    powers: [u8; 255],
    logs: [u8; 256],
}

fn field() -> &'static Field {
    static FIELD: OnceLock<Field> = OnceLock::new();
    FIELD.get_or_init(|| {
        let mut powers = [0_u8; 255];
        let mut logs = [0_u8; 256];
        let mut value = 1_u8;
        for (exponent, power) in powers.iter_mut().enumerate() {
            *power = value;
            logs[usize::from(value)] = u8::try_from(exponent).unwrap_or(0);
            let overflow = value & 0x80 != 0;
            value <<= 1;
            if overflow {
                value ^= 0x1d;
            }
        }
        Field { powers, logs }
    })
}

fn multiply(left: u8, right: u8) -> u8 {
    let field = field();
    if left == 0 || right == 0 {
        return 0;
    }
    let exponent = (usize::from(field.logs[usize::from(left)])
        + usize::from(field.logs[usize::from(right)]))
        % 255;
    field.powers[exponent]
}

/// The multiplicative inverse of a nonzero field element.
fn inverse(value: u8) -> u8 {
    let field = field();
    field.powers[(255 - usize::from(field.logs[usize::from(value)])) % 255]
}

/// The Cauchy coefficient of `column` in parity `row`: the inverse of `(255 - row) XOR column`.
///
/// The denominator is never zero: `255 - row` is at least `252` and `column` is below `64`.
fn coefficient(row: usize, column: usize) -> u8 {
    let row = u8::try_from(255 - row).unwrap_or(0);
    let column = u8::try_from(column).unwrap_or(0);
    inverse(row ^ column)
}

fn scale(block: &[u8], factor: u8) -> Vec<u8> {
    block.iter().map(|&byte| multiply(byte, factor)).collect()
}

fn xor_into(target: &mut [u8], source: &[u8]) {
    for (byte, source) in target.iter_mut().zip(source) {
        *byte ^= source;
    }
}

/// The digest of one shard, which the writer computes over the shard padded to its size.
pub(super) fn shard_digest(shard: &[u8]) -> [u8; DIGEST_BYTES] {
    let mut hash = blake3::Hasher::new_derive_key("gts2 shard");
    hash.update(shard);
    let mut digest = [0; DIGEST_BYTES];
    digest.copy_from_slice(&hash.finalize().as_bytes()[..DIGEST_BYTES]);
    digest
}

/// The shard at `index` of `covered`, padded with zeros to `size` bytes.
pub(super) fn shard_at(covered: &[u8], index: usize, size: usize) -> Vec<u8> {
    let start = (index * size).min(covered.len());
    let end = ((index + 1) * size).min(covered.len());
    let mut shard = covered[start..end].to_vec();
    shard.resize(size, 0);
    shard
}

/// The parity contribution of the known columns of a stripe to `row`.
fn combine(columns: &[Option<Vec<u8>>], row: usize, size: usize) -> Vec<u8> {
    let mut sum = vec![0; size];
    for (column, block) in columns.iter().enumerate() {
        if let Some(block) = block {
            xor_into(&mut sum, &scale(block, coefficient(row, column)));
        }
    }
    sum
}

/// Solve the erased data columns of one stripe from its held parity rows (P5 6.4).
///
/// Returns each erased column with its solved bytes, or `None` when the stripe holds more
/// unknowns than its parity rows. The Cauchy coefficients over distinct points make the system
/// invertible, so a stripe within its rows always solves.
fn solve_stripe(
    columns: &[Option<Vec<u8>>],
    blocks: &[Option<&[u8]>],
    size: usize,
) -> Option<Vec<(usize, Vec<u8>)>> {
    let missing: Vec<usize> = columns
        .iter()
        .enumerate()
        .filter_map(|(column, block)| block.is_none().then_some(column))
        .collect();
    if missing.is_empty() {
        return Some(Vec::new());
    }
    let held: Vec<(usize, &[u8])> = blocks
        .iter()
        .enumerate()
        .filter_map(|(row, block)| block.map(|block| (row, block)))
        .take(missing.len())
        .collect();
    if held.len() < missing.len() {
        return None;
    }
    let mut coefficients: Vec<Vec<u8>> = held
        .iter()
        .map(|&(row, _)| {
            missing
                .iter()
                .map(|&column| coefficient(row, column))
                .collect()
        })
        .collect();
    let mut rhs: Vec<Vec<u8>> = held
        .iter()
        .map(|&(row, block)| {
            let mut value = block.to_vec();
            xor_into(&mut value, &combine(columns, row, size));
            value
        })
        .collect();
    eliminate(&mut coefficients, &mut rhs)?;
    Some(missing.into_iter().zip(rhs).collect())
}

/// Gauss-Jordan elimination over GF(256), leaving the solution in `rhs`.
fn eliminate(coefficients: &mut [Vec<u8>], rhs: &mut [Vec<u8>]) -> Option<()> {
    let count = coefficients.len();
    for column in 0..count {
        let pivot = (column..count).find(|&row| coefficients[row][column] != 0)?;
        coefficients.swap(column, pivot);
        rhs.swap(column, pivot);
        let factor = inverse(coefficients[column][column]);
        coefficients[column] = scale(&coefficients[column], factor);
        rhs[column] = scale(&rhs[column], factor);
        for other in 0..count {
            let factor = coefficients[other][column];
            if other == column || factor == 0 {
                continue;
            }
            let pivot_row = coefficients[column].clone();
            for (cell, &value) in coefficients[other].iter_mut().zip(&pivot_row) {
                *cell ^= multiply(factor, value);
            }
            let pivot_rhs = rhs[column].clone();
            xor_into(&mut rhs[other], &scale(&pivot_rhs, factor));
        }
    }
    Some(())
}

/// The shards that a span's parity writes: each solved data shard by its index, and each solved
/// parity shard by its row.
#[derive(Debug, Default, Eq, PartialEq)]
pub(super) struct Solved {
    pub(super) data: BTreeMap<usize, Vec<u8>>,
    pub(super) parity: BTreeMap<usize, Vec<u8>>,
}

/// Solve every erased shard of a span of `covered` bytes with `size`-byte shards.
///
/// `parity` holds the span's parity bytes, `depth` stripes of four rows. The unknowns are the
/// failing data shards in `bad_data` and the failing parity rows in `bad_parity`. Returns `None`
/// when a stripe holds more than four unknowns (`too_many_erasures`).
pub(super) fn solve(
    covered: &[u8],
    parity: &[u8],
    size: usize,
    bad_data: &BTreeSet<usize>,
    bad_parity: &BTreeSet<usize>,
) -> Option<Solved> {
    let depth = (parity.len() / (size * ROWS)).max(1);
    let mut counts = vec![0_usize; depth];
    for &index in bad_data {
        *counts.get_mut(index % depth)? += 1;
    }
    for &row in bad_parity {
        *counts.get_mut(row / ROWS)? += 1;
    }
    if counts.iter().any(|&count| count > ROWS) {
        return None;
    }
    let data_count = covered.len().div_ceil(size);
    let mut solved = Solved::default();
    for (stripe, &count) in counts.iter().enumerate() {
        if count == 0 {
            continue;
        }
        let indices: Vec<usize> = (stripe..data_count).step_by(depth).collect();
        let mut columns: Vec<Option<Vec<u8>>> = indices
            .iter()
            .map(|index| (!bad_data.contains(index)).then(|| shard_at(covered, *index, size)))
            .collect();
        let rows: Vec<usize> = (stripe * ROWS..(stripe + 1) * ROWS).collect();
        let blocks: Vec<Option<&[u8]>> = rows
            .iter()
            .map(|row| (!bad_parity.contains(row)).then(|| &parity[row * size..(row + 1) * size]))
            .collect();
        for (column, shard) in solve_stripe(&columns, &blocks, size)? {
            columns[column] = Some(shard.clone());
            solved.data.insert(indices[column], shard);
        }
        for (local, row) in rows.iter().enumerate() {
            if blocks[local].is_none() {
                solved.parity.insert(*row, combine(&columns, local, size));
            }
        }
    }
    Some(solved)
}
