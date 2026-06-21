# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0

module GmeowGTS

using Libdl

export ABI_VERSION,
    GtsError,
    Status,
    UnpackFlags,
    abi_version,
    build_metadata_json,
    capabilities_json,
    files_diff_json,
    files_pack,
    files_unpack,
    from_nquads,
    libgts_version,
    read_json,
    status_name,
    to_nquads,
    verify_json

const ABI_VERSION = UInt32(1)
const VERSION = v"0.9.4"

module Status
const OK = Int32(0)
const INVALID_ARGUMENT = Int32(1)
const IO = Int32(2)
const PARSE = Int32(3)
const DIAGNOSTIC = Int32(4)
const INTERNAL = Int32(5)
const PANIC = Int32(6)
end

module UnpackFlags
const NONE = UInt32(0)
const INCLUDE_SUPPRESSED = UInt32(1) << 0
const ALLOW_SYMLINKS = UInt32(1) << 1
const ALLOW_SPECIAL = UInt32(1) << 2
const SAME_OWNER = UInt32(1) << 3
const PRESERVE_SETID = UInt32(1) << 4
end

const STATUS_NAMES = Dict(
    Status.OK => "OK",
    Status.INVALID_ARGUMENT => "INVALID_ARGUMENT",
    Status.IO => "IO",
    Status.PARSE => "PARSE",
    Status.DIAGNOSTIC => "DIAGNOSTIC",
    Status.INTERNAL => "INTERNAL",
    Status.PANIC => "PANIC",
)

const LIBRARY_HANDLES = Dict{String,Ptr{Cvoid}}()
const LIBRARY_HANDLE_LOCK = ReentrantLock()

function library_handle(library::AbstractString)
    key = String(library)
    lock(LIBRARY_HANDLE_LOCK)
    try
        get!(LIBRARY_HANDLES, key) do
            Libdl.dlopen(key)
        end
    finally
        unlock(LIBRARY_HANDLE_LOCK)
    end
end

native_symbol(library::AbstractString, name::Symbol) = Libdl.dlsym(library_handle(library), name)

struct GtsBuffer
    data::Ptr{UInt8}
    len::Csize_t
    capacity::Csize_t
end

struct GtsError <: Exception
    operation::String
    status::Int32
    status_name::String
    code::String
    detail::String
end

function Base.showerror(io::IO, error::GtsError)
    message = "$(error.operation) failed with $(error.status_name)"
    if !isempty(error.code)
        message = "$message ($(error.code))"
    end
    if !isempty(error.detail)
        message = "$message: $(error.detail)"
    end
    print(io, message)
end

status_name(code::Integer) = get(STATUS_NAMES, Int32(code), "UNKNOWN")

function default_library()
    explicit = get(ENV, "GTS_LIBGTS", "")
    if !isempty(explicit)
        return explicit
    end

    directory = get(ENV, "GTS_LIB_DIR", "")
    library = if Sys.iswindows()
        "gts.dll"
    elseif Sys.isapple()
        "libgts.dylib"
    else
        "libgts.so"
    end

    isempty(directory) ? library : joinpath(directory, library)
end

function abi_version(; library::AbstractString = default_library())
    ccall(native_symbol(library, :gts_abi_version), UInt32, ())
end

function libgts_version(; library::AbstractString = default_library())
    version_pointer = ccall(native_symbol(library, :gts_version), Cstring, ())
    version_pointer == C_NULL ? "" : unsafe_string(version_pointer)
end

function build_metadata_json(; library::AbstractString = default_library())
    text_call("gts_build_metadata_json", library) do out, error
        ccall(native_symbol(library, :gts_build_metadata_json), Cint, (Ref{GtsBuffer}, Ref{Ptr{Cvoid}}), out, error)
    end
end

function capabilities_json(; library::AbstractString = default_library())
    text_call("gts_capabilities_json", library) do out, error
        ccall(native_symbol(library, :gts_capabilities_json), Cint, (Ref{GtsBuffer}, Ref{Ptr{Cvoid}}), out, error)
    end
end

