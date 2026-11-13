// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
//
// Deterministic map-key order (RFC 8949 §4.2, GTS-SPEC §4): keys are sorted
// bytewise on their encoded form, NOT length-first (RFC 7049). The two differ only
// when a map's keys mix major types, e.g. -1 (0x20) against 24 (0x18 0x18).

import { test } from "node:test";
import assert from "node:assert/strict";

import { encode } from "../src/wire.js";
import { encodeCanonical } from "../src/browser_wire.js";

// Produced by the Python reference engine: gts.wire.canonical of
// {1: 0, 24: 1, -1: 2, "a": 3, h'00': 4, 255: 5, -25: 6}.
const PYTHON_HEX = "a7010018180118ff052002381806410004616103";

function mixedKeyMap(): Map<unknown, number> {
    // Inserted in a scrambled order so the encoder must sort.
    return new Map<unknown, number>([
        ["a", 3],
        [-1, 2],
        [new Uint8Array([0]), 4],
        [24, 1],
        [-25, 6],
        [1, 0],
        [255, 5],
    ]);
}

test("Node encoder orders mixed-type keys bytewise, as the Python reference", () => {
    assert.equal(Buffer.from(encode(mixedKeyMap())).toString("hex"), PYTHON_HEX);
});

test("browser encoder orders mixed-type keys bytewise, as the Python reference", () => {
    assert.equal(
        Buffer.from(encodeCanonical(mixedKeyMap())).toString("hex"),
        PYTHON_HEX,
    );
});
