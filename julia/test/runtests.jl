# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0

using GmeowGTS
using Test

vector_path = length(ARGS) == 1 ? ARGS[1] : get(ENV, "GTS_JULIA_VECTOR", "")
if isempty(vector_path)
    @info "Skipping external GTS smoke test; pass a vector path as the sole argument to run it."
    exit(0)
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

input = read(vector_path)

@test abi_version() == ABI_VERSION
@test !isempty(libgts_version())

expect_json_string("build metadata", build_metadata_json(), "schema", "gts-capi-build-v1")
expect_json_string("capabilities", capabilities_json(), "schema", "gts-capi-capabilities-v1")
expect_json_string("read JSON", read_json(input), "schema", "gts-capi-read-v1")
expect_json_string("verify JSON", verify_json(input), "schema", "gts-capi-verify-v1")

nquads = to_nquads(input)
@test occursin("\"Cat\"@en", nquads)

round_trip = from_nquads(nquads)
@test round_trip isa Vector{UInt8}
@test !isempty(round_trip)

parse_error = try
    from_nquads("<https://example/s> <https://example/p> .\n")
    nothing
catch error
    error
end
@test parse_error isa GtsError
@test parse_error.status == Status.PARSE
@test !isempty(parse_error.code)
@test !isempty(parse_error.detail)

mktempdir() do temp
    source_dir = joinpath(temp, "src")
    unpack_dir = joinpath(temp, "unpack")
    mkpath(source_dir)
    write(joinpath(source_dir, "a.txt"), "hello\n")

    packed = files_pack([source_dir])
    @test packed isa Vector{UInt8}
    @test !isempty(packed)

    expect_json_bool("files diff", files_diff_json(packed, source_dir), "clean", true)
    expect_json_bool("files unpack", files_unpack(packed, unpack_dir), "ok", true)

    unpacked = joinpath(unpack_dir, "a.txt")
    @test isfile(unpacked)
    @test read(unpacked, String) == "hello\n"
end
