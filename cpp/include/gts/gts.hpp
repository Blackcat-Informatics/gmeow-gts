// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

#ifndef GMEOW_GTS_CPP_GTS_HPP
#define GMEOW_GTS_CPP_GTS_HPP

#include "gts.h"

#include <cstddef>
#include <cstdint>
#include <stdexcept>
#include <string>
#include <string_view>
#include <utility>
#include <vector>

namespace gts {

using Bytes = std::vector<std::uint8_t>;

inline const char *status_name(gts_status status) noexcept {
  switch (status) {
  case GTS_STATUS_OK:
    return "OK";
  case GTS_STATUS_INVALID_ARGUMENT:
    return "INVALID_ARGUMENT";
  case GTS_STATUS_IO:
    return "IO";
  case GTS_STATUS_PARSE:
    return "PARSE";
  case GTS_STATUS_DIAGNOSTIC:
    return "DIAGNOSTIC";
  case GTS_STATUS_INTERNAL:
    return "INTERNAL";
  case GTS_STATUS_PANIC:
    return "PANIC";
  default:
    return "UNKNOWN";
  }
}

class Error final : public std::runtime_error {
public:
  Error(std::string operation, gts_status status, std::string code, std::string message)
      : std::runtime_error(format(operation, status, code, message)),
        operation_(std::move(operation)),
        status_(status),
        code_(std::move(code)),
        message_(std::move(message)) {}

  const std::string &operation() const noexcept { return operation_; }
  gts_status status() const noexcept { return status_; }
  const std::string &code() const noexcept { return code_; }
  const std::string &message() const noexcept { return message_; }

private:
  static std::string format(const std::string &operation,
                            gts_status status,
                            const std::string &code,
                            const std::string &message) {
    std::string out = operation;
    out += " failed with ";
    out += status_name(status);
    if (!code.empty()) {
      out += " (";
      out += code;
      out += ")";
    }
    if (!message.empty()) {
      out += ": ";
      out += message;
    }
    return out;
  }

  std::string operation_;
  gts_status status_;
  std::string code_;
  std::string message_;
};

namespace detail {

class OwnedBuffer final {
public:
  OwnedBuffer() = default;
  OwnedBuffer(const OwnedBuffer &) = delete;
  OwnedBuffer &operator=(const OwnedBuffer &) = delete;

  OwnedBuffer(OwnedBuffer &&other) noexcept : buffer_(other.buffer_) {
    other.buffer_ = gts_buffer{nullptr, 0, 0};
  }

  OwnedBuffer &operator=(OwnedBuffer &&other) noexcept {
    if (this != &other) {
      gts_buffer_free(&buffer_);
      buffer_ = other.buffer_;
      other.buffer_ = gts_buffer{nullptr, 0, 0};
    }
    return *this;
  }

  ~OwnedBuffer() { gts_buffer_free(&buffer_); }

  gts_buffer *out() noexcept { return &buffer_; }
  const std::uint8_t *data() const noexcept { return buffer_.data; }
  std::size_t size() const noexcept { return buffer_.len; }

  std::string string() const {
    if (buffer_.len == 0) {
      return {};
    }
    return std::string(reinterpret_cast<const char *>(buffer_.data), buffer_.len);
  }

