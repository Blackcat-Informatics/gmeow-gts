// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import { FramingError, inspectFraming, itemEnd } from "../src/v2_framing.js";
import { projectRdf, rdfNQuads } from "../src/v2_rdf_projection.js";

test("framing reports unsupported wire generations without rounding", () => {
    const firstItem = Buffer.from("82011bffffffffffffffff", "hex");
    assert.deepEqual(inspectFraming(firstItem), {
        kind: "unsupported",
        generation: 18446744073709551615n,
    });
});

test("a resync prefix requires the fixed marker before exposing its registry", () => {
    const valid =
        "8b050250fcf3eb32a2f0267dd16a53f09bff8792010181018101008101805820" +
        "00".repeat(32);
    const prefix = inspectFraming(Buffer.from(valid, "hex"));
    assert.equal(prefix.kind, "v2");
    const [resync] = prefix.records;
    assert.equal(resync.kind, "resync");
    assert.equal(resync.reason, undefined);

    const invalid = inspectFraming(
        Buffer.from(valid.replace("fcf3", "00f3"), "hex"),
    );
    assert.equal(invalid.kind, "v2");
    assert.deepEqual(
        [...invalid.records],
        [
            {
                kind: "resync",
                offset: 0,
                end: valid.length / 2,
                reason: "out_of_range",
            },
        ],
    );
});

test("resync size is checked at the first byte of nested option fields", () => {
    const prefix = Buffer.from(
        "8b050250fcf3eb32a2f0267dd16a53f09bff879202098101820282",
        "hex",
    );
    const cases = [
        [4061, [0x82, 2, 0], "wrong_type"],
        [4062, [0x82, 2, 0], "out_of_range"],
        [4062, [0x81, 0], "unregistered_value"],
        [4063, [0x81, 0], "out_of_range"],
        [4061, [0x82, 2, 0x18, 0], "not_deterministic"],
    ] as const;
    for (const [size, past, reason] of cases) {
        const data = Buffer.concat([
            prefix,
            Buffer.of(0x59, size >> 8, size & 0xff),
            Buffer.alloc(size),
            Buffer.of(1, 0, ...past, 0x80, 0x40),
        ]);
        const result = inspectFraming(data);
        assert.equal(result.kind, "v2");
        const [opener] = result.records;
        assert.equal(opener.kind, "resync");
        assert.equal(opener.reason, reason);
    }
});

test("frame envelope validation preserves the first field error", () => {
    const cases = [
        ["P1/frame-flags-order", "forbidden_combination"],
        ["P1/envelope-field-before-content-value", "wrong_type"],
        ["P1/frame-compressed-empty", "forbidden_combination"],
        ["P1/stored-check-short", "wrong_length"],
        ["P1/frame-unknown-content-bad-utf8", "wrong_type"],
        ["P1/frame-storage-tag-newer", undefined],
        ["P6/sig-list-grammar-first", "wrong_length"],
        ["P6/sig-list-unordered", "unordered_or_repeated"],
        ["P6/sig-duplicate-records", "unordered_or_repeated"],
    ] as const;
    for (const [fixture, reason] of cases) {
        const path = new URL(
            `../../../vectors/v2/${fixture}.bin`,
            import.meta.url,
        );
        const result = inspectFraming(readFileSync(path));
        assert.equal(result.kind, "v2", fixture);
        const last = [...result.records].at(-1);
        assert.equal(last?.kind, "frame", fixture);
        assert.equal(last.reason, reason, fixture);
    }
});

test("an unreadable opener leaves its frames unchecked until the next opener", () => {
    const file = readFileSync(
        new URL(
            "../../../vectors/v2/P1/frame-flags-order.bin",
            import.meta.url,
        ),
    );
    const malformedHeader = Buffer.from("8701020101f481018101", "hex");
    const result = inspectFraming(
        Buffer.concat([malformedHeader, file.subarray(11), file]),
    );
    assert.equal(result.kind, "v2");
    const records = [...result.records];
    assert.deepEqual(
        records.map(({ kind, reason }) => [kind, reason]),
        [
            ["header", "wrong_type"],
            ["unchecked", undefined],
            ["header", undefined],
            ["frame", "forbidden_combination"],
        ],
    );
});

test("only the adjacent frame can witness healing, including a transparent frame", () => {
    const cases = [
        ["link-healed-before-transparent", ["healed", "linked"]],
        ["link-heal-opaque-between", ["linked", "broken", "broken", "linked"]],
    ] as const;
    for (const [name, expected] of cases) {
        const input = readFileSync(
            new URL(`../../../vectors/v2/P1/${name}.bin`, import.meta.url),
        );
        const result = inspectFraming(input);
        assert.equal(result.kind, "v2");
        assert.deepEqual(
            [...result.records].flatMap((record) =>
                record.kind === "frame" ? [record.link?.state] : [],
            ),
            expected,
        );
    }
});

