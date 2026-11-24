// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

import { inspectFraming } from "../v2_framing.js";

// Undefined routes confirmed v1 inputs to the existing composition ledger.
export function printFramingInfo(
    path: string,
    data: Uint8Array,
): boolean | undefined {
    const format = inspectFraming(data);
    switch (format.kind) {
        case "v1":
            return;
        case "empty":
            console.log(`${path}: empty input, 0 segments`);
            return false;
        case "unsupported":
            console.error(
                `gts info: ${path}: unsupported GTS generation ${format.generation}`,
            );
            return false;
        case "unknown":
            console.error(
                `gts info: ${path}: input does not identify a GTS generation`,
            );
            return false;
        case "framing_error":
            console.log(`${path}: framing stopped at byte 0: ${format.reason}`);
            return false;
        case "v2":
            break;
    }

    console.log(
        `${path}: GTS v2 framing ledger (byte ranges and frame integrity)`,
    );
    let clean = true;
    let segment = 0;
    for (const record of format.records) {
        if (record.kind === "stop") {
            console.log(
                `  framing stopped at byte ${record.offset}: ${record.reason}`,
            );
            return false;
        }
        const range = `bytes [${record.offset}, ${record.end})`;
        const { link, payload, storedMatches, closingClaimFailure } = record;
        const broken = link !== undefined && link.state !== "linked";
        const payloadUnverified =
            payload !== undefined && payload.state !== "verified";
        const damaged =
            record.reason !== undefined ||
            closingClaimFailure !== undefined ||
            broken ||
            storedMatches === false ||
            payloadUnverified;
        if (damaged) clean = false;
        const diagnostic =
            record.reason === undefined
                ? closingClaimFailure === undefined
                    ? ""
                    : `: index_claim_failed (${closingClaimFailure})`
                : record.frameType === 16n
                  ? `: parity_damaged (${record.reason})`
                  : `: ${record.reason}`;
        if (record.kind === "header" || record.kind === "resync") {
            const registry =
                record.registry === undefined
                    ? ""
                    : `, registry ${record.registry}, hash algorithm ${record.hash}`;
            console.log(
                `  segment ${segment++}: ${record.kind} ${range}${registry}${diagnostic}`,
            );
            if (record.registry !== undefined && record.registry !== 1n) {
                console.log(
                    `    unsupported registry version ${record.registry}`,
                );
                clean = false;
            }
            continue;
        }
        const frameType =
            record.frameType === undefined ? "" : ` type ${record.frameType}`;
        console.log(`    ${record.kind}${frameType} ${range}${diagnostic}`);
        if (link !== undefined && payload !== undefined) {
            const stored =
                storedMatches === undefined
                    ? ""
                    : storedMatches
                      ? ", stored check matches"
                      : ", stored check mismatch";
            const reason =
                payload.state === "verified" || payload.reason === undefined
                    ? ""
                    : ` (${payload.reason}${payload.count === undefined ? "" : `, ${payload.count} bytes`})`;
            console.log(
                `      link ${link.state}${stored}, payload ${payload.state}${reason}`,
            );
        }
    }
    return clean;
}
