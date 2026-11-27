// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The segment reader (P1 sections 2 to 4, P5 section 1): the segments of a whole input, each
//! opened by a header or a resync item, with the links, payload checks and closing items of its
//! frames. A lost item (a framing failure, a malformed item, a frame whose link is broken, a
//! closing item that breaks its grammar, an opener that cannot be read) is recovered by the
//! next resync item that the scanner accepts (P1 section 6.7). The commit, index and parity
//! checks come in later slices. A frame whose fields do not decode is still a frame: its link
//! and payload are read from its raw fields, and it is lost.

import { FramingError, itemEnd, type FramingReason } from "./v2_framing.js";
import { HeaderError } from "./v2_decoder.js";
import {
    checkClosing,
    checkClosingGrammar,
    type ClosingFinding,
    type ClosingItem,
} from "./v2_closing.js";
import {
    asFrameError,
    readSignatures,
    type Signature,
    checkTypeLength,
    decodeEnvelope,
    FrameError,
    PARITY,
    type Frame,
} from "./v2_frame.js";
import {
    checkPrefixes,
    type CommitReadError,
    type CommitReport,
    type IndexCheck,
} from "./v2_prefix.js";
import {
    verifySignature,
    type HeldKey,
    type SignatureResult,
} from "./v2_signature.js";
import { fieldOffsets, inspectLink, malformedPayload } from "./v2_inspect.js";
import {
    EarlierParity,
    judgeParity,
    statesParityEnvelope,
    type ParityJudgement,
} from "./v2_parity.js";
import { Decoder } from "./v2_decoder.js";
import { decodeHeader, REGISTRY_VERSION, type Header } from "./v2_header.js";
import { frameIdentifier } from "./v2_integrity.js";
import { linkOf, type FrameLink } from "./v2_link.js";
import {
    DEFAULT_CAPACITY,
    PayloadBudget,
    type PayloadCheck,
    type ReadCapacity,
    storedMatches,
} from "./v2_payload.js";
import {
    decodeResync,
    RESYNC_MAX,
    ResyncScanner,
    type Candidate,
    type DecodedResync,
} from "./v2_resync.js";
import { classify, ItemWalk, type Boundary, type ItemKind } from "./v2_walk.js";

/** The opener of a segment: a header item, or a resync item that begins a segment. */
export type SegmentOpener =
    | { kind: "header"; header: Header; identifier: Uint8Array | undefined }
    | { kind: "resync"; value: DecodedResync };

/** A frame of a segment as read from the wire. */
export interface WireFrame {
    /** The position of the frame's item in its segment. */
    position: number;
    /** The type of the frame, absent when the item's type field cannot be read. */
    frameType: bigint | undefined;
    transparent: boolean;
    /** The decoded frame, or the first malformed field (including a length invalid for its type). */
    envelope: { frame: Frame } | { error: FrameError };
    /** The identifier stored in the frame, read from its raw bytes when the envelope fails. */
    stored: Uint8Array | undefined;
    link: FrameLink;
    payload: PayloadCheck;
    /** Whether the frame is of a type a later registry introduces and the fold stops at. */
    unknownTypeStopsFold: boolean;
    /** The stage 1 judgement of a parity frame in order, absent for any other frame. */
    judgement: ParityJudgement | undefined;
    /** Whether the frame is read after a resumption and its anchor is not its predecessor. */
    detached: boolean;
    /** The result of each signature of the frame, empty when the frame is not eligible. */
    signatures: SignatureResult[];
}

/** An item of a segment and the bytes it covers. */
export interface WireItem {
    kind: ItemKind["kind"] | "frame";
    offset: number;
    end: number;
}

/** A segment of the input: its opener, its items and frames, and its closing findings. */
export interface WireSegment {
    opener: SegmentOpener;
    items: WireItem[];
    frames: WireFrame[];
    /** The identifier that the next frame is linked to. */
    head: Uint8Array | undefined;
    closing: ClosingFinding[];
    /** The earlier parity frames of the segment, as condition 6 of P5 section 6.6 compares them. */
    earlier: EarlierParity;
    /** The `anchor` of the parity frame at which reading resumed, until the next ordinary frame. */
    anchor: Uint8Array | undefined;
    /** The damaged regions that a parity frame resumed the read after. */
    resumptions: FramingDamage[];
    /** The parity candidate that the total decoded length refused while recovering a lost item. */
    parityRefusal: ResyncRefusal | undefined;
    /** The commit checks of the segment, or the claim that cannot be read. */
    commits: CommitReport | CommitReadError | undefined;
    /** The checks of the index frames of the segment. */
    indexes: IndexCheck[];
    /** The offset after the last byte that belongs to the segment. */
    endOffset: number;
    /** The framing damage that ends the segment before the resync item that recovered it. */
    framingFailure: FramingDamage | undefined;
    /** The candidate that the decoded-byte budget refused while recovering a lost item. */
    resyncRefusal: ResyncRefusal | undefined;
}

