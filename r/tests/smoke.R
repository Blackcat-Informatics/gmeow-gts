# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0

args <- commandArgs(trailingOnly = TRUE)
if (length(args) != 1L) {
  message("Skipping external GTS smoke test; pass a vector path as the sole argument to run it.")
  quit(save = "no", status = 0L, runLast = FALSE)
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

vector_path <- args[[1]]
input <- readBin(vector_path, what = "raw", n = file.info(vector_path)$size)

if (!identical(abi_version(), 1L)) {
  stop("unexpected ABI version", call. = FALSE)
}
if (identical(libgts_version(), "")) {
  stop("empty library version", call. = FALSE)
}

expect_json_string("build metadata", build_metadata_json(), "schema", "gts-capi-build-v1")
expect_json_string("capabilities", capabilities_json(), "schema", "gts-capi-capabilities-v1")
expect_json_string("read JSON", read_json(input), "schema", "gts-capi-read-v1")
expect_json_string("verify JSON", verify_json(input), "schema", "gts-capi-verify-v1")

nquads <- to_nquads(input)
expect_contains("N-Quads", nquads, '"Cat"@en')

round_trip <- from_nquads(nquads)
if (!is.raw(round_trip) || length(round_trip) == 0L) {
  stop("round-trip GTS output was empty", call. = FALSE)
}

error <- tryCatch(
  from_nquads("<https://example/s> <https://example/p> .\n"),
  error = identity
)
if (!inherits(error, "gmeowgts_error")) {
  stop("bad N-Quads did not raise a structured gmeowgts_error", call. = FALSE)
}
if (!identical(error$status, status[["PARSE"]])) {
  stop(sprintf("expected parse status, got %s", error$status_name), call. = FALSE)
}
if (identical(error$code, "") || identical(error$detail, "")) {
  stop("structured error did not include code and detail", call. = FALSE)
}

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
