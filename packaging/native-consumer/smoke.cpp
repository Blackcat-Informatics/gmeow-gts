// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

#include <gts/gts.hpp>

#include <cstdint>
#include <fstream>
#include <iostream>
#include <iterator>
#include <string>
#include <vector>

int main(int argc, char **argv) {
  if (argc != 2) {
    std::cerr << "usage: gts-native-consumer vectors/01-minimal.gts\n";
    return 2;
  }

  if (gts::abi_version() != GTS_ABI_VERSION) {
    std::cerr << "runtime ABI version does not match compile-time header\n";
    return 1;
  }
  if (gts::version().empty()) {
    std::cerr << "runtime version was empty\n";
    return 1;
  }

  std::ifstream input(argv[1], std::ios::binary);
  if (!input) {
    std::cerr << "could not open fixture: " << argv[1] << "\n";
    return 1;
  }
  std::vector<std::uint8_t> bytes(std::istreambuf_iterator<char>(input), {});

  const std::string read_json = gts::read_json(bytes);
  const std::string verify_json = gts::verify_json(bytes);
  const std::string nquads = gts::to_nquads(bytes);
  const gts::Bytes roundtrip = gts::from_nquads(nquads);

  if (read_json.find("\"segments\"") == std::string::npos) {
    std::cerr << "read JSON did not contain segment metadata\n";
    return 1;
  }
  if (verify_json.find("\"ok\"") == std::string::npos) {
    std::cerr << "verify JSON did not contain status metadata\n";
    return 1;
  }
  if (nquads.empty() || roundtrip.empty()) {
    std::cerr << "N-Quads conversion roundtrip returned empty output\n";
    return 1;
  }

  std::cout << gts::version() << "\n";
  return 0;
}
