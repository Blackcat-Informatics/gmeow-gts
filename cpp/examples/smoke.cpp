// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

#include "gts/gts.hpp"

#include <chrono>
#include <cstdint>
#include <cstdlib>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <stdexcept>
#include <string>
#include <vector>

namespace {

class TempDir final {
public:
  TempDir() {
    auto nonce = std::chrono::steady_clock::now().time_since_epoch().count();
    path_ = std::filesystem::temp_directory_path() / ("gts-cpp-smoke-" + std::to_string(nonce));
    std::filesystem::create_directories(path_);
  }

  TempDir(const TempDir &) = delete;
  TempDir &operator=(const TempDir &) = delete;

  ~TempDir() {
    std::error_code error;
    std::filesystem::remove_all(path_, error);
  }

  const std::filesystem::path &path() const noexcept { return path_; }

private:
  std::filesystem::path path_;
};

gts::Bytes read_file(const std::filesystem::path &path) {
  std::ifstream input(path, std::ios::binary);
  if (!input) {
    throw std::runtime_error("failed to open " + path.string());
  }
  return gts::Bytes(std::istreambuf_iterator<char>(input), std::istreambuf_iterator<char>());
}

void write_text(const std::filesystem::path &path, const std::string &text) {
  std::ofstream output(path, std::ios::binary);
  if (!output) {
    throw std::runtime_error("failed to open " + path.string());
  }
  output << text;
}

void expect_contains(const std::string &label, const std::string &haystack, const std::string &needle) {
  if (haystack.find(needle) == std::string::npos) {
    throw std::runtime_error(label + " did not contain " + needle);
  }
}

template <typename Fn> void expect_gts_error(const std::string &label, Fn &&fn, gts_status expected) {
  try {
    fn();
    throw std::runtime_error(label + " did not fail");
  } catch (const gts::Error &error) {
    if (error.status() != expected) {
      throw std::runtime_error(label + " returned unexpected status " +
                               std::string(gts::status_name(error.status())));
    }
    if (error.code().empty() || error.message().empty()) {
      throw std::runtime_error(label + " structured error did not include code and message");
    }
  }
}

} // namespace

int main(int argc, char **argv) {
  try {
    if (argc != 4) {
      std::cerr << "usage: " << argv[0]
                << " vectors/01-minimal.gts vectors/04-damaged-frame.gts vectors/28-empty-file.gts\n";
      return 2;
    }

    if (gts::abi_version() != GTS_ABI_VERSION) {
      throw std::runtime_error("unexpected ABI version");
    }
    if (gts::version().empty()) {
      throw std::runtime_error("empty library version");
    }

    const gts::Bytes input = read_file(argv[1]);
    const gts::Bytes damaged = read_file(argv[2]);
    const gts::Bytes empty = read_file(argv[3]);

    expect_contains("build metadata", gts::build_metadata_json(), "\"schema\":\"gts-capi-build-v1\"");
    expect_contains("capabilities", gts::capabilities_json(), "\"schema\":\"gts-capi-capabilities-v1\"");
    const std::string clean_read = gts::read_json(input);
    expect_contains("cpp clean-read read JSON", clean_read, "\"schema\":\"gts-capi-read-v1\"");
    expect_contains("cpp clean-read read JSON", clean_read, "\"clean\":true");
    expect_contains("verify JSON", gts::verify_json(input), "\"schema\":\"gts-capi-verify-v1\"");

    const std::string damaged_read = gts::read_json(damaged);
    expect_contains("cpp damaged-diagnostic-read read JSON", damaged_read, "\"schema\":\"gts-capi-read-v1\"");
    expect_contains("cpp damaged-diagnostic-read read JSON", damaged_read, "\"clean\":false");
    expect_contains("cpp damaged-diagnostic-read read JSON", damaged_read, "\"code\":\"DamagedFrame\"");
    expect_gts_error(
        "cpp damaged-diagnostic-read to_nquads",
        [&damaged]() {
          (void)gts::to_nquads(damaged);
        },
        GTS_STATUS_DIAGNOSTIC);

    const std::string empty_read = gts::read_json(empty);
    expect_contains("cpp empty-malformed-refusal read JSON", empty_read, "\"schema\":\"gts-capi-read-v1\"");
    expect_contains("cpp empty-malformed-refusal read JSON", empty_read, "\"clean\":false");
    expect_contains("cpp empty-malformed-refusal read JSON", empty_read, "\"code\":\"EmptyFile\"");
    expect_gts_error(
        "cpp empty-malformed-refusal to_nquads",
        [&empty]() {
          (void)gts::to_nquads(empty);
        },
        GTS_STATUS_DIAGNOSTIC);

    std::string nquads = gts::to_nquads(input);
    expect_contains("N-Quads", nquads, "\"Cat\"@en");

    gts::Bytes roundtrip = gts::from_nquads(nquads);
    if (roundtrip.empty()) {
      throw std::runtime_error("roundtrip GTS output was empty");
    }

    expect_gts_error(
        "cpp malformed-nquads-refusal from_nquads",
        []() {
          (void)gts::from_nquads("<https://example/s> <https://example/p> .\n");
        },
        GTS_STATUS_PARSE);

    TempDir temp;
    const auto source_dir = temp.path() / "src";
    const auto unpack_dir = temp.path() / "unpack";
    std::filesystem::create_directories(source_dir);
    write_text(source_dir / "a.txt", "hello\n");

    gts::Bytes packed = gts::files_pack({source_dir.string()});
    expect_contains("files diff", gts::files_diff_json(packed, source_dir.string()), "\"clean\":true");
    expect_contains("files unpack", gts::files_unpack(packed, unpack_dir.string()), "\"ok\":true");
    if (!std::filesystem::exists(unpack_dir / "a.txt")) {
      throw std::runtime_error("unpacked file missing");
    }
  } catch (const std::exception &error) {
    std::cerr << error.what() << '\n';
    return 1;
  }

  return 0;
}