function read_json(data; library::AbstractString = default_library())
    with_bytes(data, "data") do data_pointer, len
        text_call("gts_read_json", library) do out, error
            ccall(
                native_symbol(library, :gts_read_json),
                Cint,
                (Ptr{UInt8}, Csize_t, Ref{GtsBuffer}, Ref{Ptr{Cvoid}}),
                data_pointer,
                len,
                out,
                error,
            )
        end
    end
end

function verify_json(data; library::AbstractString = default_library())
    with_bytes(data, "data") do data_pointer, len
        text_call("gts_verify_json", library) do out, error
            ccall(
                native_symbol(library, :gts_verify_json),
                Cint,
                (Ptr{UInt8}, Csize_t, Ref{GtsBuffer}, Ref{Ptr{Cvoid}}),
                data_pointer,
                len,
                out,
                error,
            )
        end
    end
end

function to_nquads(data; library::AbstractString = default_library())
    with_bytes(data, "data") do data_pointer, len
        text_call("gts_to_nquads", library) do out, error
            ccall(
                native_symbol(library, :gts_to_nquads),
                Cint,
                (Ptr{UInt8}, Csize_t, Ref{GtsBuffer}, Ref{Ptr{Cvoid}}),
                data_pointer,
                len,
                out,
                error,
            )
        end
    end
end

function from_nquads(text::AbstractString; library::AbstractString = default_library())
    text_string = checked_string(text, "text")
    ensure_no_nul(text_string, "text")

    GC.@preserve text_string begin
        text_pointer = if isempty(text_string)
            Ptr{UInt8}(C_NULL)
        else
            Base.unsafe_convert(Ptr{UInt8}, Base.pointer(text_string))
        end
        raw_call("gts_from_nquads", library) do out, error
            ccall(
                native_symbol(library, :gts_from_nquads),
                Cint,
                (Ptr{UInt8}, Csize_t, Ref{GtsBuffer}, Ref{Ptr{Cvoid}}),
                text_pointer,
                Csize_t(sizeof(text_string)),
                out,
                error,
            )
        end
    end
end

function files_pack(paths; library::AbstractString = default_library())
    strings = checked_strings(paths, "paths")
    cstrings = Vector{Cstring}(undef, length(strings))
    for (index, path) in pairs(strings)
        cstrings[index] = checked_cstring(path, "paths[$index]")
    end

    GC.@preserve strings cstrings begin
        raw_call("gts_files_pack", library) do out, error
            ccall(
                native_symbol(library, :gts_files_pack),
                Cint,
                (Ptr{Cstring}, Csize_t, Ref{GtsBuffer}, Ref{Ptr{Cvoid}}),
                Base.pointer(cstrings),
                Csize_t(length(cstrings)),
                out,
                error,
            )
        end
    end
end

function files_unpack(data, destination::AbstractString; flags = UnpackFlags.NONE, library::AbstractString = default_library())
    destination_string = checked_string(destination, "destination")
    ensure_no_nul(destination_string, "destination")
    unpack_flags = checked_uint32(flags, "flags")

    GC.@preserve destination_string begin
        destination_pointer = checked_cstring(destination_string, "destination")
        with_bytes(data, "data") do data_pointer, len
            text_call("gts_files_unpack", library) do out, error
                ccall(
                    native_symbol(library, :gts_files_unpack),
                    Cint,
                    (Ptr{UInt8}, Csize_t, Cstring, UInt32, Ref{GtsBuffer}, Ref{Ptr{Cvoid}}),
                    data_pointer,
                    len,
                    destination_pointer,
                    unpack_flags,
                    out,
                    error,
                )
            end
        end
    end
end

function files_diff_json(data, directory::AbstractString; library::AbstractString = default_library())
    directory_string = checked_string(directory, "directory")
    ensure_no_nul(directory_string, "directory")

    GC.@preserve directory_string begin
        directory_pointer = checked_cstring(directory_string, "directory")
        with_bytes(data, "data") do data_pointer, len
            text_call("gts_files_diff_json", library) do out, error
                ccall(
                    native_symbol(library, :gts_files_diff_json),
                    Cint,
                    (Ptr{UInt8}, Csize_t, Cstring, Ref{GtsBuffer}, Ref{Ptr{Cvoid}}),
                    data_pointer,
                    len,
                    directory_pointer,
                    out,
                    error,
                )
            end
        end
    end
end

