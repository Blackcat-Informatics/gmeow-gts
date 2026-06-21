/* SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> */
/* SPDX-License-Identifier: MIT OR Apache-2.0 */

#include <R.h>
#include <R_ext/Rdynload.h>
#include <Rinternals.h>

#include <stdint.h>
#include <string.h>

#include "gts.h"

static SEXP named_result(int ok, SEXP value, const char *operation, int status, const char *code, const char *detail) {
  PROTECT(value);
  SEXP result = PROTECT(Rf_allocVector(VECSXP, 6));
  SEXP names = PROTECT(Rf_allocVector(STRSXP, 6));

  SET_STRING_ELT(names, 0, Rf_mkChar("ok"));
  SET_STRING_ELT(names, 1, Rf_mkChar("value"));
  SET_STRING_ELT(names, 2, Rf_mkChar("operation"));
  SET_STRING_ELT(names, 3, Rf_mkChar("status"));
  SET_STRING_ELT(names, 4, Rf_mkChar("code"));
  SET_STRING_ELT(names, 5, Rf_mkChar("detail"));

  SET_VECTOR_ELT(result, 0, Rf_ScalarLogical(ok));
  SET_VECTOR_ELT(result, 1, value);
  SET_VECTOR_ELT(result, 2, Rf_mkString(operation));
  SET_VECTOR_ELT(result, 3, Rf_ScalarInteger(status));
  SET_VECTOR_ELT(result, 4, Rf_mkString(code));
  SET_VECTOR_ELT(result, 5, Rf_mkString(detail));
  Rf_setAttrib(result, R_NamesSymbol, names);

  UNPROTECT(3);
  return result;
}

static SEXP success_result(SEXP value) {
  return named_result(1, value, "", GTS_STATUS_OK, "", "");
}

static SEXP error_result(const char *operation, gts_status status, gts_error *gts_err) {
  const char *code = "";
  const char *detail = "";

  if (gts_err != NULL) {
    code = gts_error_code(gts_err);
    detail = gts_error_message(gts_err);
    if (code == NULL) {
      code = "";
    }
    if (detail == NULL) {
      detail = "";
    }
  }

  SEXP result = PROTECT(named_result(0, R_NilValue, operation, status, code, detail));
  if (gts_err != NULL) {
    gts_error_free(gts_err);
  }
  UNPROTECT(1);
  return result;
}

static SEXP copy_text_and_free(gts_buffer *buffer) {
  if (buffer->len > 0 && buffer->data == NULL) {
    gts_buffer_free(buffer);
    Rf_error("C ABI returned a null data pointer with non-zero length");
  }

  size_t len = buffer->len;
  if (len > INT_MAX) {
    gts_buffer_free(buffer);
    Rf_error("C ABI returned a text buffer too large for an R string");
  }

  const char *text = len == 0 ? "" : (const char *)buffer->data;
  SEXP value = PROTECT(Rf_ScalarString(Rf_mkCharLenCE(text, (int)len, CE_UTF8)));
  gts_buffer_free(buffer);
  UNPROTECT(1);
  return value;
}

static SEXP copy_raw_and_free(gts_buffer *buffer) {
  if (buffer->len > 0 && buffer->data == NULL) {
    gts_buffer_free(buffer);
    Rf_error("C ABI returned a null data pointer with non-zero length");
  }

  size_t len = buffer->len;
  if (len > (size_t)R_XLEN_T_MAX) {
    gts_buffer_free(buffer);
    Rf_error("C ABI returned a raw buffer too large for an R vector");
  }

  SEXP value = PROTECT(Rf_allocVector(RAWSXP, (R_xlen_t)len));
  if (len > 0) {
    memcpy(RAW(value), buffer->data, len);
  }
  gts_buffer_free(buffer);
  UNPROTECT(1);
  return value;
}

