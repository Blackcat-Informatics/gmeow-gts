-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
-- SPDX-License-Identifier: MIT OR Apache-2.0

local ffi = require("ffi")
local bit = require("bit")

ffi.cdef([[
typedef unsigned char uint8_t;
typedef unsigned int uint32_t;

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

uint32_t gts_abi_version(void);
const char *gts_version(void);

void gts_buffer_free(gts_buffer *buffer);
void gts_error_free(gts_error *error);
const char *gts_error_code(const gts_error *error);
const char *gts_error_message(const gts_error *error);

gts_status gts_build_metadata_json(gts_buffer *out, gts_error **error);
gts_status gts_capabilities_json(gts_buffer *out, gts_error **error);
gts_status gts_read_json(const char *data, size_t len, gts_buffer *out, gts_error **error);
gts_status gts_verify_json(const char *data, size_t len, gts_buffer *out, gts_error **error);
gts_status gts_to_nquads(const char *data, size_t len, gts_buffer *out, gts_error **error);
gts_status gts_from_nquads(const char *text, size_t len, gts_buffer *out, gts_error **error);

gts_status gts_files_pack(const char *const *paths, size_t path_count, gts_buffer *out, gts_error **error);
gts_status gts_files_unpack(
  const char *data,
  size_t len,
  const char *dest,
  uint32_t flags,
  gts_buffer *out,
  gts_error **error
);
gts_status gts_files_diff_json(
  const char *data,
  size_t len,
  const char *directory,
  gts_buffer *out,
  gts_error **error
);
]])

local M = {}

M.status = {
  OK = 0,
  INVALID_ARGUMENT = 1,
  IO = 2,
  PARSE = 3,
  DIAGNOSTIC = 4,
  INTERNAL = 5,
  PANIC = 6,
}

M.unpack_flags = {
  NONE = 0,
  INCLUDE_SUPPRESSED = bit.lshift(1, 0),
  ALLOW_SYMLINKS = bit.lshift(1, 1),
  ALLOW_SPECIAL = bit.lshift(1, 2),
  SAME_OWNER = bit.lshift(1, 3),
  PRESERVE_SETID = bit.lshift(1, 4),
}

local status_names = {
  [M.status.OK] = "OK",
  [M.status.INVALID_ARGUMENT] = "INVALID_ARGUMENT",
  [M.status.IO] = "IO",
  [M.status.PARSE] = "PARSE",
  [M.status.DIAGNOSTIC] = "DIAGNOSTIC",
  [M.status.INTERNAL] = "INTERNAL",
  [M.status.PANIC] = "PANIC",
}

local error_mt = {}

function error_mt:__tostring()
  local message = string.format("%s failed with %s", self.operation, self.status_name)
  if self.code ~= "" then
    message = message .. " (" .. self.code .. ")"
  end
  if self.detail ~= "" then
    message = message .. ": " .. self.detail
  end
  return message
end

local Library = {}
Library.__index = Library

local function status_name(status)
  return status_names[status] or "UNKNOWN"
end

local function default_library()
  local from_env = os.getenv("GTS_LIBGTS")
  if from_env and from_env ~= "" then
    return from_env
  end
  if ffi.os == "Windows" then
    return "gts.dll"
  end
  if ffi.os == "OSX" then
    return "libgts.dylib"
  end
  return "libgts.so"
end

local function copy_c_string(value)
  if value == nil then
    return ""
  end
  return ffi.string(value)
end

local function copy_buffer(buffer)
  local len = tonumber(buffer.len)
  if len == 0 then
    return ""
  end
  if buffer.data == nil then
    error("C ABI returned a null data pointer with non-zero length", 0)
  end
  return ffi.string(buffer.data, len)
end

local function raise_structured(operation, status, code, detail)
  error(setmetatable({
    operation = operation,
    status = status,
    status_name = status_name(status),
    code = code or "",
    detail = detail or "",
  }, error_mt), 0)
end

function Library:_build_error(operation, status, err)
  local code = ""
  local detail = ""
  if err ~= nil then
    code = copy_c_string(self.C.gts_error_code(err))
    detail = copy_c_string(self.C.gts_error_message(err))
    self.C.gts_error_free(err)
  end
  raise_structured(operation, status, code, detail)
end

