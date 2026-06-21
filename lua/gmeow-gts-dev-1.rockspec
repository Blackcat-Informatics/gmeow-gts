-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
-- SPDX-License-Identifier: MIT OR Apache-2.0

package = "gmeow-gts"
version = "dev-1"

source = {
  url = "git+https://github.com/Blackcat-Informatics/gmeow-gts.git"
}

description = {
  summary = "LuaJIT FFI wrapper for the GTS C ABI",
  detailed = [[
    Thin LuaJIT FFI bindings over the Rust-backed libgts C ABI. The module
    copies and frees C ABI buffers/errors internally and exposes Lua strings,
    tables, and structured error objects to callers.
  ]],
  homepage = "https://blackcatinformatics.ca/projects/gts",
  license = "MIT OR Apache-2.0"
}

dependencies = {
  "lua >= 5.1"
}

build = {
  type = "builtin",
  modules = {
    ["gmeow.gts"] = "gmeow/gts.lua"
  }
}
