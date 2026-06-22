/* SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> */
/* SPDX-License-Identifier: MIT OR Apache-2.0 */

#include "gts.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#if defined(_WIN32)
#error "The C smoke example currently expects POSIX temporary-directory helpers."
#endif

#include <ftw.h>
#include <sys/stat.h>
#include <unistd.h>

static char cleanup_path[512];

static int remove_tree_entry(const char *path,
                             const struct stat *info,
                             int typeflag,
                             struct FTW *ftwbuf) {
  (void)info;
  (void)typeflag;
  (void)ftwbuf;
  return remove(path);
}

static void cleanup_temp(void) {
  if (cleanup_path[0] == '\0') {
    return;
  }
  if (nftw(cleanup_path, remove_tree_entry, 64, FTW_DEPTH | FTW_PHYS) != 0) {
    perror("cleanup temp");
  }
  cleanup_path[0] = '\0';
}

static void fail_error(const char *label, gts_status status, gts_error *error) {
  fprintf(stderr, "%s failed with status %d", label, status);
  if (error != NULL) {
    fprintf(stderr, " (%s: %s)", gts_error_code(error), gts_error_message(error));
    gts_error_free(error);
  }
  fputc('\n', stderr);
  exit(1);
}

static unsigned char *read_file(const char *path, size_t *len) {
  FILE *file = fopen(path, "rb");
  if (file == NULL) {
    perror(path);
    exit(1);
  }
  if (fseek(file, 0, SEEK_END) != 0) {
    perror("fseek");
    exit(1);
  }
  long size = ftell(file);
  if (size < 0) {
    perror("ftell");
    exit(1);
  }
  rewind(file);
  unsigned char *data = malloc((size_t)size);
  if (data == NULL && size != 0) {
    perror("malloc");
    exit(1);
  }
  if (fread(data, 1, (size_t)size, file) != (size_t)size) {
    perror("fread");
    exit(1);
  }
  fclose(file);
  *len = (size_t)size;
  return data;
}

static void expect_contains(const char *label, const gts_buffer *buffer, const char *needle) {
  if (memmem(buffer->data, buffer->len, needle, strlen(needle)) == NULL) {
    fprintf(stderr, "%s did not contain %s\n", label, needle);
    exit(1);
  }
}

static void write_text(const char *path, const char *text) {
  FILE *file = fopen(path, "wb");
  if (file == NULL) {
    perror(path);
    exit(1);
  }
  if (fwrite(text, 1, strlen(text), file) != strlen(text)) {
    perror("fwrite");
    exit(1);
  }
  fclose(file);
}

static void checked_snprintf(char *dest, size_t len, const char *fmt, const char *value) {
  int written = snprintf(dest, len, fmt, value);
  if (written < 0 || (size_t)written >= len) {
    fprintf(stderr, "path buffer too small\n");
    exit(1);
  }
}

