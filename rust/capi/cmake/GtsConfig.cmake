# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0

include_guard(GLOBAL)

get_filename_component(_GTS_PREFIX "${CMAKE_CURRENT_LIST_DIR}/.." ABSOLUTE)
find_path(GTS_INCLUDE_DIR NAMES gts.h PATHS "${_GTS_PREFIX}/include" NO_DEFAULT_PATH)
find_library(GTS_LIBRARY NAMES gts PATHS "${_GTS_PREFIX}/lib" NO_DEFAULT_PATH)

if(NOT GTS_INCLUDE_DIR OR NOT GTS_LIBRARY)
  set(Gts_FOUND FALSE)
  return()
endif()

add_library(Gts::gts UNKNOWN IMPORTED)
set_target_properties(Gts::gts PROPERTIES
  IMPORTED_LOCATION "${GTS_LIBRARY}"
  INTERFACE_INCLUDE_DIRECTORIES "${GTS_INCLUDE_DIR}")

set(Gts_FOUND TRUE)