function text_call(f, operation::AbstractString, library::AbstractString)
    String(raw_buffer_call(operation, library, f))
end

function raw_call(f, operation::AbstractString, library::AbstractString)
    raw_buffer_call(operation, library, f)
end

function raw_buffer_call(operation::AbstractString, library::AbstractString, f)
    buffer_free = native_symbol(library, :gts_buffer_free)
    out = Ref(GtsBuffer(Ptr{UInt8}(C_NULL), Csize_t(0), Csize_t(0)))
    error = Ref{Ptr{Cvoid}}(C_NULL)

    try
        status = Int32(f(out, error))
        error_pointer = error[]
        if status != Status.OK
            throw(build_error(operation, status, error_pointer, library))
        end
        if error_pointer != C_NULL
            throw(build_error(operation, Status.INTERNAL, error_pointer, library))
        end
        copy_buffer(out[])
    finally
        ccall(buffer_free, Cvoid, (Ref{GtsBuffer},), out)
    end
end

function build_error(operation::AbstractString, status::Integer, error_pointer::Ptr{Cvoid}, library::AbstractString)
    code = ""
    detail = ""

    if error_pointer != C_NULL
        error_code = native_symbol(library, :gts_error_code)
        error_message = native_symbol(library, :gts_error_message)
        error_free = native_symbol(library, :gts_error_free)

        try
            code_pointer = ccall(error_code, Cstring, (Ptr{Cvoid},), error_pointer)
            detail_pointer = ccall(error_message, Cstring, (Ptr{Cvoid},), error_pointer)
            code = code_pointer == C_NULL ? "" : unsafe_string(code_pointer)
            detail = detail_pointer == C_NULL ? "" : unsafe_string(detail_pointer)
        finally
            ccall(error_free, Cvoid, (Ptr{Cvoid},), error_pointer)
        end
    end

    GtsError(String(operation), Int32(status), status_name(status), code, detail)
end

function copy_buffer(buffer::GtsBuffer)
    len = checked_length(buffer.len)
    if len == 0
        return UInt8[]
    end
    if buffer.data == C_NULL
        error("C ABI returned a null data pointer with non-zero length")
    end
    copy(unsafe_wrap(Vector{UInt8}, buffer.data, len; own = false))
end

function with_bytes(f, data, name::AbstractString)
    bytes = checked_bytes(data, name)
    GC.@preserve bytes begin
        data_pointer = isempty(bytes) ? Ptr{UInt8}(C_NULL) : Base.unsafe_convert(Ptr{UInt8}, Base.pointer(bytes))
        f(data_pointer, Csize_t(length(bytes)))
    end
end

function checked_bytes(data, name::AbstractString)
    if data isa Vector{UInt8}
        return data
    end
    if data isa AbstractVector{UInt8}
        return Vector{UInt8}(data)
    end
    throw(ArgumentError("$name must be a Vector{UInt8} or AbstractVector{UInt8}"))
end

function checked_strings(values, name::AbstractString)
    if !(values isa AbstractVector)
        throw(ArgumentError("$name must be a non-empty vector of strings"))
    end
    if isempty(values)
        throw(ArgumentError("$name must be a non-empty vector of strings"))
    end
    [checked_string(value, "$name[$index]") for (index, value) in pairs(values)]
end

function checked_string(value, name::AbstractString)
    if !(value isa AbstractString)
        throw(ArgumentError("$name must be a string"))
    end
    String(value)
end

function checked_cstring(value::String, name::AbstractString)
    ensure_no_nul(value, name)
    Base.unsafe_convert(Cstring, value)
end

function ensure_no_nul(value::AbstractString, name::AbstractString)
    if occursin('\0', value)
        throw(ArgumentError("$name must not contain NUL bytes"))
    end
end

function checked_uint32(value, name::AbstractString)
    if !(value isa Integer)
        throw(ArgumentError("$name must be an integer"))
    end
    if value < 0 || value > typemax(UInt32)
        throw(ArgumentError("$name must be a non-negative 32-bit integer"))
    end
    UInt32(value)
end

function checked_length(value::Csize_t)
    if value > typemax(Int)
        error("C ABI returned a buffer too large for this Julia runtime")
    end
    Int(value)
end

end
