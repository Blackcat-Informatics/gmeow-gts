// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
//
// Pins how `cose.verifySig` treats degenerate Ed25519 signatures (§9.2). Each
// case is a well-formed COSE_Sign1 whose signature was built with the secret
// key, so none is a forgery. The outcomes follow @noble/ed25519 3.x with its
// default (ZIP-215) options; 2.3.0 gave the opposite answer in every case here.

import { test } from "node:test";
import assert from "node:assert/strict";

import { verifySig } from "../src/cose.js";

const unhex = (s: string): Uint8Array => Uint8Array.from(Buffer.from(s, "hex"));

// Public key of the fixed test seed, and two frame ids it signs.
const PUB = unhex("4647f65e1aafdea1d27399bf1074297c827c1bcd02e20e6f61e214ab09f1070d");
const FID1 = unhex("2448c2128baf38f334dadf863cacc9582b2815466bad9e69159a3ccdf486c10d");
const FID2 = unhex("521061b47ec459905a37f77dc55462a74212a0cb0ffa4f307e44cb98028214f3");

// COSE_Sign1 (tag 18) with protected header {1: -8}, kid "probe-kid", detached payload.
function cose(sig: Uint8Array): Uint8Array {
    return Buffer.concat([
        unhex("d28443a10127a10449"),
        Buffer.from("probe-kid"),
        unhex("f65840"),
        sig,
    ]);
}

const IDENTITY = "0100000000000000000000000000000000000000000000000000000000000000";
const NON_CANONICAL = "edffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f";

test("identity R with S = H(empty)·a does not verify for any frame id", () => {
    // 2.3.0 failed to encode the identity point, swallowed the error and hashed an
    // empty input, so this one signature verified for every frame id. It must not.
    const sig = unhex(IDENTITY + "9f7961e66ad354343ffe8bf1d00a997a8c00e293703b904e56ccb31ee464d30d");
    assert.equal(verifySig(cose(sig), FID1, PUB), "invalid");
    assert.equal(verifySig(cose(sig), FID2, PUB), "invalid");
});

test("identity R with S computed over R, A and the message verifies (RFC 8032)", () => {
    const sig = unhex(IDENTITY + "45c3d2f03302be706318aeb2c0f795605ca517b312417c8fd421b40891400504");
    assert.equal(verifySig(cose(sig), FID1, PUB), "valid");
});

test("non-canonical R is hashed as received (ZIP-215)", () => {
    // R is y = p (order 4). S over the raw R bytes verifies; S over the canonical
    // re-encoding of R does not.
    const raw = unhex(NON_CANONICAL + "d9ff9390f7d02ab2fb8c182c5273ba8cdfa94016765d1122837c678b34d29405");
    const canonical = unhex(NON_CANONICAL + "28d27d6ebdc33ba06fe91a259af8b0b9699df40ccd0fc83c254d5ce30b420807");
    assert.equal(verifySig(cose(raw), FID1, PUB), "valid");
    assert.equal(verifySig(cose(canonical), FID1, PUB), "invalid");
});
