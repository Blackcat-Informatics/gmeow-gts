# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0

root = File.expand_path(__dir__)
lib = File.join(root, "lib")
version_file = File.join(lib, "gmeow", "gts.rb")
version = File.read(version_file, encoding: "UTF-8")[/^\s*VERSION\s*=\s*"([^"]+)"/, 1]
raise "Unable to find Gmeow::Gts::VERSION in #{version_file}" if version.nil?

Gem::Specification.new do |spec|
  spec.name = "gmeow-gts"
  spec.version = version
  spec.authors = ["Blackcat Informatics"]
  spec.email = ["paudley@blackcatinformatics.ca"]
  spec.summary = "Ruby FFI wrapper for the source-only GTS C ABI."
  spec.description = "A source-only Ruby FFI wrapper over the Rust-backed libgts C ABI. The gem expects libgts to be provided by the host at runtime."
  spec.homepage = "https://blackcatinformatics.ca/projects/gts"
  spec.licenses = ["MIT", "Apache-2.0"]
  spec.required_ruby_version = ">= 3.1"

  spec.metadata = {
    "allowed_push_host" => "https://rubygems.org",
    "changelog_uri" => "https://github.com/Blackcat-Informatics/gmeow-gts/blob/main/CHANGELOG.md",
    "homepage_uri" => spec.homepage,
    "rubygems_mfa_required" => "true",
    "source_code_uri" => "https://github.com/Blackcat-Informatics/gmeow-gts",
    "bug_tracker_uri" => "https://github.com/Blackcat-Informatics/gmeow-gts/issues"
  }

  spec.files = Dir.chdir(root) do
    Dir["README.md", "lib/**/*.rb"]
  end
  spec.require_paths = ["lib"]

  spec.add_dependency "ffi", "~> 1.17"
end
