// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

import { Buffer } from "node:buffer";
import { head, inspectFraming } from "../v2_framing.js";
import { V2Range } from "../v2_range.js";

type V2Inspection = Extract<ReturnType<typeof inspectFraming>, { kind: "v2" }>;

function verifyRecords(path: string, { records }: V2Inspection): boolean {
    let clean = true;
    let segment = -1;
    let range: V2Range | undefined;
    let committed = 0n;
    let tail = 0n;
    let indexSeen = false;
    const problem = (offset: number, reason: string): void => {
        console.error(`gts verify: ${path}: byte ${offset}: ${reason}`);
        clean = false;
    };
    const reportEnd = (): void => {
        if (range === undefined) return;
        const prefix =
            committed === 0n ? "no_commit" : `committed_frames ${committed}`;
        const uncommitted = tail === 0n ? "" : `, uncommitted_tail ${tail}`;
        const index = indexSeen ? ", index claims unchecked" : "";
        console.log(`  segment ${segment}: ${prefix}${uncommitted}${index}`);
    };

    console.log(`${path}: GTS v2 wire and commit integrity`);
    for (const record of records) {
        const { kind, offset, reason } = record;
        if (kind === "stop") {
            problem(offset, reason);
            break;
        }
        if (kind === "header" || kind === "resync") {
            reportEnd();
            segment++;
            range = undefined;
            committed = 0n;
            tail = 0n;
            indexSeen = false;
            if (reason !== undefined) problem(offset, reason);
            else if (kind === "resync")
                problem(offset, "unsupported resync opener");
            else if (record.registry !== 1n || record.openerId === undefined)
                problem(
                    offset,
                    `unsupported registry version ${record.registry}`,
                );
            else {
                range = new V2Range();
                range.push(record.openerId);
            }
            continue;
        }
        if (range === undefined) continue;
        if (reason !== undefined) problem(offset, reason);
        if (kind === "locator" || kind === "offsets") {
            problem(offset, `unsupported ${kind} claim`);
            continue;
        }
        const { link, storedMatches, payload, frameType, content } = record;
        if (kind !== "frame") {
            if (reason === undefined) problem(offset, `unchecked ${kind} item`);
            continue;
        }
        if (link?.state !== "linked")
            problem(offset, `link ${link?.state ?? "unchecked"}`);
        if (storedMatches === false) problem(offset, "stored check mismatch");
        if (payload?.state !== "verified")
            problem(
                offset,
                `payload ${payload?.reason ?? payload?.state ?? "unchecked"}`,
            );
        if (frameType === 15n) indexSeen = true;
        else if (link?.effective !== undefined) tail++;

        if (
            frameType === 14n &&
            reason === undefined &&
            content !== undefined
        ) {
            // The envelope has already checked the [count, digest] grammar.
            const count = head(content, head(content, 0).end);
            const root = content.subarray(head(content, count.end).end);
            const actualCount = range.leafCount - 1n;
            const correctCount = count.value === actualCount;
            const correctRoot = Buffer.compare(root, range.root()) === 0;
            if (!correctCount)
                problem(
                    offset,
                    `commit_count_mismatch (expected ${actualCount})`,
                );
            if (!correctRoot) problem(offset, "commit_root_mismatch");
            if (
                correctCount &&
                correctRoot &&
                (link?.state === "linked" || link?.state === "healed")
            ) {
                committed = actualCount + 1n;
                tail = 0n;
            }
        }
        if (link?.effective !== undefined) range.push(link.effective);
    }
    reportEnd();
    return clean;
}

/** Undefined routes a confirmed archived generation to its verifier. */
export function verifyV2(
    path: string,
    data: Uint8Array,
    hasKeys: boolean,
): boolean | undefined {
    const format = inspectFraming(data, true);
    if (format.kind === "v1") return;
    if (format.kind === "v2") {
        if (hasKeys) {
            console.error(
                `gts verify: ${path}: --key is unsupported for GTS v2`,
            );
            return false;
        }
        return verifyRecords(path, format);
    }
    const reason =
        format.kind === "unsupported"
            ? `unsupported GTS generation ${format.generation}`
            : format.kind === "framing_error"
              ? `framing stopped at byte 0: ${format.reason}`
              : format.kind === "empty"
                ? "empty input"
                : "input does not identify a GTS generation";
    console.error(`gts verify: ${path}: ${reason}`);
    return false;
}
