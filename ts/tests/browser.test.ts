// SPDX-FileCopyrightText: 2026 Blackcat Informatics(R) Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

import { readFileSync, readdirSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { test } from "node:test";
import assert from "node:assert/strict";

import {
    decrypt0WithWebCrypto,
    foldStream,
    foldStreamToSink,
    readStream,
    recipientKid,
    type BrowserFoldEvent,
} from "../src/browser.js";
import { BrowserWireError, cborItemLength } from "../src/browser_wire.js";

const __dirname = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(__dirname, "../../../");
const vectorsDir = join(repoRoot, "vectors");

function unhex(s: string): Uint8Array {
    return Uint8Array.from(Buffer.from(s, "hex"));
}

function vectorNames(): string[] {
    const names = new Set<string>();
    for (const file of readdirSync(vectorsDir)) {
        if (file.endsWith(".gts")) names.add(file.replace(/\.gts$/, ""));
    }
    return [...names].sort();
}

function eventKinds(events: BrowserFoldEvent[]): string[] {
    return events.map((event) => event.kind);
}

function streamableShape(
    items: {
        claimed: boolean;
        covered: number;
        tail: number;
        head?: Uint8Array;
    }[],
): { claimed: boolean; covered: number; tail: number; head?: Uint8Array }[] {
    return items.map((info) => ({
        claimed: info.claimed,
        covered: info.covered,
        tail: info.tail,
        head: info.head,
    }));
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

test("browser wire item length skips CBOR major-7 payload bytes", () => {
    assert.equal(
        cborItemLength(Uint8Array.of(0xfb, 0x3f, 0xf0, 0, 0, 0, 0, 0, 0), 0),
        9,
    );
    assert.equal(cborItemLength(Uint8Array.of(0xf9, 0x3c, 0), 0), 3);
    assert.throws(
        () => cborItemLength(Uint8Array.of(0xfb, 0x3f, 0xf0), 0),
        BrowserWireError,
    );
});

test("browser sink-only fold matches materialized browser fold for corpus", async () => {
    for (const name of vectorNames()) {
        const bytes = readFileSync(join(vectorsDir, `${name}.gts`));
        const materializedEvents: BrowserFoldEvent[] = [];
        const sinkEvents: BrowserFoldEvent[] = [];
        const materialized = await foldStream(chunkedStream(bytes, 19), {
            allowSegments: true,
            onEvent(event) {
                materializedEvents.push(event);
            },
        });
        const streamed = await foldStreamToSink(chunkedStream(bytes, 19), {
            allowSegments: true,
            onEvent(event) {
                sinkEvents.push(event);
            },
        });

        assert.deepEqual(
            streamed.diagnostics,
            materialized.graph.diagnostics,
            `${name}: diagnostics`,
        );
        assert.deepEqual(
            streamed.segmentHeads,
            materialized.graph.segmentHeads,
            `${name}: segment heads`,
        );
        assert.deepEqual(
            streamed.segmentProfiles,
            materialized.graph.segmentProfiles,
            `${name}: segment profiles`,
        );
        assert.deepEqual(
            streamed.segmentMeta,
            materialized.graph.segmentMeta,
            `${name}: segment metadata`,
        );
        assert.deepEqual(
            streamableShape(streamed.segmentStreamable),
            streamableShape(materialized.graph.segmentStreamable),
            `${name}: streamable layout`,
        );
        assert.deepEqual(
            eventKinds(sinkEvents),
            eventKinds(materializedEvents),
            `${name}: event kinds`,
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
