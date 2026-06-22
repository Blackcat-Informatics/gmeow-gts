/* SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> */
/* SPDX-License-Identifier: MIT OR Apache-2.0 */

#ifndef GTS_H
#define GTS_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define GTS_ABI_VERSION 1u

typedef enum gts_status {
  GTS_STATUS_OK = 0,
  GTS_STATUS_INVALID_ARGUMENT = 1,
  GTS_STATUS_IO = 2,
  GTS_STATUS_PARSE = 3,
  GTS_STATUS_DIAGNOSTIC = 4,
  GTS_STATUS_INTERNAL = 5,
  GTS_STATUS_PANIC = 6
} gts_status;

typedef struct gts_buffer {
  uint8_t *data;
  size_t len;
  size_t capacity;
} gts_buffer;

typedef struct gts_error gts_error;

enum {
  GTS_UNPACK_INCLUDE_SUPPRESSED = 1u << 0,
  GTS_UNPACK_ALLOW_SYMLINKS = 1u << 1,
  GTS_UNPACK_ALLOW_SPECIAL = 1u << 2,
  GTS_UNPACK_SAME_OWNER = 1u << 3,
  GTS_UNPACK_PRESERVE_SETID = 1u << 4
};

uint32_t gts_abi_version(void);
const char *gts_version(void);

void gts_buffer_free(gts_buffer *buffer);
void gts_error_free(gts_error *error);
const char *gts_error_code(const gts_error *error);
const char *gts_error_message(const gts_error *error);

gts_status gts_build_metadata_json(gts_buffer *out, gts_error **error);
gts_status gts_capabilities_json(gts_buffer *out, gts_error **error);
gts_status gts_formats_json(gts_buffer *out, gts_error **error);
gts_status gts_read_json(const uint8_t *data, size_t len, gts_buffer *out, gts_error **error);
gts_status gts_verify_json(const uint8_t *data, size_t len, gts_buffer *out, gts_error **error);
gts_status gts_to_format(const uint8_t *data,
                         size_t len,
                         const char *format,
                         gts_buffer *out,
                         gts_error **error);
gts_status gts_from_format(const char *format,
                           const char *text,
                           size_t len,
                           gts_buffer *out,
                           gts_error **error);
gts_status gts_to_nquads(const uint8_t *data, size_t len, gts_buffer *out, gts_error **error);
gts_status gts_from_nquads(const char *text, size_t len, gts_buffer *out, gts_error **error);

gts_status gts_files_pack(const char *const *paths, size_t path_count, gts_buffer *out, gts_error **error);
gts_status gts_files_unpack(const uint8_t *data,
                            size_t len,
                            const char *dest,
                            uint32_t flags,
                            gts_buffer *out,
                            gts_error **error);
gts_status gts_files_diff_json(const uint8_t *data,
                               size_t len,
                               const char *directory,
                               gts_buffer *out,
                               gts_error **error);

#ifdef __cplusplus
}
#endif

#endif /* GTS_H */
