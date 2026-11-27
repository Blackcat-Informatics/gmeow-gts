// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import * as ed from "@noble/ed25519";
import { ml_dsa65 } from "@noble/post-quantum/ml-dsa.js";
import { sha512 } from "@noble/hashes/sha2.js";
import {
    COMPOSITE_ALGORITHM,
    ED25519_ALGORITHM,
    heldKeyIdentifier,
    signatureSubject,
    verifySignature,
    type HeldKey,
    type SignatureRecord,
} from "../src/v2_signature.js";

const COMPOSITE_PREFIX = new TextEncoder().encode(
    "CompositeAlgorithmSignatures2025",
);
const COMPOSITE_LABEL = new TextEncoder().encode(
    "COMPSIG-MLDSA65-Ed25519-SHA512",
);

const P6 = new URL("../../../vectors/v2/P6/", import.meta.url);
const hex = (value: string): Uint8Array =>
    Uint8Array.from(Buffer.from(value, "hex"));
const bytes = (name: string): Uint8Array =>
    new Uint8Array(readFileSync(new URL(name, P6)));

const sign = JSON.parse(
    readFileSync(new URL("sign-composite-reproduce.json", P6), "utf8"),
);
const signer = sign.cases[0];
const publicKey = bytes(signer.public_key_file);
const value = bytes(signer.value_file);
const keyId = hex(signer.signer.key_id);
const h0 = hex(signer.subject_fields.header_identifier);
const frameIdentifier = hex(signer.subject_fields.frame_identifier);

const heldKey: HeldKey = { keyId, algorithm: COMPOSITE_ALGORITHM, publicKey };
const record: SignatureRecord = {
    algorithm: COMPOSITE_ALGORITHM,
    keyId,
    value,
};

function withByte(source: Uint8Array, index: number): Uint8Array {
    const copy = new Uint8Array(source);
    copy[index] ^= 0x01;
    return copy;
}

test("the composite key identifier and subject match the reproduction vector", () => {
    assert.deepEqual(heldKeyIdentifier(COMPOSITE_ALGORITHM, publicKey), keyId);
    assert.equal(
        Buffer.from(signatureSubject(record, h0, frameIdentifier)).toString(
            "hex",
        ),
        signer.result.message,
    );
});

test("a composite signature verifies under its held key", () => {
    assert.deepEqual(verifySignature(record, h0, frameIdentifier, [heldKey]), {
        status: "valid",
        algorithm: COMPOSITE_ALGORITHM,
        keyId,
    });
});

test("damage to either half of a composite value is refused by that half", () => {
    const mldsaHalf = verifySignature(
        { ...record, value: withByte(value, 0) },
        h0,
        frameIdentifier,
        [heldKey],
    );
    assert.equal(mldsaHalf.status, "invalid");
    assert.equal(mldsaHalf.reason, "composite_post_quantum_failed");

    const ed25519Half = verifySignature(
        { ...record, value: withByte(value, value.length - 1) },
        h0,
        frameIdentifier,
        [heldKey],
    );
    assert.equal(ed25519Half.status, "invalid");
    assert.equal(ed25519Half.reason, "composite_classical_failed");
});

test("a composite value is refused under a different subject", () => {
    const other = verifySignature(record, h0, new Uint8Array(32), [heldKey]);
    assert.equal(other.status, "invalid");
    assert.equal(other.reason, "composite_classical_failed");
});

test("a composite signature is unverified without its key and invalid for a mismatched key", () => {
    const absent = verifySignature(record, h0, frameIdentifier, []);
    assert.equal(absent.status, "unverified");
    assert.equal(absent.reason, "key_not_held");

    const mismatched = verifySignature(record, h0, frameIdentifier, [
        { ...heldKey, publicKey: withByte(publicKey, 0) },
    ]);
    assert.equal(mismatched.status, "invalid");
    assert.equal(mismatched.reason, "key_mismatch");
});

const ed25519Sign = JSON.parse(
    readFileSync(new URL("sign-ed25519-reproduce.json", P6), "utf8"),
).cases[0];
const ed25519Key = hex(ed25519Sign.signer.public_key);
const ed25519Value = hex(ed25519Sign.result.value);
const ed25519Id = hex(ed25519Sign.signer.key_id);
const ed25519Held: HeldKey = {
    keyId: ed25519Id,
    algorithm: ED25519_ALGORITHM,
    publicKey: ed25519Key,
};
const ed25519Record: SignatureRecord = {
    algorithm: ED25519_ALGORITHM,
    keyId: ed25519Id,
    value: ed25519Value,
};
const identityPoint = hex("01" + "00".repeat(31));
const groupOrder = hex(
    "edd3f55c1a631258d69cf7a2def9de1400000000000000000000000000000010",
);

function ed25519Outcome(
    record: SignatureRecord,
    frame: Uint8Array = frameIdentifier,
    held: HeldKey = ed25519Held,
) {
    return verifySignature(record, h0, frame, [held]);
}

test("an Ed25519 signature from the reproduction vector verifies", () => {
    assert.equal(ed25519Outcome(ed25519Record).status, "valid");
});

test("a scalar not below the group order is refused", () => {
    const value = new Uint8Array(ed25519Value);
    value.set(groupOrder, 32);
    const result = ed25519Outcome({ ...ed25519Record, value });
    assert.equal(result.reason, "non_canonical_scalar");
});

