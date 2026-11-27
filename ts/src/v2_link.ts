// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The link of a frame to its predecessor (P1 sections 3.3 and 3.4). A frame's identifier is
//! computed from its predecessor and its identity fields; the link is linked when the computed
//! identifier equals the stored one, healed when a following frame confirms the computed
//! identifier, and broken otherwise.

import { frameIdentifier } from "./v2_integrity.js";

/** Whether the link of a frame to its predecessor is intact (`LinkState` of the reader). */
export type LinkState = "linked" | "healed" | "broken";

/** The fields of a frame that the link depends on. */
export interface LinkFields {
    /** Whether the frame is transparent (it has no identifier of its own in the chain). */
    transparent: boolean;
    /** The encoded identity fields of the frame, if they can be read. */
    identity: Uint8Array | undefined;
    /** The stored identifier of the frame, if it can be read. */
    stored: Uint8Array | undefined;
}

/** The link of a frame: its state, the computed and the effective identifier. */
export interface FrameLink {
    state: LinkState;
    computedIdentifier: Uint8Array | undefined;
    effectiveIdentifier: Uint8Array | undefined;
}

function equal(left: Uint8Array, right: Uint8Array): boolean {
    return (
        left.length === right.length &&
        left.every((byte, index) => byte === right[index])
    );
}

/**
 * The link of a frame whose predecessor is `predecessor`.
 *
 * `followingConfirms` tells whether the next item of the segment is a frame whose stored
 * predecessor is the given identifier. It is consulted only for a frame whose stored
 * identifier does not match its computed one, and it is `undefined` when there is no
 * following frame to consult.
 */
export function linkOf(
    fields: LinkFields,
    predecessor: Uint8Array,
    followingConfirms: (identifier: Uint8Array) => boolean | undefined,
): FrameLink {
    const computed =
        fields.identity === undefined
            ? undefined
            : frameIdentifier(predecessor, fields.identity);
    let state: LinkState;
    if (
        computed !== undefined &&
        fields.stored !== undefined &&
        equal(computed, fields.stored)
    ) {
        state = "linked";
    } else if (
        computed !== undefined &&
        !fields.transparent &&
        fields.stored !== undefined &&
        followingConfirms(computed) === true
    ) {
        state = "healed";
    } else {
        state = "broken";
    }
    let effective: Uint8Array | undefined;
    if (fields.transparent) {
        effective = undefined;
    } else if (state === "healed") {
        effective = computed;
    } else {
        effective = fields.stored ?? computed;
    }
    return {
        state,
        computedIdentifier: computed,
        effectiveIdentifier: effective,
    };
}