/** Framing damage over a run of bytes. */
export interface FramingDamage {
    reason: FramingReason;
    offset: number;
    length: number;
}

/** A resync candidate that the total decoded length refused (P1 section 5.4). */
export interface ResyncRefusal {
    /** The position in the segment of the item that was being recovered. */
    position: number;
    offset: number;
    length: number;
    count: bigint;
}

/** Why the read stopped before the end of the input. */
export type ReadStop =
    | { kind: "framing"; reason: string; offset: number }
    | { kind: "lost_item"; offset: number }
    | { kind: "item_limit"; offset: number }
    | { kind: "unreadable_opener"; offset: number };

/** The segments of an input, in order, and the reason reading stopped, if it did. */
export interface WireRead {
    segments: WireSegment[];
    stop: ReadStop | undefined;
    /** The framing damage before the first segment's resync opener, if one recovered the read. */
    prefixFailure: FramingDamage | undefined;
    /** The candidate refused before any segment began. */
    resyncRefusal: ResyncRefusal | undefined;
}

function headerOf(opener: SegmentOpener): {
    registryVersion: bigint;
    identifier: Uint8Array | undefined;
} {
    if (opener.kind === "header") {
        return {
            registryVersion: opener.header.registryVersion,
            identifier: opener.identifier,
        };
    }
    return {
        registryVersion: opener.value.header.registryVersion,
        identifier: opener.value.identifier,
    };
}

function openerOf(
    data: Uint8Array,
    offset: number,
    end: number,
    kind: "header" | "resync",
): SegmentOpener | undefined {
    try {
        if (kind === "header") {
            const decoded = decodeHeader(data.subarray(0, end), offset);
            return {
                kind: "header",
                header: decoded.header,
                identifier: decoded.identifier,
            };
        }
        return { kind: "resync", value: decodeResync(data, offset, end) };
    } catch (error) {
        if (error instanceof HeaderError) return undefined;
        throw error;
    }
}

/**
 * Whether the next item is a frame whose stored identifier equals the identifier computed
 * from `candidate` as its predecessor (P1 section 3.4): the evidence that heals a link.
 *
 * @throws FramingError when the next frame's elements cannot be delimited.
 */
function followingConfirms(
    data: Uint8Array,
    walk: ItemWalk,
    candidate: Uint8Array,
): boolean {
    const next = walk.peek();
    if (
        next === undefined ||
        next.failure !== undefined ||
        data[next.offset] !== 0x8b ||
        data[next.offset + 1] !== 0x02
    )
        return false;
    const following = inspectLink(data, fieldOffsets(data, next.offset));
    if (following.identity === undefined || following.stored === undefined)
        return false;
    const computed = frameIdentifier(candidate, following.identity);
    return (
        computed.length === following.stored.length &&
        computed.every((byte, index) => byte === following.stored?.[index])
    );
}

/**
 * Reads the frame item at `boundary`: its envelope when it decodes, otherwise its link and
 * payload from its raw fields. `malformed` tells whether the envelope failed before the check
 * of the length against the type.
 *
 * @throws FramingError when the elements of an undecodable frame cannot be delimited.
 */
