# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0

if(NOT DEFINED ENV{GMEOW_GTS_SOURCE_PATH})
  message(FATAL_ERROR "Set GMEOW_GTS_SOURCE_PATH to a gmeow-gts source checkout for the local overlay port.")
endif()

file(TO_CMAKE_PATH "$ENV{GMEOW_GTS_SOURCE_PATH}" SOURCE_PATH)
if(NOT EXISTS "${SOURCE_PATH}/rust/capi/Cargo.toml")
  message(FATAL_ERROR "GMEOW_GTS_SOURCE_PATH does not look like a gmeow-gts checkout: ${SOURCE_PATH}")
endif()

if(VCPKG_LIBRARY_LINKAGE STREQUAL "static")
  message(FATAL_ERROR "The local gmeow-gts vcpkg overlay currently validates the dynamic libgts package layout. Use a dynamic triplet such as x64-linux-dynamic.")
endif()

file(READ "${CMAKE_CURRENT_LIST_DIR}/vcpkg.json" _manifest_json)
string(JSON GMEOW_GTS_VERSION GET "${_manifest_json}" version)
file(READ "${SOURCE_PATH}/rust/capi/include/gts.h" _gts_header)
string(REGEX MATCH "#define GTS_ABI_VERSION ([0-9]+)" _abi_match "${_gts_header}")
if(NOT CMAKE_MATCH_1)
  message(FATAL_ERROR "Could not determine GTS_ABI_VERSION from ${SOURCE_PATH}/rust/capi/include/gts.h")
endif()
set(GMEOW_GTS_ABI_VERSION "${CMAKE_MATCH_1}")

find_program(CARGO_COMMAND NAMES cargo REQUIRED)
vcpkg_execute_required_process(
  COMMAND "${CARGO_COMMAND}" build --manifest-path "${SOURCE_PATH}/rust/capi/Cargo.toml" --release --locked
  WORKING_DIRECTORY "${SOURCE_PATH}"
  LOGNAME "build-release-${PORT}-${TARGET_TRIPLET}"
)
vcpkg_execute_required_process(
  COMMAND "${CARGO_COMMAND}" build --manifest-path "${SOURCE_PATH}/rust/capi/Cargo.toml" --locked
  WORKING_DIRECTORY "${SOURCE_PATH}"
  LOGNAME "build-debug-${PORT}-${TARGET_TRIPLET}"
)

set(_release_target_dir "${SOURCE_PATH}/rust/capi/target/release")
set(_debug_target_dir "${SOURCE_PATH}/rust/capi/target/debug")

file(MAKE_DIRECTORY
  "${CURRENT_PACKAGES_DIR}/include/gts"
  "${CURRENT_PACKAGES_DIR}/lib/cmake/Gts"
  "${CURRENT_PACKAGES_DIR}/lib/pkgconfig"
  "${CURRENT_PACKAGES_DIR}/debug/lib/cmake/Gts"
  "${CURRENT_PACKAGES_DIR}/debug/lib"
  "${CURRENT_PACKAGES_DIR}/share/${PORT}"
  "${CURRENT_PACKAGES_DIR}/share/gts"
)

file(INSTALL "${SOURCE_PATH}/rust/capi/include/gts.h" DESTINATION "${CURRENT_PACKAGES_DIR}/include")
file(INSTALL "${SOURCE_PATH}/cpp/include/gts/gts.hpp" DESTINATION "${CURRENT_PACKAGES_DIR}/include/gts")
file(INSTALL "${SOURCE_PATH}/rust/capi/cmake/GtsConfig.cmake" DESTINATION "${CURRENT_PACKAGES_DIR}/lib/cmake/Gts")
file(INSTALL "${SOURCE_PATH}/rust/capi/cmake/GtsConfig.cmake" DESTINATION "${CURRENT_PACKAGES_DIR}/debug/lib/cmake/Gts")
file(INSTALL "${SOURCE_PATH}/rust/capi/README.md" DESTINATION "${CURRENT_PACKAGES_DIR}/share/${PORT}" RENAME README.md)