test("an empty indefinite stored byte string still gets its stored check", () => {
    const input = readFileSync(
        new URL("../../../vectors/v2/P1/zstd-ok.bin", import.meta.url),
    );
    // Replace only the definite byte string holding the 275 codec bytes.
    assert.deepEqual(input.subarray(0x6e, 0x71), Buffer.of(0x59, 0x01, 0x13));
    const changed = Buffer.concat([
        input.subarray(0, 0x6e),
        Buffer.of(0x5f, 0xff),
        input.subarray(0x71 + 275),
    ]);
    const result = inspectFraming(changed);
    assert.equal(result.kind, "v2");
    const frame = [...result.records].at(-1);
    assert.equal(frame?.kind, "frame");
    assert.equal(frame.reason, "not_deterministic");
    assert.equal(frame.link?.state, "linked");
    assert.equal(frame.storedMatches, false);
    assert.deepEqual(frame.payload, { state: "damaged" });
});

test("closing claims use observed item lengths and the last typed index", () => {
    const cases = [
        ["offsets-counts-opaque", undefined, undefined],
        ["offsets-count-short", undefined, "index_offsets"],
        ["offsets-count-long", undefined, "index_offsets"],
        ["offsets-length-differs", undefined, "index_offsets"],
        ["locator-last-index.first", undefined, "index_locator"],
        ["locator-last-index.second", undefined, undefined],
        ["locator-index-type-long-form", undefined, "index_locator"],
        ["locator-short-form", "not_deterministic", undefined],
    ] as const;
    for (const [fixture, reason, claim] of cases) {
        const file = readFileSync(
            new URL(`../../../vectors/v2/P1/${fixture}.bin`, import.meta.url),
        );
        const result = inspectFraming(file);
        assert.equal(result.kind, "v2", fixture);
        const last = [...result.records].at(-1);
        assert.ok(
            last?.kind === "offsets" || last?.kind === "locator",
            fixture,
        );
        assert.equal(last.reason, reason, fixture);
        assert.equal(last.closingClaimFailure, claim, fixture);
    }
});

test("closing placement takes precedence and resets at the next opener", () => {
    const opener = Buffer.from("8701020101810181018101", "hex");
    const offsets = Buffer.of(0x82, 4, 0x81, opener.length);
    const locator = Buffer.of(0x82, 3, 0);
    const file = Buffer.concat([
        opener,
        offsets,
        locator,
        locator,
        opener,
        offsets,
    ]);
    const result = inspectFraming(file);
    assert.equal(result.kind, "v2");
    const claims = [...result.records].filter(
        (item) => item.kind === "offsets" || item.kind === "locator",
    );
    assert.deepEqual(
        claims.map((item) => item.reason),
        [
            "unordered_or_repeated",
            "unordered_or_repeated",
            "unordered_or_repeated",
            undefined,
        ],
    );
    assert.ok(
        claims.every(
            (item) =>
                item.kind !== "stop" && item.closingClaimFailure === undefined,
        ),
    );
});

test("RDF projection keeps its fold when closing grammar or claims fail", () => {
    const original = readFileSync(
        new URL(
            "../../../vectors/v2/P3/project__round-trip-rdf.bin",
            import.meta.url,
        ),
    );
    const valid = inspectFraming(original, true);
    assert.equal(valid.kind, "v2");
    const expected = rdfNQuads(projectRdf(valid));
    for (const closing of [Buffer.of(0x82, 4, 0x80), Buffer.of(0x82, 3, 0)]) {
        const damaged = inspectFraming(
            Buffer.concat([original, closing]),
            true,
        );
        assert.equal(damaged.kind, "v2");
        assert.equal(rdfNQuads(projectRdf(damaged)), expected);
    }
});

// Examined ends of framing failures (P1 section 6.3, `item_end_examined`): the repair walk holds the bytes
// through them, so each value is the offset the delimiting read reached, worked out from the rule of
// its failure site.

test("a byte string longer than the input reports the end of its head as the examined end", () => {
    // A byte string of length 5 with one byte after its head: torn, examined through the head at offset 2.
    const data = Buffer.from([0x58, 0x05, 0x01]);
    assert.throws(
        () => itemEnd(data, 0),
        (error: unknown) =>
            error instanceof FramingError &&
            error.reason === "torn_tail" &&
            error.examinedEnd === 2,
    );
});

test("a break inside a definite array reports the byte after the break as the examined end", () => {
    // An array of one item whose item is a break: undelimitable at offset 1, examined through offset 2.
    const data = Buffer.from([0x81, 0xff]);
    assert.throws(
        () => itemEnd(data, 0),
        (error: unknown) =>
            error instanceof FramingError &&
            error.reason === "undelimitable_input" &&
            error.examinedEnd === 2,
    );
});