function observeFrame(
    data: Uint8Array,
    boundary: Boundary,
    position: number,
    predecessor: Uint8Array,
    registryVersion: bigint,
    walk: ItemWalk,
    budget: PayloadBudget,
    parity: { h0: Uint8Array; earlier: EarlierParity } | undefined,
): { wire: WireFrame; malformed: boolean } {
    let decoded: { frame: Frame; lengthOffset: number } | undefined;
    let failure: FrameError | undefined;
    try {
        decoded = asFrameError(() =>
            decodeEnvelope(data, boundary.offset, registryVersion, false),
        );
    } catch (error) {
        if (!(error instanceof FrameError)) throw error;
        failure = error;
    }
    let frameType: bigint | undefined;
    let fields: {
        transparent: boolean;
        identity: Uint8Array | undefined;
        stored: Uint8Array | undefined;
    };
    let offsets: readonly number[] | undefined;
    if (decoded !== undefined) {
        frameType = decoded.frame.frameType;
        fields = {
            transparent: decoded.frame.transparent,
            identity: decoded.frame.identityFields,
            stored: decoded.frame.identifier,
        };
    } else {
        offsets = fieldOffsets(data, boundary.offset);
        try {
            frameType = new Decoder(data, offsets[0]).uint();
        } catch (error) {
            if (!(error instanceof HeaderError)) throw error;
        }
        fields = inspectLink(data, offsets);
    }
    const link = linkOf(fields, predecessor, (candidate) =>
        followingConfirms(data, walk, candidate),
    );
    let payload: PayloadCheck;
    if (decoded !== undefined) {
        payload = budget.check(
            decoded.frame,
            storedMatches(decoded.frame.storage, [decoded.frame.storedPayload]),
        );
    } else {
        const raw = malformedPayload(
            data,
            offsets ?? fieldOffsets(data, boundary.offset),
            registryVersion > REGISTRY_VERSION,
        );
        payload =
            raw.fields === undefined
                ? {
                      storedMatches: raw.storedMatches,
                      state: { kind: "damaged", damage: "malformed_fields" },
                  }
                : budget.check(raw.fields, raw.storedMatches);
    }
    let envelope: WireFrame["envelope"];
    if (decoded === undefined) {
        envelope = { error: failure as FrameError };
    } else {
        try {
            envelope = {
                frame: checkTypeLength(decoded.frame, decoded.lengthOffset),
            };
        } catch (error) {
            if (!(error instanceof FrameError)) throw error;
            envelope = { error };
        }
    }
    const newer = registryVersion > REGISTRY_VERSION;
    const unknownTypeStopsFold =
        newer &&
        frameType !== undefined &&
        frameType > PARITY &&
        (decoded !== undefined
            ? decoded.frame.critical
            : data[(offsets ?? [])[1]] === 0xf5 &&
              (data[(offsets ?? [])[2]] === 0xf4 ||
                  data[(offsets ?? [])[2]] === 0xf5));
    const judgement =
        frameType === PARITY && parity !== undefined
            ? judgeParity(
                  {
                      envelope: decoded?.frame,
                      link: link.state,
                      predecessor,
                      payload: payload.state,
                      h0: parity.h0,
                      earlier: parity.earlier,
                  },
                  newer,
              )
            : undefined;
    return {
        wire: {
            position,
            unknownTypeStopsFold,
            judgement,
            signatures: [],
            detached: false,
            frameType,
            transparent: fields.transparent,
            envelope,
            stored: fields.stored,
            link,
            payload,
        },
        malformed: decoded === undefined,
    };
}

function bytesEqual(left: Uint8Array, right: Uint8Array): boolean {
    return (
        left.length === right.length &&
        left.every((byte, index) => byte === right[index])
    );
}

/** The identifier of the segment's opener, absent when the opener has none. */
function identifierOf(segment: WireSegment): Uint8Array | undefined {
    return headerOf(segment.opener).identifier;
}

/**
 * Records a parity frame for condition 6 of P5 section 6.6 when its link is not broken. The
 * frame is read from its raw item, so a frame whose envelope fails elsewhere still counts.
 */
function rememberParity(
    segment: WireSegment,
    data: Uint8Array,
    boundary: Boundary,
    frame: WireFrame,
    registryVersion: bigint,
): void {
    if (frame.frameType !== PARITY || frame.link.state === "broken") return;
    let offsets: readonly number[];
    try {
        offsets = fieldOffsets(data.subarray(0, boundary.end), boundary.offset);
    } catch (error) {
        if (error instanceof FramingError) return;
        throw error;
    }
    segment.earlier.remember(
        data.subarray(offsets[3], offsets[4]),
        registryVersion > REGISTRY_VERSION,
        statesParityEnvelope(data, offsets),
    );
}

function emptySegment(opener: SegmentOpener): WireSegment {
    const { identifier } = headerOf(opener);
    return {
        opener,
        items: [],
        frames: [],
        head: identifier,
        closing: [],
        earlier: new EarlierParity(),
        anchor: undefined,
        resumptions: [],
        parityRefusal: undefined,
        commits: undefined,
        indexes: [],
        endOffset: 0,
        framingFailure: undefined,
        resyncRefusal: undefined,
    };
}

