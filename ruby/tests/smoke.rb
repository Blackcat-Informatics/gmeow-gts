# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0

require "fileutils"
require "json"
require "tmpdir"

unless ENV["GTS_RUBY_SMOKE_INSTALLED"] == "1"
  $LOAD_PATH.unshift(File.expand_path("../lib", __dir__))
end
require "gmeow/gts"

def expect_json_property(label, json, property, expected)
  decoded = JSON.parse(json)
  raise "#{label} missing JSON property #{property}." unless decoded.key?(property)
  return if decoded.fetch(property) == expected

  raise "#{label} JSON property #{property} had an unexpected value."
end

def expect_contains(label, haystack, needle)
  raise "#{label} did not contain #{needle}." unless haystack.include?(needle)
end

unless ARGV.length == 1
  warn "usage: ruby -I ruby/lib ruby/tests/smoke.rb vectors/01-minimal.gts"
  exit 2
end

gts = Gmeow::Gts.load
raise "Unexpected ABI version: #{gts.abi_version}" unless gts.abi_version == Gmeow::Gts::ABI_VERSION
raise "Empty library version." if gts.version.empty?

input = File.binread(ARGV.fetch(0))

expect_json_property("build metadata", gts.build_metadata_json, "schema", "gts-capi-build-v1")
expect_json_property("capabilities", gts.capabilities_json, "schema", "gts-capi-capabilities-v1")
expect_json_property("read JSON", gts.read_json(input), "schema", "gts-capi-read-v1")
expect_json_property("verify JSON", gts.verify_json(input), "schema", "gts-capi-verify-v1")

nquads = gts.to_nquads(input)
expect_contains("N-Quads", nquads, '"Cat"@en')

round_trip = gts.from_nquads(nquads)
raise "Round-trip GTS output was empty." if round_trip.empty?

begin
  gts.from_nquads("<https://example/s> <https://example/p> .\n")
  raise "Bad N-Quads did not fail."
rescue Gmeow::Gts::Error => error
  raise "Expected parse status, got #{error.status_name}." unless error.status == Gmeow::Gts::Status::PARSE
  raise "Structured error did not include code and detail." if error.code.empty? || error.detail.empty?
end

Dir.mktmpdir("gts-ruby-smoke-") do |temp|
  source_dir = File.join(temp, "src")
  unpack_dir = File.join(temp, "unpack")

  FileUtils.mkdir_p(source_dir)
  File.binwrite(File.join(source_dir, "a.txt"), "hello\n")

  packed = gts.files_pack([source_dir])
  expect_json_property("files diff", gts.files_diff_json(packed, source_dir), "clean", true)
  expect_json_property("files unpack", gts.files_unpack(packed, unpack_dir), "ok", true)

  unpacked = File.join(unpack_dir, "a.txt")
  raise "Unpacked file missing." unless File.file?(unpacked)
  raise "Unpacked file content mismatch." unless File.binread(unpacked) == "hello\n"
end
