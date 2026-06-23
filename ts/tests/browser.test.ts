// SPDX-FileCopyrightText: 2026 Blackcat Informatics(R) Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

import { readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { test } from "node:test";
import assert from "node:assert/strict";

import {
    decrypt0WithWebCrypto,
    foldStream,
    readStream,
    recipientKid,
    type BrowserFoldEvent,
} from "../src/browser.js";

const __dirname = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(__dirname, "../../../");

function unhex(s: string): Uint8Array {
    return Uint8Array.from(Buffer.from(s, "hex"));
}

function chunkedStream(
    bytes: Uint8Array,
    chunkSize = 17,
    onClose?: () => void,
): ReadableStream<Uint8Array> {
    let offset = 0;
    return new ReadableStream<Uint8Array>({
        pull(controller) {
            if (offset >= bytes.length) {
                onClose?.();
                controller.close();
                return;
            }
            const end = Math.min(bytes.length, offset + chunkSize);
            controller.enqueue(bytes.subarray(offset, end));
            offset = end;
        },
    });
}

test("browser stream fold emits useful events before full materialization", async () => {
    const bytes = readFileSync(join(repoRoot, "vectors", "01-minimal.gts"));
    const events: BrowserFoldEvent[] = [];
    let closed = false;
    let usefulBeforeClose = false;
    const result = await foldStream(
        chunkedStream(bytes, 11, () => {
            closed = true;
        }),
        {
            allowSegments: false,
            onEvent(event) {
                events.push(event);
                if (
                    !closed &&
                    (event.kind === "term" ||
                        event.kind === "quad" ||
                        event.kind === "blob")
                ) {
                    usefulBeforeClose = true;
                }
            },
        },
    );

    assert.equal(usefulBeforeClose, true);
    assert.equal(result.graph.terms.length, 3);
    assert.equal(result.graph.quads.length, 1);
    assert.equal(result.graph.diagnostics.length, 0);
    assert.deepEqual(
        events.map((event) => event.kind),
        [
            "segment-start",
            "term",
            "term",
            "term",
            "quad",
            "streamable-layout",
            "segment-head",
        ],
    );
});

test("browser stream fold reports malformed input diagnostics without throwing", async () => {
    const valid = readFileSync(join(repoRoot, "vectors", "01-minimal.gts"));
    const torn = new Uint8Array(valid.length + 1);
    torn.set(valid);
    torn.set([0xa3], valid.length);

    const cases: [Uint8Array, string[]][] = [
        [new Uint8Array(), ["EmptyFile"]],
        [Uint8Array.of(0x01), ["DamagedFrame"]],
        [torn, ["TornAppendError"]],
    ];
    for (const [data, expected] of cases) {
        const graph = await readStream(chunkedStream(data), {
            allowSegments: false,
        });
        assert.deepEqual(
            graph.diagnostics.map((d) => d.code),
            expected,
        );
    }
});

test("browser stream fold verifies COSE_Sign1 with WebCrypto keys", async () => {
    const vector = JSON.parse(
        readFileSync(
            join(repoRoot, "vectors", "signed", "basic.json"),
            "utf-8",
        ),
    );
    const bytes = unhex(vector.gts);
    const pub = unhex(vector.pub);
    const signatureEvents: BrowserFoldEvent[] = [];
    const graph = await readStream(chunkedStream(bytes, 13), {
        allowSegments: false,
        keys: {
            verificationKey(kid) {
                return kid === vector.kid ? pub : null;
            },
        },
        onEvent(event) {
            if (event.kind === "signature") signatureEvents.push(event);
        },
    });

    assert.equal(graph.signatures.length, 2);
    assert.ok(
        graph.signatures.every(
            (sig) => sig.kid === vector.kid && sig.status === "valid",
        ),
    );
    assert.equal(signatureEvents.length, 2);
});

test("browser COSE_Encrypt0 helper opens AES-GCM vector with WebCrypto", async () => {
    const vector = JSON.parse(
        readFileSync(
            join(repoRoot, "vectors", "encrypt0", "basic.json"),
            "utf-8",
        ),
    );
    const cose = unhex(vector.cose);
    const key = unhex(vector.key);
    const plaintext = await decrypt0WithWebCrypto(cose, {
        contentKey(kid) {
            return kid === vector.kid ? key : null;
        },
    });

    assert.equal(recipientKid(cose), vector.kid);
    assert.deepEqual(plaintext, unhex(vector.plaintext));
});
