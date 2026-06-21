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

local vector = assert(arg[1], "usage: luajit lua/tests/smoke.lua vectors/01-minimal.gts")
local gts = gts_module.load()

assert(gts:abi_version() == 1, "unexpected ABI version")
assert(gts:version() ~= "", "empty library version")

local input = read_file(vector)

expect_json_string("build metadata", gts:build_metadata_json(), "schema", "gts-capi-build-v1")
expect_json_string("capabilities", gts:capabilities_json(), "schema", "gts-capi-capabilities-v1")
expect_json_string("read JSON", gts:read_json(input), "schema", "gts-capi-read-v1")
expect_json_string("verify JSON", gts:verify_json(input), "schema", "gts-capi-verify-v1")

local nquads = gts:to_nquads(input)
expect_contains("N-Quads", nquads, [["Cat"@en]])

local round_trip = gts:from_nquads(nquads)
assert(#round_trip > 0, "round-trip GTS output was empty")

local ok, err = pcall(function()
  return gts:from_nquads("<https://example/s> <https://example/p> .\n")
end)
assert(not ok, "bad N-Quads did not fail")
assert(type(err) == "table", "structured error was not a table")
assert(err.status == gts_module.status.PARSE, "structured error status was not parse")
assert(err.code ~= "", "structured error code was empty")
assert(err.detail ~= "", "structured error detail was empty")

local tmp = os.tmpname()
os.remove(tmp)
local source_dir = tmp .. "-src"
local unpack_dir = tmp .. "-unpack"

local cleanup_ok, cleanup_err = xpcall(function()
  mkdir_p(source_dir)
  write_file(source_dir .. "/a.txt", "hello\n")

  local packed = gts:files_pack({ source_dir })
  expect_json_bool("files diff", gts:files_diff_json(packed, source_dir), "clean", true)
  expect_json_bool("files unpack", gts:files_unpack(packed, unpack_dir), "ok", true)
  assert(read_file(unpack_dir .. "/a.txt") == "hello\n", "unpacked file content mismatch")
end, debug.traceback)

rm_rf(source_dir)
rm_rf(unpack_dir)

if not cleanup_ok then
  error(cleanup_err, 0)
end