/**
 * The signature records of a frame item and the encoded size of its `signatures` element,
 * or absent when the element is outside the grammar of P6 section 1.
 */
function signatureRecords(
    data: Uint8Array,
    item: WireItem,
    frame: WireFrame,
    newer: boolean,
): { records: readonly Signature[]; size: number } | undefined {
    if ("frame" in frame.envelope) {
        return {
            records: frame.envelope.frame.signatures,
            size: frame.envelope.frame.encodedSignaturesSize,
        };
    }
    const itemData = data.subarray(0, item.end);
    try {
        const offsets = fieldOffsets(itemData, item.offset);
        const records = readSignatures(
            new Decoder(itemData, offsets[9]),
            newer,
        );
        return { records, size: offsets[10] - offsets[9] };
    } catch (error) {
        if (error instanceof HeaderError || error instanceof FramingError) {
            return undefined;
        }
        throw error;
    }
}

/**
 * Verifies each signature of each eligible frame against the opener's identifier `h0` (P6
 * section 6.2). A frame is eligible when it is ordinary, has an effective identifier, has a
 * `signatures` element in grammar and that element is within `storedLimit`; its other fields
 * do not matter.
 */
function verifySignatures(
    segment: WireSegment,
    data: Uint8Array,
    h0: Uint8Array,
    storedLimit: bigint,
    keys: readonly HeldKey[],
    newer: boolean,
): void {
    for (const frame of segment.frames) {
        const identifier = frame.transparent
            ? undefined
            : frame.link.effectiveIdentifier;
        if (identifier === undefined) continue;
        const found = signatureRecords(
            data,
            segment.items[frame.position],
            frame,
            newer,
        );
        if (found === undefined || BigInt(found.size) > storedLimit) continue;
        frame.signatures = found.records.map((record) =>
            verifySignature(record, h0, identifier, keys),
        );
    }
}

function finish(
    segment: WireSegment,
    data: Uint8Array,
    capacity: ReadCapacity,
    keys: readonly HeldKey[],
): void {
    const { registryVersion, identifier } = headerOf(segment.opener);
    if (identifier === undefined) return;
    verifySignatures(
        segment,
        data,
        identifier,
        capacity.storedBytesPerFrame,
        keys,
        registryVersion > REGISTRY_VERSION,
    );
    const items: ClosingItem[] = segment.items.map((item) => ({
        kind:
            item.kind === "offsets" || item.kind === "locator"
                ? item.kind
                : "other",
        offset: item.offset,
        end: item.end,
    }));
    const frames = segment.frames.map((frame) => ({
        position: frame.position,
        frameType: frame.frameType,
    }));
    segment.closing = checkClosing(data, items, frames);
    const checked = checkPrefixes(
        identifier,
        segment.frames,
        capacity.storedBytesPerFrame,
        registryVersion,
    );
    if (checked.ok) {
        segment.commits = checked.report;
        segment.indexes = checked.indexes;
    } else {
        segment.commits = checked.error;
    }
}

function damageBefore(
    data: Uint8Array,
    start: number,
    end: number,
): FramingDamage | undefined {
    if (start >= end) return undefined;
    try {
        itemEnd(data.subarray(0, end), start);
    } catch (error) {
        if (!(error instanceof FramingError)) throw error;
        return { reason: error.reason, offset: start, length: end - start };
    }
    return undefined;
}

function recoveredSegment(
    offset: number,
    candidate: Extract<Candidate, { kind: "accepted" }>,
): WireSegment {
    const segment = emptySegment({ kind: "resync", value: candidate.value });
    segment.items.push({
        kind: "resync",
        offset,
        end: candidate.value.nextOffset,
    });
    segment.endOffset = candidate.value.nextOffset;
    return segment;
}

/** Whether the last frame of `segment` is the last item and its link was healed. */
function healedLast(segment: WireSegment): WireFrame | undefined {
    const last = segment.frames.at(-1);
    return last !== undefined &&
        last.position + 1 === segment.items.length &&
        last.link.state === "healed"
        ? last
        : undefined;
}

/**
 * Ends `segment` at the accepted resync item at `offset`. A healed link of the last frame was
 * confirmed by the item the resync item replaces, so it is broken again and keeps its stored
 * identifier. Damage before the resync item is reported on the segment.
 */
