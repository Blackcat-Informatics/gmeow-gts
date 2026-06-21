# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0

require "ffi"
require "rbconfig"
require "thread"

module Gmeow
  module Gts
    VERSION = "0.9.4"
    ABI_VERSION = 1
    UINT32_MAX = 0xffff_ffff

    module Status
      OK = 0
      INVALID_ARGUMENT = 1
      IO = 2
      PARSE = 3
      DIAGNOSTIC = 4
      INTERNAL = 5
      PANIC = 6
    end

    module UnpackFlags
      NONE = 0
      INCLUDE_SUPPRESSED = 1 << 0
      ALLOW_SYMLINKS = 1 << 1
      ALLOW_SPECIAL = 1 << 2
      SAME_OWNER = 1 << 3
      PRESERVE_SETID = 1 << 4
    end

    STATUS_NAMES = {
      Status::OK => "OK",
      Status::INVALID_ARGUMENT => "INVALID_ARGUMENT",
      Status::IO => "IO",
      Status::PARSE => "PARSE",
      Status::DIAGNOSTIC => "DIAGNOSTIC",
      Status::INTERNAL => "INTERNAL",
      Status::PANIC => "PANIC"
    }.freeze

    class Error < StandardError
      attr_reader :operation, :status, :status_name, :code, :detail

      def initialize(operation:, status:, code:, detail:)
        @operation = operation
        @status = status
        @status_name = Gts.status_name(status)
        @code = code
        @detail = detail
        super(build_message)
      end

      private

      def build_message
        message = "#{operation} failed with #{status_name}"
        message = "#{message} (#{code})" unless code.empty?
        message = "#{message}: #{detail}" unless detail.empty?
        message
      end
    end

    class Buffer < FFI::Struct
      layout :data, :pointer,
             :len, :size_t,
             :capacity, :size_t
    end

    module Native
      @modules = {}
      @mutex = Mutex.new

      def self.bind(library)
        key = library.to_s
        @mutex.synchronize do
          @modules[key] ||= build(key)
        end
      end

      def self.build(library)
        Module.new do
          extend FFI::Library

          ffi_lib library

          attach_function :gts_abi_version, [], :uint32
          attach_function :gts_version, [], :pointer

          attach_function :gts_buffer_free, [:pointer], :void
          attach_function :gts_error_free, [:pointer], :void
          attach_function :gts_error_code, [:pointer], :pointer
          attach_function :gts_error_message, [:pointer], :pointer

          attach_function :gts_build_metadata_json, [:pointer, :pointer], :int
          attach_function :gts_capabilities_json, [:pointer, :pointer], :int
          attach_function :gts_read_json, [:pointer, :size_t, :pointer, :pointer], :int
          attach_function :gts_verify_json, [:pointer, :size_t, :pointer, :pointer], :int
          attach_function :gts_to_nquads, [:pointer, :size_t, :pointer, :pointer], :int
          attach_function :gts_from_nquads, [:pointer, :size_t, :pointer, :pointer], :int
          attach_function :gts_files_pack, [:pointer, :size_t, :pointer, :pointer], :int
          attach_function :gts_files_unpack,
                          [:pointer, :size_t, :pointer, :uint32, :pointer, :pointer],
                          :int
          attach_function :gts_files_diff_json,
                          [:pointer, :size_t, :pointer, :pointer, :pointer],
                          :int
        end
      end

      private_class_method :build
    end

    class Library
      def initialize(library = nil)
        @native = Native.bind(library || Gts.default_library)
      end

      def abi_version
        @native.gts_abi_version
      end

      def version
        copy_c_string(@native.gts_version)
      end

      def build_metadata_json
        call_buffer("gts_build_metadata_json") do |out, error|
          @native.gts_build_metadata_json(out, error)
        end
      end

      def capabilities_json
        call_buffer("gts_capabilities_json") do |out, error|
          @native.gts_capabilities_json(out, error)
        end
      end

      def read_json(data)
        with_bytes(data, "data") do |pointer, length|
          call_buffer("gts_read_json") do |out, error|
            @native.gts_read_json(pointer, length, out, error)
          end
        end
      end

      def verify_json(data)
        with_bytes(data, "data") do |pointer, length|
          call_buffer("gts_verify_json") do |out, error|
            @native.gts_verify_json(pointer, length, out, error)
          end
        end
      end

      def to_nquads(data)
        with_bytes(data, "data") do |pointer, length|
          call_buffer("gts_to_nquads") do |out, error|
            @native.gts_to_nquads(pointer, length, out, error)
          end
        end
      end

      def from_nquads(text)
        with_bytes(text, "text") do |pointer, length|
          call_buffer("gts_from_nquads") do |out, error|
            @native.gts_from_nquads(pointer, length, out, error)
          end
        end
      end

      def files_pack(paths)
        path_pointers, keepalive = native_path_list(paths)
        call_buffer("gts_files_pack") do |out, error|
          @native.gts_files_pack(path_pointers, keepalive.length, out, error)
        end
      end

      def files_unpack(data, destination, flags = UnpackFlags::NONE)
        destination_pointer = native_string(destination, "destination")
        unpack_flags = checked_uint32(flags, "flags")

        with_bytes(data, "data") do |pointer, length|
          call_buffer("gts_files_unpack") do |out, error|
            @native.gts_files_unpack(pointer, length, destination_pointer, unpack_flags, out, error)
          end
        end
      end

      def files_diff_json(data, directory)
        directory_pointer = native_string(directory, "directory")

        with_bytes(data, "data") do |pointer, length|
          call_buffer("gts_files_diff_json") do |out, error|
            @native.gts_files_diff_json(pointer, length, directory_pointer, out, error)
          end
        end
      end

      private

      def call_buffer(operation)
        out = new_buffer
        error = FFI::MemoryPointer.new(:pointer)
        error.write_pointer(FFI::Pointer::NULL)

        begin
          status = yield(out.to_ptr, error)
          err_ptr = error.read_pointer
          if status != Status::OK
            raise build_error(operation, status, err_ptr)
          end
          unless err_ptr.null?
            raise build_error(operation, Status::INTERNAL, err_ptr)
          end
          copy_buffer(out)
        ensure
          @native.gts_buffer_free(out.to_ptr)
        end
      end

      def new_buffer
        buffer = Buffer.new
        buffer[:data] = FFI::Pointer::NULL
        buffer[:len] = 0
        buffer[:capacity] = 0
        buffer
      end

      def build_error(operation, status, error)
        code = ""
        detail = ""

        unless error.null?
          begin
            code = copy_c_string(@native.gts_error_code(error))
            detail = copy_c_string(@native.gts_error_message(error))
          ensure
            @native.gts_error_free(error)
          end
        end

        Error.new(operation: operation, status: status, code: code, detail: detail)
      end

      def copy_buffer(buffer)
        length = buffer[:len]
        return "" if length.zero?

        data = buffer[:data]
        raise RuntimeError, "C ABI returned a null data pointer with non-zero length." if data.null?

        data.read_string_length(length)
      end

      def copy_c_string(pointer)
        return "" if pointer.nil? || pointer.null?

        pointer.read_string
      end

      def with_bytes(value, name)
        raise TypeError, "#{name} must be a String" unless value.is_a?(String)

        length = value.bytesize
        pointer = FFI::MemoryPointer.new(:uint8, [length, 1].max)
        pointer.put_bytes(0, value) unless length.zero?
        yield(pointer, length)
      end

      def native_string(value, name)
        raise TypeError, "#{name} must be a String" unless value.is_a?(String)
        raise ArgumentError, "#{name} must not contain NUL bytes" if value.include?("\0")

        FFI::MemoryPointer.from_string(value)
      end

      def native_path_list(paths)
        raise TypeError, "paths must be an Array" unless paths.is_a?(Array)
        raise ArgumentError, "paths must not be empty" if paths.empty?

        pointer_array = FFI::MemoryPointer.new(:pointer, paths.length)
        keepalive = paths.each_with_index.map do |path, index|
          pointer = native_string(path, "paths[#{index}]")
          pointer_array.put_pointer(index * FFI.type_size(:pointer), pointer)
          pointer
        end

        [pointer_array, keepalive]
      end

      def checked_uint32(value, name)
        raise TypeError, "#{name} must be an Integer" unless value.is_a?(Integer)
        raise RangeError, "#{name} must be an unsigned 32-bit integer" if value.negative? || value > UINT32_MAX

        value
      end
    end

    def self.default_library
      from_env = ENV.fetch("GTS_LIBGTS", nil)
      return from_env unless from_env.nil? || from_env.empty?

      host_os = RbConfig::CONFIG.fetch("host_os", "")
      case host_os
      when /darwin/i
        "libgts.dylib"
      when /mswin|mingw|cygwin/i
        "gts.dll"
      else
        "libgts.so"
      end
    end

    def self.load(library = nil)
      Library.new(library)
    end

    def self.status_name(status)
      STATUS_NAMES.fetch(status, "UNKNOWN")
    end
  end
end
