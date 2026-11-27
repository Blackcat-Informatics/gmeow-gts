// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import type { CommitReport } from "../src/v2_prefix.js";
import { readWire } from "../src/v2_segment.js";

const P5 = new URL("../../../vectors/v2/P5/", import.meta.url);

function segmentOf(name: string) {
    const read = readWire(new Uint8Array(readFileSync(new URL(name, P5))));
    assert.equal(read.stop, undefined);
    assert.equal(read.segments.length, 1);
    const segment = read.segments[0];
    assert.ok(segment !== undefined);
    return segment;
}

function report(name: string): CommitReport {
    const commits = segmentOf(name).commits;
    assert.ok(commits !== undefined && "checks" in commits);
    return commits;
}

test("a commit that states the count and the root of its prefix holds and commits the frames before it", () => {
    const commits = report("commit-valid.bin");
    assert.deepEqual(
        commits.checks.map((check) => [check.valid, check.ordinaryPosition]),
        [[true, 2n]],
    );
    assert.equal(commits.lastValidPosition, 4);
    assert.equal(commits.committedFrames, 3n);
    // The ordinary frame after the commit is uncommitted; the index after it is not counted.
    assert.deepEqual(commits.uncommittedTail, { position: 5, count: 1n });
});

test("a commit whose count or root differs from its prefix does not hold, and commits nothing", () => {
    const count = report("commit-count-mismatch.bin");
    assert.deepEqual(
        count.checks.map((check) => [check.countMatches, check.rootMatches]),
        [[false, true]],
    );
    assert.equal(count.lastValidPosition, undefined);
    assert.equal(count.committedFrames, 0n);
    const root = report("commit-root-mismatch.bin");
    assert.deepEqual(
        root.checks.map((check) => [check.countMatches, check.rootMatches]),
        [[true, false]],
    );
    assert.equal(root.committedFrames, 0n);
});

test("a segment without a commit leaves all its frames uncommitted", () => {
    const commits = report("commit-none.bin");
    assert.deepEqual(commits.checks, []);
    assert.deepEqual(commits.uncommittedTail, { position: 1, count: 2n });
});

test("an index that is wrong in one claim fails that claim alone", () => {
    for (const [name, claim] of [
        ["index-each-claim.head.bin", "headMatches"],
        ["index-each-claim.range_root.bin", "rootMatches"],
        ["index-each-claim.type_runs.bin", "typeRunsMatch"],
    ] as const) {
        const [check] = segmentOf(name).indexes;
        assert.ok(check?.outcome.kind === "checked", name);
        const claims = check.outcome.claims;
        for (const key of Object.keys(claims) as (keyof typeof claims)[]) {
            assert.equal(claims[key], key !== claim, `${name} ${key}`);
        }
    }
});
