// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

import CGts
import Foundation

#if os(Linux)
import Glibc
#else
import Darwin
#endif

public struct GtsStatus: Equatable, Hashable, Sendable, CustomStringConvertible {
    public let rawValue: UInt32

    public init(rawValue: UInt32) {
        self.rawValue = rawValue
    }

    public static let ok = GtsStatus(rawValue: 0)
    public static let invalidArgument = GtsStatus(rawValue: 1)
    public static let io = GtsStatus(rawValue: 2)
    public static let parse = GtsStatus(rawValue: 3)
    public static let diagnostic = GtsStatus(rawValue: 4)
    public static let internalError = GtsStatus(rawValue: 5)
    public static let panic = GtsStatus(rawValue: 6)

    public var name: String {
        switch self {
        case .ok:
            return "OK"
        case .invalidArgument:
            return "INVALID_ARGUMENT"
        case .io:
            return "IO"
        case .parse:
            return "PARSE"
        case .diagnostic:
            return "DIAGNOSTIC"
        case .internalError:
            return "INTERNAL"
        case .panic:
            return "PANIC"
        default:
            return "UNKNOWN"
        }
    }

    public var description: String {
        name
    }
}

public struct GtsUnpackFlags: OptionSet, Sendable {
    public let rawValue: UInt32

    public init(rawValue: UInt32) {
        self.rawValue = rawValue
    }

    public static let includeSuppressed = GtsUnpackFlags(rawValue: UInt32(GTS_UNPACK_INCLUDE_SUPPRESSED))
    public static let allowSymlinks = GtsUnpackFlags(rawValue: UInt32(GTS_UNPACK_ALLOW_SYMLINKS))
    public static let allowSpecial = GtsUnpackFlags(rawValue: UInt32(GTS_UNPACK_ALLOW_SPECIAL))
    public static let sameOwner = GtsUnpackFlags(rawValue: UInt32(GTS_UNPACK_SAME_OWNER))
    public static let preserveSetID = GtsUnpackFlags(rawValue: UInt32(GTS_UNPACK_PRESERVE_SETID))
}

public struct GtsError: Error, CustomStringConvertible {
    public let operation: String
    public let status: GtsStatus
    public let code: String
    public let detail: String

    public var description: String {
        var message = "\(operation) failed with \(status.name)"
        if !code.isEmpty {
            message += " (\(code))"
        }
        if !detail.isEmpty {
            message += ": \(detail)"
        }
        return message
    }
}

public enum GTS {
    public static var abiVersion: UInt32 {
        gts_abi_version()
    }

    public static var version: String {
        copyCString(gts_version())
    }

    public static func buildMetadataJSON() throws -> String {
        try callString("gts_build_metadata_json") { out, error in
            gts_build_metadata_json(out, error)
        }
    }

    public static func capabilitiesJSON() throws -> String {
        try callString("gts_capabilities_json") { out, error in
            gts_capabilities_json(out, error)
        }
    }

    public static func readJSON(_ data: Data) throws -> String {
        try withUnsafeBytes(data) { pointer in
            try callString("gts_read_json") { out, error in
                gts_read_json(pointer, data.count, out, error)
            }
        }
    }

    public static func verifyJSON(_ data: Data) throws -> String {
        try withUnsafeBytes(data) { pointer in
            try callString("gts_verify_json") { out, error in
                gts_verify_json(pointer, data.count, out, error)
            }
        }
    }

    public static func toNQuads(_ data: Data) throws -> String {
        try withUnsafeBytes(data) { pointer in
            try callString("gts_to_nquads") { out, error in
                gts_to_nquads(pointer, data.count, out, error)
            }
        }
    }

    public static func fromNQuads(_ text: String) throws -> Data {
        let bytes = Array(text.utf8)
        return try bytes.withUnsafeBufferPointer { buffer in
            let pointer = buffer.baseAddress.map {
                UnsafeRawPointer($0).assumingMemoryBound(to: CChar.self)
            }
            return try callData("gts_from_nquads") { out, error in
                gts_from_nquads(pointer, bytes.count, out, error)
            }
        }
    }

