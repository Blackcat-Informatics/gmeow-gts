// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

/**
 * Normalise a blob frame's declared `pub.digest` to the `blake3:<hex>` text form, as the
 * Python and Rust readers do: text without the `blake3:` prefix gets it, and a raw 32-byte
 * string (the CDDL `digest` form) becomes lowercase hex. Anything else is not a declared
 * digest and yields undefined. The text is otherwise kept as written, so upper-case hex
 * does not equal the lowercase form the specification requires.
 */
export function declaredBlobDigest(value: unknown): string | undefined {
    if (typeof value === "string") {
        return value.startsWith("blake3:") ? value : `blake3:${value}`;
    }
    if (value instanceof Uint8Array && value.length === 32) {
        let hex = "";
        for (const b of value) hex += b.toString(16).padStart(2, "0");
        return `blake3:${hex}`;
    }
    return undefined;
}