test("a small-order public key is refused before the equation", () => {
    const held: HeldKey = {
        keyId: heldKeyIdentifier(ED25519_ALGORITHM, identityPoint),
        algorithm: ED25519_ALGORITHM,
        publicKey: identityPoint,
    };
    const record = { ...ed25519Record, keyId: held.keyId };
    assert.equal(
        ed25519Outcome(record, frameIdentifier, held).reason,
        "small_order_key",
    );
});

test("a small-order nonce is refused before the equation", () => {
    const value = new Uint8Array(ed25519Value);
    value.set(identityPoint, 0);
    assert.equal(
        ed25519Outcome({ ...ed25519Record, value }).reason,
        "small_order_nonce",
    );
});

test("an Ed25519 signature over a different subject fails the equation", () => {
    assert.equal(
        ed25519Outcome(ed25519Record, new Uint8Array(32)).reason,
        "equation_failed",
    );
});

// The two cases of P6 that no corpus vector holds yet: the halves of two valid composite
// signatures combined, and a composite held under one of its component keys only.
function compositeTestKey(privateKey: Uint8Array): {
    publicKey: Uint8Array;
    mldsaSecret: Uint8Array;
    ed25519Secret: Uint8Array;
} {
    const { publicKey: mldsaPublic, secretKey } = ml_dsa65.keygen(
        privateKey.subarray(0, 32),
    );
    const ed25519Public = ed.getPublicKey(privateKey.subarray(32));
    return {
        publicKey: new Uint8Array([...mldsaPublic, ...ed25519Public]),
        mldsaSecret: secretKey,
        ed25519Secret: privateKey.subarray(32),
    };
}

function compositeHalves(
    key: ReturnType<typeof compositeTestKey>,
    h0Value: Uint8Array,
    frame: Uint8Array,
): { mldsa: Uint8Array; ed25519: Uint8Array } {
    const subject = signatureSubject(
        {
            algorithm: COMPOSITE_ALGORITHM,
            keyId: heldKeyIdentifier(COMPOSITE_ALGORITHM, key.publicKey),
            value: new Uint8Array(),
        },
        h0Value,
        frame,
    );
    const representative = new Uint8Array([
        ...COMPOSITE_PREFIX,
        ...COMPOSITE_LABEL,
        0,
        ...sha512(subject),
    ]);
    return {
        mldsa: ml_dsa65.sign(representative, key.mldsaSecret, {
            context: COMPOSITE_LABEL,
            extraEntropy: false,
        }),
        ed25519: ed.sign(representative, key.ed25519Secret),
    };
}

test("the halves of two valid composite signatures do not combine", () => {
    const first = compositeTestKey(hex(signer.signer.private_key));
    const second = compositeTestKey(hex("80".repeat(32) + "a0".repeat(32)));
    const firstId = heldKeyIdentifier(COMPOSITE_ALGORITHM, first.publicKey);
    const secondId = heldKeyIdentifier(COMPOSITE_ALGORITHM, second.publicKey);
    const keys: HeldKey[] = [
        {
            keyId: firstId,
            algorithm: COMPOSITE_ALGORITHM,
            publicKey: first.publicKey,
        },
        {
            keyId: secondId,
            algorithm: COMPOSITE_ALGORITHM,
            publicKey: second.publicKey,
        },
    ];
    const firstHalves = compositeHalves(first, h0, frameIdentifier);
    const secondHalves = compositeHalves(second, h0, frameIdentifier);
    const valid = {
        algorithm: COMPOSITE_ALGORITHM,
        keyId: firstId,
        value: new Uint8Array([...firstHalves.mldsa, ...firstHalves.ed25519]),
    };
    assert.equal(
        verifySignature(valid, h0, frameIdentifier, keys).status,
        "valid",
    );
    const swapped = {
        algorithm: COMPOSITE_ALGORITHM,
        keyId: firstId,
        value: new Uint8Array([...firstHalves.mldsa, ...secondHalves.ed25519]),
    };
    const outcome = verifySignature(swapped, h0, frameIdentifier, keys);
    assert.equal(outcome.status, "invalid");
    assert.equal(outcome.reason, "composite_classical_failed");
});

test("a composite is not verified under one of its component keys", () => {
    const key = compositeTestKey(hex(signer.signer.private_key));
    const halves = compositeHalves(key, h0, frameIdentifier);
    const record: SignatureRecord = {
        algorithm: COMPOSITE_ALGORITHM,
        keyId: heldKeyIdentifier(COMPOSITE_ALGORITHM, key.publicKey),
        value: new Uint8Array([...halves.mldsa, ...halves.ed25519]),
    };
    const edPublic = ed.getPublicKey(key.ed25519Secret);
    const edKeyId = heldKeyIdentifier(ED25519_ALGORITHM, edPublic);
    assert.deepEqual(
        verifySignature(record, h0, frameIdentifier, [
            {
                keyId: edKeyId,
                algorithm: ED25519_ALGORITHM,
                publicKey: edPublic,
            },
        ]),
        {
            status: "unverified",
            reason: "key_not_held",
            algorithm: COMPOSITE_ALGORITHM,
            keyId: record.keyId,
        },
    );
    assert.equal(
        verifySignature(record, h0, frameIdentifier, [
            {
                keyId: record.keyId,
                algorithm: COMPOSITE_ALGORITHM,
                publicKey: edPublic,
            },
        ]).reason,
        "key_mismatch",
    );
});