static SEXP finish_text_call(const char *operation, gts_status status, gts_buffer *out, gts_error *gts_err) {
  if (status != GTS_STATUS_OK) {
    gts_buffer_free(out);
    return error_result(operation, status, gts_err);
  }
  if (gts_err != NULL) {
    gts_buffer_free(out);
    return error_result(operation, GTS_STATUS_INTERNAL, gts_err);
  }
  SEXP value = PROTECT(copy_text_and_free(out));
  SEXP result = PROTECT(success_result(value));
  UNPROTECT(2);
  return result;
}

static SEXP finish_raw_call(const char *operation, gts_status status, gts_buffer *out, gts_error *gts_err) {
  if (status != GTS_STATUS_OK) {
    gts_buffer_free(out);
    return error_result(operation, status, gts_err);
  }
  if (gts_err != NULL) {
    gts_buffer_free(out);
    return error_result(operation, GTS_STATUS_INTERNAL, gts_err);
  }
  SEXP value = PROTECT(copy_raw_and_free(out));
  SEXP result = PROTECT(success_result(value));
  UNPROTECT(2);
  return result;
}

static const uint8_t *raw_input(SEXP value, size_t *len) {
  if (TYPEOF(value) != RAWSXP) {
    Rf_error("data must be a raw vector");
  }
  *len = (size_t)XLENGTH(value);
  return *len == 0 ? NULL : RAW(value);
}

static const char *string_input(SEXP value, const char *name, size_t *len) {
  if (TYPEOF(value) != STRSXP || XLENGTH(value) != 1 || STRING_ELT(value, 0) == NA_STRING) {
    Rf_error("%s must be a non-NA string", name);
  }
  const char *text = Rf_translateCharUTF8(STRING_ELT(value, 0));
  *len = strlen(text);
  return text;
}

static SEXP c_gmeowgts_abi_version(void) {
  return Rf_ScalarInteger((int)gts_abi_version());
}

static SEXP c_gmeowgts_version(void) {
  const char *version = gts_version();
  return Rf_mkString(version == NULL ? "" : version);
}

static SEXP c_gmeowgts_build_metadata_json(void) {
  gts_buffer out = {0};
  gts_error *gts_err = NULL;
  gts_status status = gts_build_metadata_json(&out, &gts_err);
  return finish_text_call("gts_build_metadata_json", status, &out, gts_err);
}

static SEXP c_gmeowgts_capabilities_json(void) {
  gts_buffer out = {0};
  gts_error *gts_err = NULL;
  gts_status status = gts_capabilities_json(&out, &gts_err);
  return finish_text_call("gts_capabilities_json", status, &out, gts_err);
}

static SEXP c_gmeowgts_read_json(SEXP data) {
  size_t len = 0;
  const uint8_t *bytes = raw_input(data, &len);
  gts_buffer out = {0};
  gts_error *gts_err = NULL;
  gts_status status = gts_read_json(bytes, len, &out, &gts_err);
  return finish_text_call("gts_read_json", status, &out, gts_err);
}

static SEXP c_gmeowgts_verify_json(SEXP data) {
  size_t len = 0;
  const uint8_t *bytes = raw_input(data, &len);
  gts_buffer out = {0};
  gts_error *gts_err = NULL;
  gts_status status = gts_verify_json(bytes, len, &out, &gts_err);
  return finish_text_call("gts_verify_json", status, &out, gts_err);
}

static SEXP c_gmeowgts_to_nquads(SEXP data) {
  size_t len = 0;
  const uint8_t *bytes = raw_input(data, &len);
  gts_buffer out = {0};
  gts_error *gts_err = NULL;
  gts_status status = gts_to_nquads(bytes, len, &out, &gts_err);
  return finish_text_call("gts_to_nquads", status, &out, gts_err);
}

static SEXP c_gmeowgts_from_nquads(SEXP text) {
  size_t len = 0;
  const char *nquads = string_input(text, "text", &len);
  gts_buffer out = {0};
  gts_error *gts_err = NULL;
  gts_status status = gts_from_nquads(nquads, len, &out, &gts_err);
  return finish_raw_call("gts_from_nquads", status, &out, gts_err);
}