foreach(_lib IN ITEMS libgts.so libgts.dylib)
  if(EXISTS "${_release_target_dir}/${_lib}")
    file(INSTALL "${_release_target_dir}/${_lib}" DESTINATION "${CURRENT_PACKAGES_DIR}/lib")
  endif()
  if(EXISTS "${_debug_target_dir}/${_lib}")
    file(INSTALL "${_debug_target_dir}/${_lib}" DESTINATION "${CURRENT_PACKAGES_DIR}/debug/lib")
  endif()
endforeach()
foreach(_bin IN ITEMS gts.dll gts.pdb)
  if(EXISTS "${_release_target_dir}/${_bin}")
    file(INSTALL "${_release_target_dir}/${_bin}" DESTINATION "${CURRENT_PACKAGES_DIR}/bin")
  endif()
  if(EXISTS "${_debug_target_dir}/${_bin}")
    file(INSTALL "${_debug_target_dir}/${_bin}" DESTINATION "${CURRENT_PACKAGES_DIR}/debug/bin")
  endif()
endforeach()
foreach(_import_lib IN ITEMS gts.dll.lib gts.lib)
  if(EXISTS "${_release_target_dir}/${_import_lib}")
    file(INSTALL "${_release_target_dir}/${_import_lib}" DESTINATION "${CURRENT_PACKAGES_DIR}/lib")
  endif()
  if(EXISTS "${_debug_target_dir}/${_import_lib}")
    file(INSTALL "${_debug_target_dir}/${_import_lib}" DESTINATION "${CURRENT_PACKAGES_DIR}/debug/lib")
  endif()
endforeach()

if(NOT EXISTS "${CURRENT_PACKAGES_DIR}/lib/libgts.so"
   AND NOT EXISTS "${CURRENT_PACKAGES_DIR}/lib/libgts.dylib"
   AND NOT EXISTS "${CURRENT_PACKAGES_DIR}/bin/gts.dll")
  message(FATAL_ERROR "No dynamic libgts artifact was produced in ${_release_target_dir}")
endif()

file(WRITE "${CURRENT_PACKAGES_DIR}/lib/pkgconfig/gts.pc"
"prefix=\${pcfiledir}/../..
exec_prefix=\${prefix}
libdir=\${exec_prefix}/lib
includedir=\${prefix}/include

Name: gts
Description: Graph Transport Substrate Rust C ABI
Version: ${GMEOW_GTS_VERSION}
Libs: -L\${libdir} -lgts
Cflags: -I\${includedir}
")
file(WRITE "${CURRENT_PACKAGES_DIR}/share/gts/VERSION" "${GMEOW_GTS_VERSION}\n")
file(WRITE "${CURRENT_PACKAGES_DIR}/share/gts/ABI_VERSION" "${GMEOW_GTS_ABI_VERSION}\n")
file(WRITE "${CURRENT_PACKAGES_DIR}/share/gts/archive.json"
"{
  \"schema\": \"gts-capi-archive-v1\",
  \"package\": \"gmeow-gts\",
  \"version\": \"${GMEOW_GTS_VERSION}\",
  \"abi_version\": ${GMEOW_GTS_ABI_VERSION},
  \"manager\": \"vcpkg\"
}
")
vcpkg_cmake_config_fixup(PACKAGE_NAME Gts CONFIG_PATH lib/cmake/Gts)
vcpkg_fixup_pkgconfig()

file(READ "${SOURCE_PATH}/LICENSE-MIT" _license_mit)
file(READ "${SOURCE_PATH}/LICENSE-APACHE" _license_apache)
file(WRITE "${CURRENT_PACKAGES_DIR}/share/${PORT}/copyright"
"gmeow-gts is dual licensed under MIT OR Apache-2.0.

MIT license:

${_license_mit}

Apache-2.0 license:

${_license_apache}
")