function Library:_call_buffer(operation, fn)
  local out = ffi.new("gts_buffer[1]")
  local err = ffi.new("gts_error *[1]")
  local status = tonumber(fn(out, err))

  if status ~= M.status.OK then
    self.C.gts_buffer_free(out)
    self:_build_error(operation, status, err[0])
  end
  if err[0] ~= nil then
    self.C.gts_buffer_free(out)
    self:_build_error(operation, M.status.INTERNAL, err[0])
  end

  local ok, result = pcall(copy_buffer, out[0])
  self.C.gts_buffer_free(out)
  if not ok then
    error(result, 0)
  end
  return result
end

function Library:abi_version()
  return tonumber(self.C.gts_abi_version())
end

function Library:version()
  return copy_c_string(self.C.gts_version())
end

function Library:build_metadata_json()
  return self:_call_buffer("gts_build_metadata_json", function(out, err)
    return self.C.gts_build_metadata_json(out, err)
  end)
end

function Library:capabilities_json()
  return self:_call_buffer("gts_capabilities_json", function(out, err)
    return self.C.gts_capabilities_json(out, err)
  end)
end

function Library:read_json(data)
  assert(type(data) == "string", "data must be a string")
  return self:_call_buffer("gts_read_json", function(out, err)
    return self.C.gts_read_json(data, #data, out, err)
  end)
end

function Library:verify_json(data)
  assert(type(data) == "string", "data must be a string")
  return self:_call_buffer("gts_verify_json", function(out, err)
    return self.C.gts_verify_json(data, #data, out, err)
  end)
end

function Library:to_nquads(data)
  assert(type(data) == "string", "data must be a string")
  return self:_call_buffer("gts_to_nquads", function(out, err)
    return self.C.gts_to_nquads(data, #data, out, err)
  end)
end

function Library:from_nquads(text)
  assert(type(text) == "string", "text must be a string")
  return self:_call_buffer("gts_from_nquads", function(out, err)
    return self.C.gts_from_nquads(text, #text, out, err)
  end)
end

function Library:files_pack(paths)
  assert(type(paths) == "table", "paths must be a table")
  local path_count = 0
  local max_index = 0
  for key in pairs(paths) do
    assert(
      type(key) == "number" and key >= 1 and key % 1 == 0,
      "paths must be a dense 1-based array"
    )
    path_count = path_count + 1
    if key > max_index then
      max_index = key
    end
  end
  assert(path_count > 0, "paths must not be empty")
  assert(max_index == path_count, "paths must be a dense 1-based array")

  local raw = ffi.new("const char *[?]", path_count)
  local keepalive = {}
  for idx = 1, path_count do
    local path = paths[idx]
    assert(type(path) == "string", "path entries must be strings")
    assert(not path:find("%z", 1, true), "path entries must not contain NUL bytes")
    keepalive[idx] = path
    raw[idx - 1] = path
  end
  return self:_call_buffer("gts_files_pack", function(out, err)
    return self.C.gts_files_pack(raw, path_count, out, err)
  end)
end

function Library:files_unpack(data, destination, flags)
  assert(type(data) == "string", "data must be a string")
  assert(type(destination) == "string", "destination must be a string")
  assert(not destination:find("%z", 1, true), "destination must not contain NUL bytes")
  if flags == nil then
    flags = M.unpack_flags.NONE
  else
    assert(type(flags) == "number", "flags must be a number")
    assert(
      flags >= 0 and flags <= 0xffffffff and flags % 1 == 0,
      "flags must be an unsigned 32-bit integer"
    )
  end
  return self:_call_buffer("gts_files_unpack", function(out, err)
    return self.C.gts_files_unpack(data, #data, destination, flags, out, err)
  end)
end

function Library:files_diff_json(data, directory)
  assert(type(data) == "string", "data must be a string")
  assert(type(directory) == "string", "directory must be a string")
  assert(not directory:find("%z", 1, true), "directory must not contain NUL bytes")
  return self:_call_buffer("gts_files_diff_json", function(out, err)
    return self.C.gts_files_diff_json(data, #data, directory, out, err)
  end)
end

function M.default_library()
  return default_library()
end

function M.load(library)
  return setmetatable({ C = ffi.load(library or default_library()) }, Library)
end

function M.status_name(status)
  return status_name(status)
end

return M
