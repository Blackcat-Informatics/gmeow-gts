// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

import { Buffer } from "node:buffer";
import { RdfCursor, RdfProjectionError, requireRdf } from "./v2_rdf_cursor.js";

export const valueProjectionBytes = 2 ** 28;

type TypedValue =
    | [kind: 1, value: boolean]
    | [kind: 2 | 3, value: bigint]
    | [kind: 4, seconds: bigint, nanoseconds: bigint]
    | [kind: 5, value: Uint8Array];

export function readTypedValue(cursor: RdfCursor): TypedValue {
    const arity = cursor.array();
    requireRdf(arity > 0n, "wrong_length");
    const kind = cursor.code(5n);
    requireRdf(arity === (kind === 4 ? 3n : 2n), "wrong_length");
    switch (kind) {
        case 1:
            return [kind, cursor.bool()];
        case 2:
            return [kind, cursor.uint()];
        case 3:
            return [kind, cursor.int()];
        case 4: {
            const seconds = cursor.int();
            requireRdf(
                seconds >= -62135596800n && seconds <= 253402300799n,
                "out_of_range",
            );
            const nanoseconds = cursor.uint();
            requireRdf(nanoseconds < 1000000000n, "out_of_range");
            return [kind, seconds, nanoseconds];
        }
        case 5:
            return [kind, cursor.body(2)];
        default:
            throw new RdfProjectionError("unregistered_value");
    }
}

const xsd = "http://www.w3.org/2001/XMLSchema#";
export const hexDatatype = xsd + "hexBinary";

export function hexLexical(octets: Uint8Array): string {
    return Buffer.from(octets.buffer, octets.byteOffset, octets.byteLength)
        .toString("hex")
        .toUpperCase();
}

export function typedLiteral(value: TypedValue): {
    lexical: string;
    datatype: string;
} {
    switch (value[0]) {
        case 1:
            return { lexical: String(value[1]), datatype: xsd + "boolean" };
        case 2:
            return {
                lexical: value[1].toString(),
                datatype: xsd + "nonNegativeInteger",
            };
        case 3:
            return { lexical: value[1].toString(), datatype: xsd + "integer" };
        case 4: {
            // The admitted seconds fit exactly in JavaScript milliseconds.
            // Format the nanoseconds separately to preserve their full precision.
            const civil = new Date(Number(value[1]) * 1000)
                .toISOString()
                .slice(0, 19);
            const fraction =
                value[2] === 0n
                    ? ""
                    : "." +
                      value[2].toString().padStart(9, "0").replace(/0+$/, "");
            return {
                lexical: civil + fraction + "Z",
                datatype: xsd + "dateTime",
            };
        }
        case 5: {
            return { lexical: hexLexical(value[1]), datatype: hexDatatype };
        }
    }
}
