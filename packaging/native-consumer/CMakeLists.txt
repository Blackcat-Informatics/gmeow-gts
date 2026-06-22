# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0

cmake_minimum_required(VERSION 3.16)
project(gts_native_consumer LANGUAGES CXX)

find_package(Gts REQUIRED)

add_executable(gts-native-consumer smoke.cpp)
target_compile_features(gts-native-consumer PRIVATE cxx_std_17)
target_link_libraries(gts-native-consumer PRIVATE Gts::gts)
