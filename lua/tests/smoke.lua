-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
-- SPDX-License-Identifier: MIT OR Apache-2.0

local gts_module = require("gmeow.gts")

local function read_file(path)
  local handle = assert(io.open(path, "rb"))
  local data = assert(handle:read("*a"))
  assert(handle:close())
  return data
end

local function shell_quote(value)
  return "'" .. value:gsub("'", [["'"']]) .. "'"
end

local function mkdir_p(path)
  local ok = os.execute("mkdir -p " .. shell_quote(path))
  assert(ok == true or ok == 0, "mkdir failed for " .. path)
end

local function rm_rf(path)
  local ok = os.execute("rm -rf " .. shell_quote(path))
  assert(ok == true or ok == 0, "rm failed for " .. path)
end

local function write_file(path, data)
  local handle = assert(io.open(path, "wb"))
  assert(handle:write(data))
  assert(handle:close())
end

local function expect_contains(label, haystack, needle)
  assert(haystack:find(needle, 1, true), label .. " did not contain " .. needle)
end

local function expect_json_string(label, json, key, expected)
  local compact = json:gsub("%s+", "")
  local needle = '"' .. key .. '":"' .. expected .. '"'
  assert(compact:find(needle, 1, true), label .. " missing " .. key .. "=" .. expected)
end

local function expect_json_bool(label, json, key, expected)
  local compact = json:gsub("%s+", "")
  local literal = expected and "true" or "false"
  local needle = '"' .. key .. '":' .. literal
  assert(compact:find(needle, 1, true), label .. " missing " .. key .. "=" .. literal)
end

local function expect_assertion(label, fn, expected)
  local ok, err = pcall(fn)
  assert(not ok, label .. " did not fail")
  expect_contains(label, tostring(err), expected)
end

local function expect_gts_error(label, fn, expected_status)
  local ok, err = pcall(fn)
  assert(not ok, label .. " did not fail")
  assert(type(err) == "table", label .. " structured error was not a table")
  assert(err.status == expected_status, label .. " structured error status mismatch")
  assert(err.code ~= "", label .. " structured error code was empty")
  assert(err.detail ~= "", label .. " structured error detail was empty")
end

local vector = assert(arg[1], "usage: luajit lua/tests/smoke.lua vectors/01-minimal.gts vectors/04-damaged-frame.gts vectors/28-empty-file.gts")
local damaged_vector = assert(arg[2], "missing damaged fixture path")
local empty_vector = assert(arg[3], "missing empty fixture path")
local gts = gts_module.load()

assert(gts:abi_version() == 1, "unexpected ABI version")
assert(gts:version() ~= "", "empty library version")

local input = read_file(vector)
local damaged = read_file(damaged_vector)
local empty = read_file(empty_vector)

expect_json_string("build metadata", gts:build_metadata_json(), "schema", "gts-capi-build-v1")
expect_json_string("capabilities", gts:capabilities_json(), "schema", "gts-capi-capabilities-v1")
local clean_read = gts:read_json(input)
expect_json_string("lua clean-read read JSON", clean_read, "schema", "gts-capi-read-v1")
expect_json_bool("lua clean-read read JSON", clean_read, "clean", true)
expect_json_string("verify JSON", gts:verify_json(input), "schema", "gts-capi-verify-v1")

local damaged_read = gts:read_json(damaged)
expect_json_string("lua damaged-diagnostic-read read JSON", damaged_read, "schema", "gts-capi-read-v1")
expect_json_bool("lua damaged-diagnostic-read read JSON", damaged_read, "clean", false)
expect_contains("lua damaged-diagnostic-read read JSON", damaged_read, [["code":"DamagedFrame"]])
expect_gts_error("lua damaged-diagnostic-read to_nquads", function()
  return gts:to_nquads(damaged)
end, gts_module.status.DIAGNOSTIC)

local empty_read = gts:read_json(empty)
expect_json_string("lua empty-malformed-refusal read JSON", empty_read, "schema", "gts-capi-read-v1")
expect_json_bool("lua empty-malformed-refusal read JSON", empty_read, "clean", false)
expect_contains("lua empty-malformed-refusal read JSON", empty_read, [["code":"EmptyFile"]])
expect_gts_error("lua empty-malformed-refusal to_nquads", function()
  return gts:to_nquads(empty)
end, gts_module.status.DIAGNOSTIC)

local nquads = gts:to_nquads(input)
expect_contains("N-Quads", nquads, [["Cat"@en]])

local round_trip = gts:from_nquads(nquads)
assert(#round_trip > 0, "round-trip GTS output was empty")

expect_gts_error("lua malformed-nquads-refusal from_nquads", function()
  return gts:from_nquads(os.getenv("GTS_WRAPPER_BAD_NQUADS") or "<https://example/s> <https://example/p> .\n")
end, gts_module.status.PARSE)

local tmp = os.tmpname()
os.remove(tmp)
local source_dir = tmp .. "-src"
local unpack_dir = tmp .. "-unpack"

local cleanup_ok, cleanup_err = xpcall(function()
  mkdir_p(source_dir)
  write_file(source_dir .. "/a.txt", "hello\n")

  local packed = gts:files_pack({ source_dir })
  expect_assertion("sparse files_pack paths", function()
    local sparse_paths = { source_dir }
    sparse_paths[3] = source_dir
    return gts:files_pack(sparse_paths)
  end, "dense 1-based array")
  expect_assertion("invalid files_unpack flags", function()
    return gts:files_unpack(packed, unpack_dir, {})
  end, "flags must be a number")
  expect_json_bool("files diff", gts:files_diff_json(packed, source_dir), "clean", true)
  expect_json_bool("files unpack", gts:files_unpack(packed, unpack_dir), "ok", true)
  assert(read_file(unpack_dir .. "/a.txt") == "hello\n", "unpacked file content mismatch")
end, debug.traceback)

rm_rf(source_dir)
rm_rf(unpack_dir)

if not cleanup_ok then
  error(cleanup_err, 0)
end