function endAtResync(
    segment: WireSegment,
    data: Uint8Array,
    lost: Boundary,
    offset: number,
): void {
    const healed = healedLast(segment);
    if (healed !== undefined) {
        const stored = healed.stored;
        healed.link = {
            ...healed.link,
            state: "broken",
            effectiveIdentifier: stored,
        };
        segment.head = stored;
    }
    segment.endOffset = offset;
    segment.framingFailure = damageBefore(data, lost.offset, offset);
    const failure = segment.framingFailure;
    if (failure === undefined) return;
    if (failure.reason === "torn_tail") {
        segment.endOffset = failure.offset;
    } else {
        segment.items.push({
            kind: "damaged_region",
            offset: failure.offset,
            end: offset,
        });
    }
}

/**
 * Recovers from a lost item by scanning for the next resync item or parity candidate. An accepted
 * resync item ends `segment` and is returned. A passing parity candidate keeps `segment`: the bytes
 * before it become one damaged region, and its anchor becomes the predecessor of the next ordinary
 * frame. A refused candidate is recorded on `segment` and the read goes on without recovery.
 */
function recoverLostItem(
    segment: WireSegment,
    walk: ItemWalk,
    scanner: ResyncScanner,
    budget: PayloadBudget,
    lost: Boundary,
    data: Uint8Array,
): Extract<Candidate, { kind: "accepted" }> | { kind: "parity" } | undefined {
    const last = segment.items.at(-1);
    const start =
        healedLast(segment) !== undefined || last === undefined
            ? lost.offset
            : last.offset + 1;
    const identifier = identifierOf(segment);
    const parity =
        lost.failure !== undefined && identifier !== undefined
            ? {
                  h0: identifier,
                  registry: headerOf(segment.opener).registryVersion,
                  earlier: segment.earlier,
              }
            : undefined;
    const candidate = walk.recover(scanner, start, lost, budget, parity);
    if (candidate?.kind === "accepted") {
        endAtResync(segment, data, lost, candidate.offset);
        return candidate;
    }
    if (candidate?.kind === "parity") {
        segment.resumptions.push({
            reason: "undelimitable_input",
            offset: lost.offset,
            length: candidate.offset - lost.offset,
        });
        segment.items.push({
            kind: "damaged_region",
            offset: lost.offset,
            end: candidate.offset,
        });
        segment.endOffset = candidate.offset;
        segment.anchor = candidate.anchor;
        return { kind: "parity" };
    }
    if (candidate?.kind === "parity_refused") {
        segment.parityRefusal = {
            position: segment.items.length,
            offset: candidate.offset,
            length: candidate.length,
            count: candidate.count,
        };
        return undefined;
    }
    if (candidate?.kind === "refused") {
        segment.resyncRefusal = {
            position:
                segment.items.length - (candidate.offset < lost.offset ? 1 : 0),
            offset: candidate.offset,
            length: RESYNC_MAX,
            count: candidate.count,
        };
    }
    return undefined;
}

/**
 * Reads the segments of `data` in order. Each frame is linked to the identifier before it,
 * its payload is checked against `capacity`, and the closing items are checked when a segment
 * ends. A lost item is recovered at the next accepted resync item, which begins a segment;
 * otherwise the read stops at the first item it cannot yet read, with that item's offset.
 */
