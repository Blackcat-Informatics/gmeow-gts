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

let arguments = CommandLine.arguments
try require(arguments.count == 2, "usage: GmeowGTSSmoke vectors/01-minimal.gts")

let vector = URL(fileURLWithPath: arguments[1])
let input = try Data(contentsOf: vector)

try require(GTS.abiVersion == 1, "unexpected ABI version")
try require(!GTS.version.isEmpty, "empty library version")

try expectJSONString("build metadata", try GTS.buildMetadataJSON(), key: "schema", expected: "gts-capi-build-v1")
try expectJSONString("capabilities", try GTS.capabilitiesJSON(), key: "schema", expected: "gts-capi-capabilities-v1")
try expectJSONString("read JSON", try GTS.readJSON(input), key: "schema", expected: "gts-capi-read-v1")
try expectJSONString("verify JSON", try GTS.verifyJSON(input), key: "schema", expected: "gts-capi-verify-v1")

let nquads = try GTS.toNQuads(input)
try expectContains("N-Quads", nquads, #""Cat"@en"#)

let roundTrip = try GTS.fromNQuads(nquads)
try require(!roundTrip.isEmpty, "round-trip GTS output was empty")

do {
    _ = try GTS.fromNQuads("<https://example/s> <https://example/p> .\n")
    throw SmokeError.failure("bad N-Quads did not fail")
} catch let error as GtsError {
    try require(error.status == .parse, "structured error status was not parse")
    try require(!error.code.isEmpty, "structured error code was empty")
    try require(!error.detail.isEmpty, "structured error detail was empty")
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