static SEXP c_gmeowgts_files_pack(SEXP paths) {
  if (TYPEOF(paths) != STRSXP || XLENGTH(paths) == 0) {
    Rf_error("paths must be a non-empty character vector");
  }

  R_xlen_t count = XLENGTH(paths);
  const char **items = (const char **)R_alloc((size_t)count, sizeof(char *));
  for (R_xlen_t index = 0; index < count; index++) {
    if (STRING_ELT(paths, index) == NA_STRING) {
      Rf_error("paths must not contain NA entries");
    }
    items[index] = Rf_translateCharUTF8(STRING_ELT(paths, index));
  }

  gts_buffer out = {0};
  gts_error *gts_err = NULL;
  gts_status status = gts_files_pack(items, (size_t)count, &out, &gts_err);
  return finish_raw_call("gts_files_pack", status, &out, gts_err);
}

static SEXP c_gmeowgts_files_unpack(SEXP data, SEXP destination, SEXP flags) {
  size_t len = 0;
  const uint8_t *bytes = raw_input(data, &len);
  size_t dest_len = 0;
  const char *dest = string_input(destination, "destination", &dest_len);
  (void)dest_len;
  if (TYPEOF(flags) != INTSXP || XLENGTH(flags) != 1 || INTEGER(flags)[0] == NA_INTEGER || INTEGER(flags)[0] < 0) {
    Rf_error("flags must be a single non-negative integer");
  }

  gts_buffer out = {0};
  gts_error *gts_err = NULL;
  gts_status status = gts_files_unpack(bytes, len, dest, (uint32_t)INTEGER(flags)[0], &out, &gts_err);
  return finish_text_call("gts_files_unpack", status, &out, gts_err);
}

static SEXP c_gmeowgts_files_diff_json(SEXP data, SEXP directory) {
  size_t len = 0;
  const uint8_t *bytes = raw_input(data, &len);
  size_t dir_len = 0;
  const char *dir = string_input(directory, "directory", &dir_len);
  (void)dir_len;

  gts_buffer out = {0};
  gts_error *gts_err = NULL;
  gts_status status = gts_files_diff_json(bytes, len, dir, &out, &gts_err);
  return finish_text_call("gts_files_diff_json", status, &out, gts_err);
}

static const R_CallMethodDef call_methods[] = {
    {"c_gmeowgts_abi_version", (DL_FUNC)&c_gmeowgts_abi_version, 0},
    {"c_gmeowgts_version", (DL_FUNC)&c_gmeowgts_version, 0},
    {"c_gmeowgts_build_metadata_json", (DL_FUNC)&c_gmeowgts_build_metadata_json, 0},
    {"c_gmeowgts_capabilities_json", (DL_FUNC)&c_gmeowgts_capabilities_json, 0},
    {"c_gmeowgts_read_json", (DL_FUNC)&c_gmeowgts_read_json, 1},
    {"c_gmeowgts_verify_json", (DL_FUNC)&c_gmeowgts_verify_json, 1},
    {"c_gmeowgts_to_nquads", (DL_FUNC)&c_gmeowgts_to_nquads, 1},
    {"c_gmeowgts_from_nquads", (DL_FUNC)&c_gmeowgts_from_nquads, 1},
    {"c_gmeowgts_files_pack", (DL_FUNC)&c_gmeowgts_files_pack, 1},
    {"c_gmeowgts_files_unpack", (DL_FUNC)&c_gmeowgts_files_unpack, 3},
    {"c_gmeowgts_files_diff_json", (DL_FUNC)&c_gmeowgts_files_diff_json, 2},
    {NULL, NULL, 0},
};

void R_init_gmeowgts(DllInfo *dll) {
  R_registerRoutines(dll, NULL, call_methods, NULL, NULL);
  R_useDynamicSymbols(dll, FALSE);
}