export function readWire(
    data: Uint8Array,
    capacity: ReadCapacity = DEFAULT_CAPACITY,
    keys: readonly HeldKey[] = [],
): WireRead {
    const result: WireRead = {
        segments: [],
        stop: undefined,
        prefixFailure: undefined,
        resyncRefusal: undefined,
    };
    const walk = new ItemWalk(data, Number(capacity.itemsPerRead));
    const scanner = new ResyncScanner(data);
    const budget = new PayloadBudget(capacity);
    const first = walk.take();
    if (first === undefined) return result;
    let segment: WireSegment;
    const firstKind =
        first.failure === undefined
            ? classify(data, first.offset, false).kind
            : undefined;
    const opener =
        firstKind === "header" || firstKind === "resync"
            ? openerOf(data, first.offset, first.end, firstKind)
            : undefined;
    if (
        opener !== undefined &&
        (firstKind === "header" || firstKind === "resync")
    ) {
        segment = emptySegment(opener);
        segment.items.push({
            kind: firstKind,
            offset: first.offset,
            end: first.end,
        });
        segment.endOffset = first.end;
    } else {
        const candidate = walk.recover(scanner, 0, first, budget);
        if (candidate?.kind !== "accepted") {
            if (candidate?.kind === "refused") {
                result.resyncRefusal = {
                    position: 0,
                    offset: candidate.offset,
                    length: RESYNC_MAX,
                    count: candidate.count,
                };
            }
            result.stop =
                first.failure !== undefined
                    ? {
                          kind: "framing",
                          reason: first.failure,
                          offset: first.offset,
                      }
                    : firstKind === "header" || firstKind === "resync"
                      ? { kind: "unreadable_opener", offset: first.offset }
                      : { kind: "lost_item", offset: first.offset };
            return result;
        }
        result.prefixFailure = damageBefore(data, 0, candidate.offset);
        segment = recoveredSegment(candidate.offset, candidate);
    }
    for (let next = walk.take(); next !== undefined; next = walk.take()) {
        const { registryVersion } = headerOf(segment.opener);
        const blind = segment.head === undefined;
        const kind: ItemKind =
            next.failure === undefined
                ? classify(
                      data,
                      next.offset,
                      blind || registryVersion > REGISTRY_VERSION,
                  )
                : { kind: "unchecked" };
        let lost = next.failure !== undefined || kind.kind === "malformed";
        let begun: SegmentOpener | undefined;
        let frame: WireFrame | undefined;
        if (next.failure === undefined) {
            if (kind.kind === "header" || kind.kind === "resync") {
                begun = openerOf(data, next.offset, next.end, kind.kind);
                lost = begun === undefined;
            } else if (kind.kind === "frame" && !blind) {
                try {
                    const observed = observeFrame(
                        data,
                        next,
                        segment.items.length,
                        segment.anchor ?? segment.head ?? new Uint8Array(32),
                        registryVersion,
                        walk,
                        budget,
                        identifierOf(segment) === undefined
                            ? undefined
                            : {
                                  h0: identifierOf(segment) as Uint8Array,
                                  earlier: segment.earlier,
                              },
                    );
                    frame = observed.wire;
                    frame.detached =
                        !frame.transparent &&
                        segment.anchor !== undefined &&
                        (segment.head === undefined ||
                            !bytesEqual(segment.anchor, segment.head));
                    lost = observed.malformed || frame.link.state === "broken";
                } catch (error) {
                    if (!(error instanceof FramingError)) throw error;
                    result.stop = {
                        kind: "framing",
                        reason: error.reason,
                        offset: next.offset,
                    };
                    break;
                }
            } else if (
                (kind.kind === "locator" || kind.kind === "offsets") &&
                !blind
            ) {
                try {
                    checkClosingGrammar(data, {
                        kind: kind.kind,
                        offset: next.offset,
                        end: next.end,
                    });
                } catch (error) {
                    if (!(error instanceof HeaderError)) throw error;
                    lost = true;
                }
            }
        }
        if (lost) {
            const recovered = recoverLostItem(
                segment,
                walk,
                scanner,
                budget,
                next,
                data,
            );
            if (recovered?.kind === "accepted") {
                finish(segment, data, capacity, keys);
                result.segments.push(segment);
                segment = recoveredSegment(recovered.offset, recovered);
                continue;
            }
            if (recovered?.kind === "parity") continue;
        }
        if (next.failure !== undefined) {
            result.stop = {
                kind: "framing",
                reason: next.failure,
                offset: next.offset,
            };
            break;
        }
        if (kind.kind === "header" || kind.kind === "resync") {
            if (begun === undefined) {
                result.stop = {
                    kind: "unreadable_opener",
                    offset: next.offset,
                };
                break;
            }
            segment.endOffset = next.offset;
            finish(segment, data, capacity, keys);
            result.segments.push(segment);
            segment = emptySegment(begun);
            segment.items.push({
                kind: kind.kind,
                offset: next.offset,
                end: next.end,
            });
            segment.endOffset = next.end;
            continue;
        }
        if (frame !== undefined) {
            if (frame.link.effectiveIdentifier !== undefined) {
                segment.head = frame.link.effectiveIdentifier;
                segment.anchor = undefined;
            }
            rememberParity(segment, data, next, frame, registryVersion);
            segment.frames.push(frame);
        }
        segment.items.push({
            kind: blind ? "unchecked" : kind.kind,
            offset: next.offset,
            end: next.end,
        });
        segment.endOffset = next.end;
    }
    result.stop ??=
        walk.limitOffset === undefined
            ? undefined
            : { kind: "item_limit", offset: walk.limitOffset };
    finish(segment, data, capacity, keys);
    result.segments.push(segment);
    return result;
}
