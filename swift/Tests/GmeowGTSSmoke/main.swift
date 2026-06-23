// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

import Foundation
import GmeowGTS

enum SmokeError: Error, CustomStringConvertible {
    case failure(String)

    var description: String {
        switch self {
        case let .failure(message):
            return message
        }
    }
}

func require(_ condition: @autoclosure () -> Bool, _ message: String) throws {
    if !condition() {
        throw SmokeError.failure(message)
    }
}

func expectContains(_ label: String, _ haystack: String, _ needle: String) throws {
    try require(haystack.contains(needle), "\(label) did not contain \(needle)")
}

func compactJSON(_ json: String) -> String {
    json.replacingOccurrences(of: #"\s+"#, with: "", options: .regularExpression)
}

func expectJSONString(_ label: String, _ json: String, key: String, expected: String) throws {
    let needle = #""\#(key)":"\#(expected)""#
    try expectContains(label, compactJSON(json), needle)
}

func expectJSONBool(_ label: String, _ json: String, key: String, expected: Bool) throws {
    let needle = #""\#(key)":\#(expected ? "true" : "false")"#
    try expectContains(label, compactJSON(json), needle)
}

func expectDiagnostic(_ label: String, _ json: String, code: String) throws {
    try expectContains(label, compactJSON(json), #""code":"\#(code)""#)
}

func expectGtsError(_ label: String, status: GtsStatus, _ body: () throws -> Void) throws {
    do {
        try body()
    } catch let error as GtsError {
        try require(error.status == status, "\(label) returned unexpected status")
        try require(!error.code.isEmpty, "\(label) structured error code was empty")
        try require(!error.detail.isEmpty, "\(label) structured error detail was empty")
        return
    }
    throw SmokeError.failure("\(label) did not fail")
}

let arguments = CommandLine.arguments
try require(
    arguments.count == 4,
    "usage: GmeowGTSSmoke vectors/01-minimal.gts vectors/04-damaged-frame.gts vectors/28-empty-file.gts"
)

let vector = URL(fileURLWithPath: arguments[1])
let input = try Data(contentsOf: vector)
let damaged = try Data(contentsOf: URL(fileURLWithPath: arguments[2]))
let empty = try Data(contentsOf: URL(fileURLWithPath: arguments[3]))

try require(GTS.abiVersion == 1, "unexpected ABI version")
try require(!GTS.version.isEmpty, "empty library version")

try expectJSONString("build metadata", try GTS.buildMetadataJSON(), key: "schema", expected: "gts-capi-build-v1")
try expectJSONString("capabilities", try GTS.capabilitiesJSON(), key: "schema", expected: "gts-capi-capabilities-v1")
let cleanRead = try GTS.readJSON(input)
try expectJSONString("swift clean-read read JSON", cleanRead, key: "schema", expected: "gts-capi-read-v1")
try expectJSONBool("swift clean-read read JSON", cleanRead, key: "clean", expected: true)
try expectJSONString("verify JSON", try GTS.verifyJSON(input), key: "schema", expected: "gts-capi-verify-v1")

let damagedRead = try GTS.readJSON(damaged)
try expectJSONString("swift damaged-diagnostic-read read JSON", damagedRead, key: "schema", expected: "gts-capi-read-v1")
try expectJSONBool("swift damaged-diagnostic-read read JSON", damagedRead, key: "clean", expected: false)
try expectDiagnostic("swift damaged-diagnostic-read read JSON", damagedRead, code: "DamagedFrame")
try expectGtsError("swift damaged-diagnostic-read toNQuads", status: .diagnostic) {
    _ = try GTS.toNQuads(damaged)
}

let emptyRead = try GTS.readJSON(empty)
try expectJSONString("swift empty-malformed-refusal read JSON", emptyRead, key: "schema", expected: "gts-capi-read-v1")
try expectJSONBool("swift empty-malformed-refusal read JSON", emptyRead, key: "clean", expected: false)
try expectDiagnostic("swift empty-malformed-refusal read JSON", emptyRead, code: "EmptyFile")
try expectGtsError("swift empty-malformed-refusal toNQuads", status: .diagnostic) {
    _ = try GTS.toNQuads(empty)
}

let nquads = try GTS.toNQuads(input)
try expectContains("N-Quads", nquads, #""Cat"@en"#)

let roundTrip = try GTS.fromNQuads(nquads)
try require(!roundTrip.isEmpty, "round-trip GTS output was empty")

try expectGtsError("swift malformed-nquads-refusal fromNQuads", status: .parse) {
    _ = try GTS.fromNQuads(
        ProcessInfo.processInfo.environment["GTS_WRAPPER_BAD_NQUADS"] ??
            "<https://example/s> <https://example/p> .\n"
    )
}

let fileManager = FileManager.default
let root = URL(fileURLWithPath: NSTemporaryDirectory())
    .appendingPathComponent("gmeow-gts-swift-\(UUID().uuidString)", isDirectory: true)
let sourceDirectory = root.appendingPathComponent("src", isDirectory: true)
let unpackDirectory = root.appendingPathComponent("unpack", isDirectory: true)
defer {
    try? fileManager.removeItem(at: root)
}

try fileManager.createDirectory(at: sourceDirectory, withIntermediateDirectories: true)
try Data("hello\n".utf8).write(to: sourceDirectory.appendingPathComponent("a.txt"))

let packed = try GTS.filesPack(paths: [sourceDirectory.path])
try expectJSONBool("files diff", try GTS.filesDiffJSON(packed, directory: sourceDirectory.path), key: "clean", expected: true)
try expectJSONBool("files unpack", try GTS.filesUnpack(packed, to: unpackDirectory.path), key: "ok", expected: true)
let unpacked = try Data(contentsOf: unpackDirectory.appendingPathComponent("a.txt"))
try require(String(decoding: unpacked, as: UTF8.self) == "hello\n", "unpacked file content mismatch")

print("Swift C ABI wrapper smoke test passed")