int main(int argc, char **argv) {
  if (argc != 2) {
    fprintf(stderr, "usage: %s vectors/01-minimal.gts\n", argv[0]);
    return 2;
  }
  if (gts_abi_version() != GTS_ABI_VERSION) {
    fprintf(stderr, "unexpected ABI version\n");
    return 1;
  }
  if (gts_version() == NULL || strlen(gts_version()) == 0) {
    fprintf(stderr, "empty version\n");
    return 1;
  }

  size_t input_len = 0;
  unsigned char *input = read_file(argv[1], &input_len);
  gts_buffer build_metadata = {0};
  gts_buffer capabilities = {0};
  gts_buffer format_registry = {0};
  gts_buffer read_json = {0};
  gts_buffer verify_json = {0};
  gts_buffer nquads = {0};
  gts_buffer roundtrip = {0};
  gts_buffer codec_source = {0};
  gts_error *error = NULL;

  gts_status status = gts_build_metadata_json(&build_metadata, &error);
  if (status != GTS_STATUS_OK) {
    fail_error("gts_build_metadata_json", status, error);
  }
  expect_contains("build_metadata", &build_metadata, "\"schema\":\"gts-capi-build-v1\"");

  status = gts_capabilities_json(&capabilities, &error);
  if (status != GTS_STATUS_OK) {
    fail_error("gts_capabilities_json", status, error);
  }
  expect_contains("capabilities", &capabilities, "\"to_format\"");

  status = gts_formats_json(&format_registry, &error);
  if (status != GTS_STATUS_OK) {
    fail_error("gts_formats_json", status, error);
  }
  expect_contains("format_registry", &format_registry, "\"application/ld+json\"");

  status = gts_read_json(input, input_len, &read_json, &error);
  if (status != GTS_STATUS_OK) {
    fail_error("gts_read_json", status, error);
  }
  expect_contains("read_json", &read_json, "\"schema\":\"gts-capi-read-v1\"");

  status = gts_verify_json(input, input_len, &verify_json, &error);
  if (status != GTS_STATUS_OK) {
    fail_error("gts_verify_json", status, error);
  }
  expect_contains("verify_json", &verify_json, "\"schema\":\"gts-capi-verify-v1\"");

  status = gts_to_nquads(input, input_len, &nquads, &error);
  if (status != GTS_STATUS_OK) {
    fail_error("gts_to_nquads", status, error);
  }
  expect_contains("nquads", &nquads, "\"Cat\"@en");

  status = gts_from_nquads((const char *)nquads.data, nquads.len, &roundtrip, &error);
  if (status != GTS_STATUS_OK) {
    fail_error("gts_from_nquads", status, error);
  }
  if (roundtrip.len == 0) {
    fprintf(stderr, "roundtrip output was empty\n");
    return 1;
  }

  const char bad_nq[] = "<https://example/s> <https://example/p> .\n";
  gts_buffer unused = {0};
  status = gts_from_nquads(bad_nq, strlen(bad_nq), &unused, &error);
  if (status != GTS_STATUS_PARSE || error == NULL) {
    fprintf(stderr, "bad N-Quads did not return structured parse error\n");
    return 1;
  }
  gts_error_free(error);
  error = NULL;

  const char sample_nt[] = "<https://example.test/s> <https://example.test/p> \"Cat\"@en .\n";
  status = gts_from_format("application/n-triples",
                           sample_nt,
                           strlen(sample_nt),
                           &codec_source,
                           &error);
  if (status != GTS_STATUS_OK) {
    fail_error("gts_from_format application/n-triples", status, error);
  }

  const char *codec_ids[] = {"nquads", "ntriples", "turtle", "trig", "rdfxml", "jsonld"};
  for (size_t i = 0; i < sizeof(codec_ids) / sizeof(codec_ids[0]); i++) {
    gts_buffer formatted = {0};
    gts_buffer restored = {0};
    status = gts_to_format(codec_source.data, codec_source.len, codec_ids[i], &formatted, &error);
    if (status != GTS_STATUS_OK) {
      fail_error(codec_ids[i], status, error);
    }
    if (formatted.len == 0) {
      fprintf(stderr, "%s serialization was empty\n", codec_ids[i]);
      return 1;
    }
    status = gts_from_format(codec_ids[i],
                             (const char *)formatted.data,
                             formatted.len,
                             &restored,
                             &error);
    if (status != GTS_STATUS_OK) {
      fail_error(codec_ids[i], status, error);
    }
    if (restored.len == 0) {
      fprintf(stderr, "%s parse output was empty\n", codec_ids[i]);
      return 1;
    }
    gts_buffer_free(&formatted);
    gts_buffer_free(&restored);
  }

  gts_buffer turtle_alias = {0};
  gts_buffer ttl_roundtrip = {0};
  status = gts_to_format(codec_source.data,
                         codec_source.len,
                         "text/turtle; charset=utf-8",
                         &turtle_alias,
                         &error);
  if (status != GTS_STATUS_OK) {
    fail_error("gts_to_format text/turtle", status, error);
  }
  expect_contains("turtle_alias", &turtle_alias, "Cat");
  status = gts_from_format(".ttl",
                           (const char *)turtle_alias.data,
                           turtle_alias.len,
                           &ttl_roundtrip,
                           &error);
  if (status != GTS_STATUS_OK) {
    fail_error("gts_from_format .ttl", status, error);
  }

  status = gts_to_format(codec_source.data,
                         codec_source.len,
                         "application/x-not-rdf",
                         &unused,
                         &error);
  if (status != GTS_STATUS_INVALID_ARGUMENT || error == NULL) {
    fprintf(stderr, "unsupported format did not return structured invalid-argument error\n");
    return 1;
  }
  gts_error_free(error);
  error = NULL;
  gts_buffer_free(&unused);
  gts_buffer_free(&turtle_alias);
  gts_buffer_free(&ttl_roundtrip);

  char temp_template[] = "/tmp/gts-capi-smoke-XXXXXX";
  char *temp = mkdtemp(temp_template);
  if (temp == NULL) {
    perror("mkdtemp");
    return 1;
  }
  checked_snprintf(cleanup_path, sizeof(cleanup_path), "%s", temp);
  if (atexit(cleanup_temp) != 0) {
    fprintf(stderr, "failed to register temp cleanup\n");
    return 1;
  }
  char source_dir[512];
  char unpack_dir[512];
  char file_path[1024];
  checked_snprintf(source_dir, sizeof(source_dir), "%s/src", temp);
  checked_snprintf(unpack_dir, sizeof(unpack_dir), "%s/unpack", temp);
  checked_snprintf(file_path, sizeof(file_path), "%s/a.txt", source_dir);
  if (mkdir(source_dir, 0700) != 0) {
    perror("mkdir source");
    return 1;
  }
  write_text(file_path, "hello\n");

  const char *paths[] = {source_dir};
  gts_buffer packed = {0};
  gts_buffer diff_json = {0};
  gts_buffer unpack_json = {0};
  status = gts_files_pack(paths, 1, &packed, &error);
  if (status != GTS_STATUS_OK) {
    fail_error("gts_files_pack", status, error);
  }
  status = gts_files_diff_json(packed.data, packed.len, source_dir, &diff_json, &error);
  if (status != GTS_STATUS_OK) {
    fail_error("gts_files_diff_json", status, error);
  }
  expect_contains("diff_json", &diff_json, "\"clean\":true");
  status = gts_files_unpack(packed.data, packed.len, unpack_dir, 0, &unpack_json, &error);
  if (status != GTS_STATUS_OK) {
    fail_error("gts_files_unpack", status, error);
  }
  expect_contains("unpack_json", &unpack_json, "\"ok\":true");

  gts_buffer_free(&build_metadata);
  gts_buffer_free(&capabilities);
  gts_buffer_free(&format_registry);
  gts_buffer_free(&read_json);
  gts_buffer_free(&verify_json);
  gts_buffer_free(&nquads);
  gts_buffer_free(&roundtrip);
  gts_buffer_free(&codec_source);
  gts_buffer_free(&packed);
  gts_buffer_free(&diff_json);
  gts_buffer_free(&unpack_json);
  free(input);
  return 0;
}
