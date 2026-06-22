// swift-tools-version: 6.0
// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

import PackageDescription

let package = Package(
    name: "GmeowGTS",
    platforms: [
        .macOS(.v13)
    ],
    products: [
        .library(name: "GmeowGTS", targets: ["GmeowGTS"])
    ],
    targets: [
        .systemLibrary(
            name: "CGts",
            path: "swift/Sources/CGts"
        ),
        .target(
            name: "GmeowGTS",
            dependencies: ["CGts"],
            path: "swift/Sources/GmeowGTS"
        ),
        .executableTarget(
            name: "GmeowGTSSmoke",
            dependencies: ["GmeowGTS"],
            path: "swift/Tests/GmeowGTSSmoke"
        )
    ]
)