    public static func filesPack(paths: [String]) throws -> Data {
        guard !paths.isEmpty else {
            throw invalidArgument("gts_files_pack", "paths must not be empty")
        }

        var nativePaths: [UnsafePointer<CChar>?] = []
        nativePaths.reserveCapacity(paths.count)
        defer {
            for path in nativePaths {
                if let path {
                    free(UnsafeMutableRawPointer(mutating: path))
                }
            }
        }

        for path in paths {
            guard !path.utf8.contains(0) else {
                throw invalidArgument("gts_files_pack", "path entries must not contain NUL bytes")
            }
            guard let duplicate = strdup(path) else {
                throw GtsError(
                    operation: "gts_files_pack",
                    status: .internalError,
                    code: "gts.swift.allocation_failed",
                    detail: "Unable to allocate a native path string."
                )
            }
            nativePaths.append(UnsafePointer(duplicate))
        }

        return try nativePaths.withUnsafeBufferPointer { buffer in
            try callData("gts_files_pack") { out, error in
                gts_files_pack(buffer.baseAddress, buffer.count, out, error)
            }
        }
    }

    public static func filesUnpack(
        _ data: Data,
        to destination: String,
        flags: GtsUnpackFlags = []
    ) throws -> String {
        guard !destination.utf8.contains(0) else {
            throw invalidArgument("gts_files_unpack", "destination must not contain NUL bytes")
        }

        return try destination.withCString { destinationPointer in
            try withUnsafeBytes(data) { dataPointer in
                try callString("gts_files_unpack") { out, error in
                    gts_files_unpack(dataPointer, data.count, destinationPointer, flags.rawValue, out, error)
                }
            }
        }
    }

    public static func filesDiffJSON(_ data: Data, directory: String) throws -> String {
        guard !directory.utf8.contains(0) else {
            throw invalidArgument("gts_files_diff_json", "directory must not contain NUL bytes")
        }

        return try directory.withCString { directoryPointer in
            try withUnsafeBytes(data) { dataPointer in
                try callString("gts_files_diff_json") { out, error in
                    gts_files_diff_json(dataPointer, data.count, directoryPointer, out, error)
                }
            }
        }
    }

    private static func callString(
        _ operation: String,
        _ call: (UnsafeMutablePointer<gts_buffer>?, UnsafeMutablePointer<OpaquePointer?>?) -> gts_status
    ) throws -> String {
        let data = try callData(operation, call)
        return String(decoding: data, as: UTF8.self)
    }

    private static func callData(
        _ operation: String,
        _ call: (UnsafeMutablePointer<gts_buffer>?, UnsafeMutablePointer<OpaquePointer?>?) -> gts_status
    ) throws -> Data {
        var output = gts_buffer()
        var error: OpaquePointer?
        defer {
            gts_buffer_free(&output)
        }

        let status = GtsStatus(cStatus: call(&output, &error))
        guard status == .ok else {
            throw buildError(operation: operation, status: status, error: error)
        }
        if let error {
            throw buildError(operation: operation, status: GtsStatus.internalError, error: error)
        }
        return try copyBuffer(output, operation: operation)
    }

    private static func withUnsafeBytes<T>(
        _ data: Data,
        _ body: (UnsafePointer<UInt8>?) throws -> T
    ) throws -> T {
        try data.withUnsafeBytes { rawBuffer in
            try body(rawBuffer.bindMemory(to: UInt8.self).baseAddress)
        }
    }

    private static func copyBuffer(_ buffer: gts_buffer, operation: String) throws -> Data {
        let length = Int(buffer.len)
        if length == 0 {
            return Data()
        }
        guard let data = buffer.data else {
            throw GtsError(
                operation: operation,
                status: .internalError,
                code: "gts.swift.null_buffer",
                detail: "C ABI returned a null data pointer with non-zero length."
            )
        }
        return Data(bytes: data, count: length)
    }

    private static func copyCString(_ value: UnsafePointer<CChar>?) -> String {
        guard let value else {
            return ""
        }
        return String(cString: value)
    }

    private static func buildError(
        operation: String,
        status: GtsStatus,
        error: OpaquePointer?
    ) -> GtsError {
        var code = ""
        var detail = ""
        if let error {
            code = copyCString(gts_error_code(error))
            detail = copyCString(gts_error_message(error))
            gts_error_free(error)
        }
        return GtsError(operation: operation, status: status, code: code, detail: detail)
    }

    private static func invalidArgument(_ operation: String, _ detail: String) -> GtsError {
        GtsError(
            operation: operation,
            status: .invalidArgument,
            code: "gts.swift.invalid_argument",
            detail: detail
        )
    }
}

private extension GtsStatus {
    init(cStatus: gts_status) {
        self.init(rawValue: UInt32(cStatus.rawValue))
    }
}
