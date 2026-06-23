# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0

args <- commandArgs(trailingOnly = TRUE)
if (length(args) == 0L) {
  message("Skipping external GTS smoke test; pass shared wrapper fixture paths to run it.")
  quit(save = "no", status = 0L, runLast = FALSE)
}
if (length(args) != 3L) {
  message("usage: Rscript r/tests/smoke.R vectors/01-minimal.gts vectors/04-damaged-frame.gts vectors/28-empty-file.gts")
  quit(save = "no", status = 2L, runLast = FALSE)
}

suppressPackageStartupMessages(library(gmeowgts))

expect_contains <- function(label, haystack, needle) {
  if (!grepl(needle, haystack, fixed = TRUE)) {
    stop(sprintf("%s did not contain %s.", label, needle), call. = FALSE)
  }
}

expect_json_string <- function(label, json, key, expected) {
  compact <- gsub("[[:space:]]+", "", json)
  needle <- sprintf('"%s":"%s"', key, expected)
  expect_contains(label, compact, needle)
}

expect_json_bool <- function(label, json, key, expected) {
  compact <- gsub("[[:space:]]+", "", json)
  literal <- if (isTRUE(expected)) "true" else "false"
  needle <- sprintf('"%s":%s', key, literal)
  expect_contains(label, compact, needle)
}

expect_diagnostic <- function(label, json, expected_code) {
  compact <- gsub("[[:space:]]+", "", json)
  needle <- sprintf('"code":"%s"', expected_code)
  expect_contains(label, compact, needle)
}

expect_gts_error <- function(label, expr, expected_status) {
  error <- tryCatch(
    force(expr),
    error = identity
  )
  if (!inherits(error, "gmeowgts_error")) {
    stop(sprintf("%s did not raise a structured gmeowgts_error", label), call. = FALSE)
  }
  if (!identical(error$status, expected_status)) {
    stop(
      sprintf("%s expected %s, got %s", label, status_name(expected_status), error$status_name),
      call. = FALSE
    )
  }
  if (identical(error$code, "") || identical(error$detail, "")) {
    stop(sprintf("%s structured error did not include code and detail", label), call. = FALSE)
  }
}

vector_path <- args[[1]]
input <- readBin(vector_path, what = "raw", n = file.info(vector_path)$size)
damaged_path <- args[[2]]
damaged <- readBin(damaged_path, what = "raw", n = file.info(damaged_path)$size)
empty_path <- args[[3]]
empty <- readBin(empty_path, what = "raw", n = file.info(empty_path)$size)

if (!identical(abi_version(), 1L)) {
  stop("unexpected ABI version", call. = FALSE)
}
if (identical(libgts_version(), "")) {
  stop("empty library version", call. = FALSE)
}

expect_json_string("build metadata", build_metadata_json(), "schema", "gts-capi-build-v1")
expect_json_string("capabilities", capabilities_json(), "schema", "gts-capi-capabilities-v1")
clean_read <- read_json(input)
expect_json_string("r clean-read read JSON", clean_read, "schema", "gts-capi-read-v1")
expect_json_bool("r clean-read read JSON", clean_read, "clean", TRUE)
expect_json_string("verify JSON", verify_json(input), "schema", "gts-capi-verify-v1")

damaged_read <- read_json(damaged)
expect_json_string("r damaged-diagnostic-read read JSON", damaged_read, "schema", "gts-capi-read-v1")
expect_json_bool("r damaged-diagnostic-read read JSON", damaged_read, "clean", FALSE)
expect_diagnostic("r damaged-diagnostic-read read JSON", damaged_read, "DamagedFrame")
expect_gts_error("r damaged-diagnostic-read to_nquads", to_nquads(damaged), status[["DIAGNOSTIC"]])

empty_read <- read_json(empty)
expect_json_string("r empty-malformed-refusal read JSON", empty_read, "schema", "gts-capi-read-v1")
expect_json_bool("r empty-malformed-refusal read JSON", empty_read, "clean", FALSE)
expect_diagnostic("r empty-malformed-refusal read JSON", empty_read, "EmptyFile")
expect_gts_error("r empty-malformed-refusal to_nquads", to_nquads(empty), status[["DIAGNOSTIC"]])

nquads <- to_nquads(input)
expect_contains("N-Quads", nquads, '"Cat"@en')

round_trip <- from_nquads(nquads)
if (!is.raw(round_trip) || length(round_trip) == 0L) {
  stop("round-trip GTS output was empty", call. = FALSE)
}

bad_nquads <- Sys.getenv("GTS_WRAPPER_BAD_NQUADS", "<https://example/s> <https://example/p> .\n")
expect_gts_error("r malformed-nquads-refusal from_nquads", from_nquads(bad_nquads), status[["PARSE"]])

temp <- tempfile("gts-r-smoke-")
source_dir <- file.path(temp, "src")
unpack_dir <- file.path(temp, "unpack")
dir.create(source_dir, recursive = TRUE)
on.exit(unlink(temp, recursive = TRUE, force = TRUE), add = TRUE)
writeBin(charToRaw("hello\n"), file.path(source_dir, "a.txt"))

packed <- files_pack(source_dir)
if (!is.raw(packed) || length(packed) == 0L) {
  stop("files_pack returned empty output", call. = FALSE)
}
expect_json_bool("files diff", files_diff_json(packed, source_dir), "clean", TRUE)
expect_json_bool("files unpack", files_unpack(packed, unpack_dir), "ok", TRUE)

unpacked <- file.path(unpack_dir, "a.txt")
if (!file.exists(unpacked)) {
  stop("unpacked file missing", call. = FALSE)
}
content <- rawToChar(readBin(unpacked, what = "raw", n = file.info(unpacked)$size))
if (!identical(content, "hello\n")) {
  stop("unpacked file content mismatch", call. = FALSE)
}
