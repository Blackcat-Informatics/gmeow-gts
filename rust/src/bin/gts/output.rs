// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The process streams of the `gts` program.
//!
//! Text written with these macros goes to stdout or stderr as the CLI contract
//! specifies. Only the three functions below call the print macros, so the print
//! lints are suppressed on exactly those functions. Byte-exact command output is
//! written directly with `std::io::stdout().write_all` in `gts.rs`; that path does
//! not use the print macros and needs no exception.

use std::fmt;

/// Writes a line of result text to stdout, as `println!` does.
///
/// # Panics
///
/// Panics if the write to stdout fails, as `println!` does.
#[expect(
    clippy::print_stdout,
    reason = "this function writes result text to stdout, as the CLI contract requires"
)]
pub(crate) fn stdout_line(args: fmt::Arguments<'_>) {
    println!("{args}");
}

/// Writes result text to stdout without a newline, as `print!` does.
///
/// # Panics
///
/// Panics if the write to stdout fails, as `print!` does.
#[expect(
    clippy::print_stdout,
    reason = "this function writes result text to stdout, as the CLI contract requires"
)]
pub(crate) fn stdout_text(args: fmt::Arguments<'_>) {
    print!("{args}");
}

/// Writes a line for a person to stderr, as `eprintln!` does.
///
/// # Panics
///
/// Panics if the write to stderr fails, as `eprintln!` does.
#[expect(
    clippy::print_stderr,
    reason = "this function writes a line for a person to stderr, as the CLI contract requires"
)]
pub(crate) fn stderr_line(args: fmt::Arguments<'_>) {
    eprintln!("{args}");
}

/// Writes a line of result text to stdout.
macro_rules! stdout_line {
    ($($arg:tt)*) => {
        $crate::output::stdout_line(format_args!($($arg)*))
    };
}

/// Writes result text to stdout without a newline.
macro_rules! stdout_text {
    ($($arg:tt)*) => {
        $crate::output::stdout_text(format_args!($($arg)*))
    };
}

/// Writes a line for a person to stderr.
macro_rules! stderr_line {
    ($($arg:tt)*) => {
        $crate::output::stderr_line(format_args!($($arg)*))
    };
}
