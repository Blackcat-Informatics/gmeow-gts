// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import {
    ROWS,
    coefficient,
    multiply,
    shardAt,
    solveShards,
} from "../src/v2_shards.js";

const P5 = new URL("../../../vectors/v2/P5/", import.meta.url);

interface Case {
    given: { data_shards: string[]; erased: number[] };
    result: {
        exp: string;
        product_table_len: number;
        product_table_sha256: string;
        rows: number[][];
        P0: string;
        P1: string;
        solved: string[];
    };
}

function vector(name: string): Case {
    const file = JSON.parse(readFileSync(new URL(name, P5), "utf8"));
    return file.cases[0] as Case;
}

const hex = (value: string) => new Uint8Array(Buffer.from(value, "hex"));

test("the field multiplies as the recorded exponent and product tables say", () => {
    const { result } = vector("gf-tables.json");
    const exp = hex(result.exp);
    let power = 1;
    for (let exponent = 0; exponent < exp.length; exponent++) {
        assert.equal(power, exp[exponent]);
        power = multiply(power, 2);
    }
    const table = new Uint8Array(65536);
    for (let left = 0; left < 256; left++) {
        for (let right = 0; right < 256; right++) {
            table[left * 256 + right] = multiply(left, right);
        }
    }
    assert.equal(table.length, result.product_table_len);
    assert.equal(
        createHash("sha256").update(table).digest("hex"),
        result.product_table_sha256,
    );
});

test("the parity matrix rows are the Cauchy coefficients of the recorded matrix", () => {
    const { result } = vector("parity-matrix.json");
    assert.equal(result.rows.length, ROWS);
    result.rows.forEach((row: number[], index: number) => {
        assert.deepEqual(
            row,
            row.map((_, column) => coefficient(index, column)),
        );
    });
});

test("the toy geometry writes its parity shards and solves its erased data shards", () => {
    const { given, result } = vector("gf-toy.json");
    const covered = Uint8Array.from(
        Buffer.concat(given.data_shards.map((shard: string) => hex(shard))),
    );
    const size = 4;
    const empty = new Uint8Array(ROWS * size);
    const written = solveShards(
        covered,
        empty,
        size,
        new Set(),
        new Set([0, 1]),
    );
    assert.ok(written !== undefined);
    assert.deepEqual(written.parity.get(0), hex(result.P0));
    assert.deepEqual(written.parity.get(1), hex(result.P1));

    const parity = new Uint8Array(ROWS * size);
    parity.set(hex(result.P0), 0);
    parity.set(hex(result.P1), size);
    const damaged = covered.slice();
    for (const index of given.erased) {
        damaged.fill(0, index * size, (index + 1) * size);
    }
    const solved = solveShards(
        damaged,
        parity,
        size,
        new Set(given.erased),
        new Set(),
    );
    assert.ok(solved !== undefined);
    assert.deepEqual(
        given.erased.map((index: number) => solved.data.get(index)),
        result.solved.map(hex),
    );
});

test("a stripe with more unknowns than parity rows is refused", () => {
    const covered = new Uint8Array(20);
    const parity = new Uint8Array(ROWS * 4);
    assert.equal(
        solveShards(covered, parity, 4, new Set([0, 1, 2, 3, 4]), new Set()),
        undefined,
    );
});

test("the final shard is zero padded to the shard size", () => {
    assert.deepEqual(
        shardAt(Uint8Array.of(1, 2, 3, 4, 5), 1, 4),
        Uint8Array.of(5, 0, 0, 0),
    );
});
