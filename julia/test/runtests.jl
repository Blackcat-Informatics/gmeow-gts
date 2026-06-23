# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0

using GmeowGTS
using Test

vector_path = if length(ARGS) >= 1
    ARGS[1]
else
    get(ENV, "GTS_WRAPPER_CLEAN_VECTOR", get(ENV, "GTS_JULIA_VECTOR", ""))
end
if isempty(vector_path)
    @info "Skipping external GTS smoke test; pass shared wrapper fixture paths to run it."
    exit(0)
end
damaged_path = length(ARGS) >= 2 ? ARGS[2] : get(ENV, "GTS_WRAPPER_DAMAGED_VECTOR", "")
empty_path = length(ARGS) >= 3 ? ARGS[3] : get(ENV, "GTS_WRAPPER_EMPTY_VECTOR", "")
if isempty(damaged_path) || isempty(empty_path)
    error("missing damaged or empty wrapper smoke fixture path")
end

compact_json(json::AbstractString) = replace(json, r"\s+" => "")

function expect_json_string(label::AbstractString, json::AbstractString, key::AbstractString, expected::AbstractString)
    needle = "\"$key\":\"$expected\""
    @test occursin(needle, compact_json(json)) || error("$label did not contain $needle")
end

function expect_json_bool(label::AbstractString, json::AbstractString, key::AbstractString, expected::Bool)
    literal = expected ? "true" : "false"
    needle = "\"$key\":$literal"
    @test occursin(needle, compact_json(json)) || error("$label did not contain $needle")
end

function expect_diagnostic(label::AbstractString, json::AbstractString, code::AbstractString)
    needle = "\"code\":\"$code\""
    @test occursin(needle, compact_json(json)) || error("$label did not contain $needle")
end

function expect_gts_error(fn, label::AbstractString, expected_status::Int32)
    observed = try
        fn()
        nothing
    catch error
        error
    end
    @test observed isa GtsError || error("$label did not raise GtsError")
    @test observed.status == expected_status || error("$label returned $(observed.status_name)")
    @test !isempty(observed.code) || error("$label structured error code was empty")
    @test !isempty(observed.detail) || error("$label structured error detail was empty")
end

input = read(vector_path)
damaged = read(damaged_path)
empty = read(empty_path)

version_tasks = [Threads.@spawn abi_version() for _ in 1:8]
@test all(fetch.(version_tasks) .== ABI_VERSION)

@test abi_version() == ABI_VERSION
@test !isempty(libgts_version())

expect_json_string("build metadata", build_metadata_json(), "schema", "gts-capi-build-v1")
expect_json_string("capabilities", capabilities_json(), "schema", "gts-capi-capabilities-v1")
clean_read = read_json(input)
expect_json_string("julia clean-read read JSON", clean_read, "schema", "gts-capi-read-v1")
expect_json_bool("julia clean-read read JSON", clean_read, "clean", true)
expect_json_string("verify JSON", verify_json(input), "schema", "gts-capi-verify-v1")

damaged_read = read_json(damaged)
expect_json_string("julia damaged-diagnostic-read read JSON", damaged_read, "schema", "gts-capi-read-v1")
expect_json_bool("julia damaged-diagnostic-read read JSON", damaged_read, "clean", false)
expect_diagnostic("julia damaged-diagnostic-read read JSON", damaged_read, "DamagedFrame")
expect_gts_error("julia damaged-diagnostic-read to_nquads", Status.DIAGNOSTIC) do
    to_nquads(damaged)
end

empty_read = read_json(empty)
expect_json_string("julia empty-malformed-refusal read JSON", empty_read, "schema", "gts-capi-read-v1")
expect_json_bool("julia empty-malformed-refusal read JSON", empty_read, "clean", false)
expect_diagnostic("julia empty-malformed-refusal read JSON", empty_read, "EmptyFile")
expect_gts_error("julia empty-malformed-refusal to_nquads", Status.DIAGNOSTIC) do
    to_nquads(empty)
end

nquads = to_nquads(input)
@test occursin("\"Cat\"@en", nquads)

round_trip = from_nquads(nquads)
@test round_trip isa Vector{UInt8}
@test !isempty(round_trip)

wrapped_nquads = "x$nquads"
round_trip_substring = from_nquads(SubString(wrapped_nquads, 2, lastindex(wrapped_nquads)))
@test round_trip_substring isa Vector{UInt8}
@test !isempty(round_trip_substring)

expect_gts_error("julia malformed-nquads-refusal from_nquads", Status.PARSE) do
    from_nquads(get(ENV, "GTS_WRAPPER_BAD_NQUADS", "<https://example/s> <https://example/p> .\n"))
end

mktempdir() do temp
    source_dir = joinpath(temp, "src")
    unpack_dir = joinpath(temp, "unpack")
    mkpath(source_dir)
    write(joinpath(source_dir, "a.txt"), "hello\n")

    packed = files_pack([source_dir])
    @test packed isa Vector{UInt8}
    @test !isempty(packed)

    expect_json_bool("files diff", files_diff_json(packed, source_dir), "clean", true)
    wrapped_source_dir = "x$source_dir"
    expect_json_bool(
        "files diff substring",
        files_diff_json(packed, SubString(wrapped_source_dir, 2, lastindex(wrapped_source_dir))),
        "clean",
        true,
    )

    expect_json_bool("files unpack", files_unpack(packed, unpack_dir), "ok", true)

    unpacked = joinpath(unpack_dir, "a.txt")
    @test isfile(unpacked)
    @test read(unpacked, String) == "hello\n"

    unpack_substring_dir = joinpath(temp, "unpack-substring")
    wrapped_unpack_dir = "x$unpack_substring_dir"
    expect_json_bool(
        "files unpack substring",
        files_unpack(packed, SubString(wrapped_unpack_dir, 2, lastindex(wrapped_unpack_dir))),
        "ok",
        true,
    )
end
