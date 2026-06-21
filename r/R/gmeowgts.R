# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0

status <- c(
  OK = 0L,
  INVALID_ARGUMENT = 1L,
  IO = 2L,
  PARSE = 3L,
  DIAGNOSTIC = 4L,
  INTERNAL = 5L,
  PANIC = 6L
)

unpack_flags <- c(
  NONE = 0L,
  INCLUDE_SUPPRESSED = bitwShiftL(1L, 0L),
  ALLOW_SYMLINKS = bitwShiftL(1L, 1L),
  ALLOW_SPECIAL = bitwShiftL(1L, 2L),
  SAME_OWNER = bitwShiftL(1L, 3L),
  PRESERVE_SETID = bitwShiftL(1L, 4L)
)

abi_version <- function() {
  .Call(c_gmeowgts_abi_version)
}

libgts_version <- function() {
  .Call(c_gmeowgts_version)
}

build_metadata_json <- function() {
  call_result(.Call(c_gmeowgts_build_metadata_json))
}

capabilities_json <- function() {
  call_result(.Call(c_gmeowgts_capabilities_json))
}

read_json <- function(data) {
  call_result(.Call(c_gmeowgts_read_json, checked_raw(data, "data")))
}

verify_json <- function(data) {
  call_result(.Call(c_gmeowgts_verify_json, checked_raw(data, "data")))
}

to_nquads <- function(data) {
  call_result(.Call(c_gmeowgts_to_nquads, checked_raw(data, "data")))
}

from_nquads <- function(text) {
  call_result(.Call(c_gmeowgts_from_nquads, checked_string(text, "text")))
}

files_pack <- function(paths) {
  call_result(.Call(c_gmeowgts_files_pack, checked_string_vector(paths, "paths")))
}

files_unpack <- function(data, destination, flags = unpack_flags[["NONE"]]) {
  call_result(.Call(
    c_gmeowgts_files_unpack,
    checked_raw(data, "data"),
    checked_string(destination, "destination"),
    checked_flags(flags)
  ))
}

files_diff_json <- function(data, directory) {
  call_result(.Call(
    c_gmeowgts_files_diff_json,
    checked_raw(data, "data"),
    checked_string(directory, "directory")
  ))
}

status_name <- function(code) {
  index <- match(as.integer(code), unname(status))
  ifelse(is.na(index), "UNKNOWN", names(status)[index])
}

call_result <- function(result) {
  if (isTRUE(result$ok)) {
    return(result$value)
  }
  stop_gts_error(result)
}

stop_gts_error <- function(result) {
  code <- as.integer(result$status)
  detail <- as.character(result$detail)
  error_code <- as.character(result$code)
  operation <- as.character(result$operation)
  name <- status_name(code)

  message <- sprintf("%s failed with %s", operation, name)
  if (!identical(error_code, "")) {
    message <- sprintf("%s (%s)", message, error_code)
  }
  if (!identical(detail, "")) {
    message <- sprintf("%s: %s", message, detail)
  }

  condition <- structure(
    list(
      message = message,
      call = NULL,
      operation = operation,
      status = code,
      status_name = name,
      code = error_code,
      detail = detail
    ),
    class = c("gmeowgts_error", "error", "condition")
  )
  stop(condition)
}

checked_raw <- function(value, name) {
  if (!is.raw(value)) {
    stop(sprintf("%s must be a raw vector", name), call. = FALSE)
  }
  value
}

checked_string <- function(value, name) {
  if (!is.character(value) || length(value) != 1L || is.na(value)) {
    stop(sprintf("%s must be a non-NA string", name), call. = FALSE)
  }
  value
}

checked_string_vector <- function(value, name) {
  if (!is.character(value) || length(value) == 0L || anyNA(value)) {
    stop(sprintf("%s must be a non-empty character vector without NA entries", name), call. = FALSE)
  }
  value
}

checked_flags <- function(value) {
  if (!is.numeric(value) || length(value) != 1L || is.na(value)) {
    stop("flags must be a single non-NA number", call. = FALSE)
  }
  if (value < 0 || value > .Machine$integer.max || value != as.integer(value)) {
    stop("flags must be a non-negative 32-bit integer", call. = FALSE)
  }
  as.integer(value)
}
