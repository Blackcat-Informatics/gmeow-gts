// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

import { contextHash } from "./v2_integrity.js";
import { mustEncode } from "./wire.js";

interface RangePeak {
    height: number;
    digest: Uint8Array;
}

/** The P5 Merkle range of a segment prefix, including its opener. */
export class V2Range {
    readonly #peaks: RangePeak[] = [];
    #leafCount = 0n;

    get leafCount(): bigint {
        return this.#leafCount;
    }

    root(): Uint8Array {
        return contextHash(
            "gts2 range root",
            mustEncode([
                this.#leafCount,
                this.#peaks.map((peak) => peak.digest),
            ]),
        );
    }

    push(id: Uint8Array): void {
        const peak = {
            height: 0,
            digest: contextHash("gts2 range leaf", mustEncode([id])),
        };
        let count = this.#peaks.length;
        while (this.#peaks[count - 1]?.height === peak.height) {
            const left = this.#peaks[--count];
            peak.digest = contextHash(
                "gts2 range node",
                mustEncode([left.digest, peak.digest]),
            );
            peak.height++;
        }
        this.#peaks.length = count;
        this.#peaks.push(peak);
        this.#leafCount++;
    }
}
