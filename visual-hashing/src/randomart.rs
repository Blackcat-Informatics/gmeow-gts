// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! OpenSSH-style "Drunken Bishop" ASCII-art fingerprint.
//!
//! A bishop starts in the centre of a 17×9 grid and makes four moves per input
//! byte (two bits each), incrementing a visit count on every square it lands on.
//! The counts are rendered through a character ramp; the start and end squares
//! are marked `S` and `E`. This is a byte-for-byte port of the Python reference.

const WIDTH: usize = 17;
const HEIGHT: usize = 9;

/// Character ramp indexed by visit count. The first slot (count 0) is blank; the
/// last is reserved for the end square. Matches OpenSSH's `augmentation_string`.
const VALUES: &[u8] = b" .o+=*BOX@%&#/^";

/// Render an OpenSSH-style randomart fingerprint of `data`.
///
/// `label` annotates the header (e.g. `"ED25519 256"`); pass `""` for none. The
/// art is a deterministic function of `data`; `label` only affects the header.
pub fn randomart(data: &[u8], label: &str) -> String {
    let start_x = WIDTH / 2;
    let start_y = HEIGHT / 2;
    let mut grid = [[0u32; WIDTH]; HEIGHT];
    let (mut x, mut y) = (start_x, start_y);

    for &byte in data {
        for shift in [0u32, 2, 4, 6] {
            match (byte >> shift) & 0x3 {
                0 => {
                    y = y.saturating_sub(1);
                    x = x.saturating_sub(1);
                }
                1 => {
                    y = y.saturating_sub(1);
                    x = (x + 1).min(WIDTH - 1);
                }
                2 => {
                    y = (y + 1).min(HEIGHT - 1);
                    x = x.saturating_sub(1);
                }
                _ => {
                    y = (y + 1).min(HEIGHT - 1);
                    x = (x + 1).min(WIDTH - 1);
                }
            }
            grid[y][x] += 1;
        }
    }

    let (end_x, end_y) = (x, y);
    grid[start_y][start_x] = 0;
    grid[end_y][end_x] = (VALUES.len() - 1) as u32;

    let mut lines: Vec<String> = Vec::with_capacity(HEIGHT + 2);
    lines.push(if label.is_empty() {
        "+----------------+".to_string()
    } else {
        // Left-justify the label to a 14-char field, matching Python `{:14s}`.
        format!("+--[{label:<14}]+")
    });

    for (row_idx, row) in grid.iter().enumerate() {
        let mut line = String::with_capacity(WIDTH + 2);
        line.push('|');
        for (col_idx, &count) in row.iter().enumerate() {
            if row_idx == start_y && col_idx == start_x {
                line.push('S');
            } else if row_idx == end_y && col_idx == end_x {
                line.push('E');
            } else {
                line.push(VALUES[(count as usize).min(VALUES.len() - 1)] as char);
            }
        }
        line.push('|');
        lines.push(line);
    }

    lines.push("+----------------+".to_string());
    lines.join("\n")
}