  Bytes bytes() const {
    if (buffer_.len == 0) {
      return {};
    }
    return Bytes(buffer_.data, buffer_.data + buffer_.len);
  }

private:
  gts_buffer buffer_{nullptr, 0, 0};
};

inline std::string copy_c_string(const char *value) {
  if (value == nullptr) {
    return {};
  }
  return std::string(value);
}

[[noreturn]] inline void raise_error(const char *operation, gts_status status, gts_error *error) {
  std::string code;
  std::string message;
  if (error != nullptr) {
    code = copy_c_string(gts_error_code(error));
    message = copy_c_string(gts_error_message(error));
    gts_error_free(error);
  }
  throw Error(operation, status, std::move(code), std::move(message));
}

template <typename Fn> OwnedBuffer call_buffer(const char *operation, Fn &&fn) {
  OwnedBuffer out;
  gts_error *error = nullptr;
  gts_status status = fn(out.out(), &error);
  if (status != GTS_STATUS_OK) {
    raise_error(operation, status, error);
  }
  if (error != nullptr) {
    gts_error_free(error);
    throw Error(operation, GTS_STATUS_INTERNAL, "unexpected-error-handle", "C ABI returned OK with an error handle");
  }
  return out;
}

inline const std::uint8_t *nullable_data(const Bytes &bytes) noexcept {
  return bytes.empty() ? nullptr : bytes.data();
}

} // namespace detail

inline std::uint32_t abi_version() noexcept { return gts_abi_version(); }

inline std::string version() { return detail::copy_c_string(gts_version()); }

inline std::string build_metadata_json() {
  return detail::call_buffer("gts_build_metadata_json",
                             [](gts_buffer *out, gts_error **error) {
                               return gts_build_metadata_json(out, error);
                             })
      .string();
}

inline std::string capabilities_json() {
  return detail::call_buffer("gts_capabilities_json",
                             [](gts_buffer *out, gts_error **error) {
                               return gts_capabilities_json(out, error);
                             })
      .string();
}

inline std::string read_json(const std::uint8_t *data, std::size_t len) {
  return detail::call_buffer("gts_read_json",
                             [data, len](gts_buffer *out, gts_error **error) {
                               return gts_read_json(data, len, out, error);
                             })
      .string();
}

inline std::string read_json(const Bytes &data) { return read_json(detail::nullable_data(data), data.size()); }

inline std::string verify_json(const std::uint8_t *data, std::size_t len) {
  return detail::call_buffer("gts_verify_json",
                             [data, len](gts_buffer *out, gts_error **error) {
                               return gts_verify_json(data, len, out, error);
                             })
      .string();
}

inline std::string verify_json(const Bytes &data) { return verify_json(detail::nullable_data(data), data.size()); }

inline std::string to_nquads(const std::uint8_t *data, std::size_t len) {
  return detail::call_buffer("gts_to_nquads",
                             [data, len](gts_buffer *out, gts_error **error) {
                               return gts_to_nquads(data, len, out, error);
                             })
      .string();
}

inline std::string to_nquads(const Bytes &data) { return to_nquads(detail::nullable_data(data), data.size()); }

inline Bytes from_nquads(std::string_view text) {
  return detail::call_buffer("gts_from_nquads",
                             [text](gts_buffer *out, gts_error **error) {
                               return gts_from_nquads(text.data(), text.size(), out, error);
                             })
      .bytes();
}

inline Bytes files_pack(const std::vector<std::string> &paths) {
  std::vector<const char *> raw_paths;
  raw_paths.reserve(paths.size());
  for (const auto &path : paths) {
    raw_paths.push_back(path.c_str());
  }
  return detail::call_buffer("gts_files_pack",
                             [&raw_paths](gts_buffer *out, gts_error **error) {
                               const char *const *data = raw_paths.empty() ? nullptr : raw_paths.data();
                               return gts_files_pack(data, raw_paths.size(), out, error);
                             })
      .bytes();
}

inline std::string files_unpack(const std::uint8_t *data,
                                std::size_t len,
                                std::string_view destination,
                                std::uint32_t flags = 0) {
  std::string destination_string(destination);
  return detail::call_buffer("gts_files_unpack",
                             [data, len, &destination_string, flags](gts_buffer *out, gts_error **error) {
                               return gts_files_unpack(data, len, destination_string.c_str(), flags, out, error);
                             })
      .string();
}

inline std::string files_unpack(const Bytes &data, std::string_view destination, std::uint32_t flags = 0) {
  return files_unpack(detail::nullable_data(data), data.size(), destination, flags);
}

inline std::string files_diff_json(const std::uint8_t *data, std::size_t len, std::string_view directory) {
  std::string directory_string(directory);
  return detail::call_buffer("gts_files_diff_json",
                             [data, len, &directory_string](gts_buffer *out, gts_error **error) {
                               return gts_files_diff_json(data, len, directory_string.c_str(), out, error);
                             })
      .string();
}

inline std::string files_diff_json(const Bytes &data, std::string_view directory) {
  return files_diff_json(detail::nullable_data(data), data.size(), directory);
}

} // namespace gts

#endif // GMEOW_GTS_CPP_GTS_HPP
