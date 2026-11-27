// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The encoding of parity findings as the seven-field records of P7 section 1:
//! `[code, reason, subject, position, offset, length, count]`, where an absent field is `[1]`
//! and a present one is `[2, value]`.

import type {
    ParityCode,
    ParityFinding,
    ParityReason,
} from "./v2_parity_repair.js";

const CODES: Record<ParityCode, number> = {
    exceeds_capacity: 20,
    parity_damaged: 30,
    parity_mismatch: 31,
    parity_unexplained: 32,
    parity_stale: 33,
    repair_failed: 34,
    repaired: 35,
    unprotected_tail: 36,
};

const REASONS: Record<ParityReason, number> = {
    decoded_size: 30,
    stored_size: 31,
    total_decoded: 86,
    data: 40,
    parity: 41,
    too_many_erasures: 42,
    verify_failed: 43,
};

/** The step of the read that produces a finding (P7 section 1.4). */
const STEPS: Record<ParityCode, number> = {
    exceeds_capacity: 7,
    parity_damaged: 8,
    parity_mismatch: 8,
    parity_unexplained: 8,
    parity_stale: 8,
    repair_failed: 9,
    repaired: 9,
    unprotected_tail: 10,
};

/** The head of an unsigned integer in its shortest form. */
function uint(value: bigint, out: number[]): void {
    if (value < 24n) out.push(Number(value));
    else if (value < 0x100n) out.push(0x18, Number(value));
    else if (value < 0x10000n) {
        out.push(0x19, Number(value >> 8n), Number(value & 0xffn));
    } else {
        const size = value < 0x100000000n ? 4 : 8;
        out.push(size === 4 ? 0x1a : 0x1b);
        for (let shift = size - 1; shift >= 0; shift--) {
            out.push(Number((value >> BigInt(shift * 8)) & 0xffn));
        }
    }
}

function optional(value: bigint | undefined, out: number[]): void {
    if (value === undefined) {
        out.push(0x81, 0x01);
    } else {
        out.push(0x82, 0x02);
        uint(value, out);
    }
}

function number(value: number | undefined): bigint | undefined {
    return value === undefined ? undefined : BigInt(value);
}

function encode(finding: ParityFinding): Uint8Array {
    const out: number[] = [];
    uint(BigInt(CODES[finding.code]), out);
    optional(
        finding.reason === undefined
            ? undefined
            : BigInt(REASONS[finding.reason]),
        out,
    );
    optional(undefined, out);
    optional(number(finding.position), out);
    optional(number(finding.offset), out);
    optional(number(finding.length), out);
    optional(finding.count, out);
    return Uint8Array.from(out);
}

function compareBytes(left: Uint8Array, right: Uint8Array): number {
    const common = Math.min(left.length, right.length);
    for (let index = 0; index < common; index++) {
        if (left[index] !== right[index]) return left[index] - right[index];
    }
    return left.length - right.length;
}

/**
 * The records of `findings` in the order of P7 section 1.4: the findings with a position first,
 * by position, then by step, then by encoding; the findings without one after them.
 */
export function encodeParityFindings(
    findings: readonly ParityFinding[],
): Uint8Array[] {
    return findings
        .map((finding) => ({
            positioned: finding.position !== undefined,
            position: finding.position ?? 0,
            step: STEPS[finding.code],
            record: encode(finding),
        }))
        .sort(
            (left, right) =>
                Number(!left.positioned) - Number(!right.positioned) ||
                left.position - right.position ||
                left.step - right.step ||
                compareBytes(left.record, right.record),
        )
        .map(({ record }) => record);
}
