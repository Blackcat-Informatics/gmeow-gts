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

def expect_diagnostic(label, json, expected_code)
  decoded = JSON.parse(json)
  diagnostics = decoded.fetch("diagnostics")
  return if diagnostics.any? { |diagnostic| diagnostic.fetch("code") == expected_code }

  raise "#{label} missing diagnostic #{expected_code}."
end

def expect_gts_error(label, expected_status)
  yield
rescue Gmeow::Gts::Error => error
  raise "#{label} expected #{Gmeow::Gts.status_name(expected_status)}, got #{error.status_name}." unless error.status == expected_status
  raise "#{label} structured error did not include code and detail." if error.code.empty? || error.detail.empty?

  return
else
  raise "#{label} did not fail with #{Gmeow::Gts.status_name(expected_status)}."
end

unless ARGV.length == 3
  warn "usage: ruby -I ruby/lib ruby/tests/smoke.rb vectors/01-minimal.gts vectors/04-damaged-frame.gts vectors/28-empty-file.gts"
  exit 2
end

gts = Gmeow::Gts.load
raise "Unexpected ABI version: #{gts.abi_version}" unless gts.abi_version == Gmeow::Gts::ABI_VERSION
raise "Empty library version." if gts.version.empty?

input = File.binread(ARGV.fetch(0))
damaged = File.binread(ARGV.fetch(1))
empty = File.binread(ARGV.fetch(2))

expect_json_property("build metadata", gts.build_metadata_json, "schema", "gts-capi-build-v1")
expect_json_property("capabilities", gts.capabilities_json, "schema", "gts-capi-capabilities-v1")
clean_read = gts.read_json(input)
expect_json_property("ruby clean-read read JSON", clean_read, "schema", "gts-capi-read-v1")
expect_json_property("ruby clean-read read JSON", clean_read, "clean", true)
expect_json_property("verify JSON", gts.verify_json(input), "schema", "gts-capi-verify-v1")

damaged_read = gts.read_json(damaged)
expect_json_property("ruby damaged-diagnostic-read read JSON", damaged_read, "schema", "gts-capi-read-v1")
expect_json_property("ruby damaged-diagnostic-read read JSON", damaged_read, "clean", false)
expect_diagnostic("ruby damaged-diagnostic-read read JSON", damaged_read, "DamagedFrame")
expect_gts_error("ruby damaged-diagnostic-read to_nquads", Gmeow::Gts::Status::DIAGNOSTIC) do
  gts.to_nquads(damaged)
end

empty_read = gts.read_json(empty)
expect_json_property("ruby empty-malformed-refusal read JSON", empty_read, "schema", "gts-capi-read-v1")
expect_json_property("ruby empty-malformed-refusal read JSON", empty_read, "clean", false)
expect_diagnostic("ruby empty-malformed-refusal read JSON", empty_read, "EmptyFile")
expect_gts_error("ruby empty-malformed-refusal to_nquads", Gmeow::Gts::Status::DIAGNOSTIC) do
  gts.to_nquads(empty)
end

nquads = gts.to_nquads(input)
expect_contains("N-Quads", nquads, '"Cat"@en')

round_trip = gts.from_nquads(nquads)
raise "Round-trip GTS output was empty." if round_trip.empty?

expect_gts_error("ruby malformed-nquads-refusal from_nquads", Gmeow::Gts::Status::PARSE) do
  gts.from_nquads(ENV.fetch("GTS_WRAPPER_BAD_NQUADS", "<https://example/s> <https://example/p> .\n"))
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
