// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

import { isUtf8 } from "node:buffer";
import {
    checkLink,
    checkStored,
    contextHash,
    PayloadBudget,
    type FrameBytes,
    type FrameLink,
    type PayloadCheck,
    type Storage,
} from "./v2_integrity.js";

type FramingReason = "torn_tail" | "undelimitable_input";
type EnvelopeReason =
    | "wrong_type"
    | "wrong_length"
    | "not_deterministic"
    | "out_of_range"
    | "unknown_item_kind"
    | "unregistered_value"
    | "forbidden_combination"
    | "unordered_or_repeated"
    | "codec_constraint";

const resyncMarker = Uint8Array.of(
    0xfc,
    0xf3,
    0xeb,
    0x32,
    0xa2,
    0xf0,
    0x26,
    0x7d,
    0xd1,
    0x6a,
    0x53,
    0xf0,
    0x9b,
    0xff,
    0x87,
    0x92,
);

export class FramingError extends Error {
    constructor(readonly reason: FramingReason) {
        super(reason);
    }
}

interface Head {
    major: number;
    value: bigint;
    end: number;
    indefinite: boolean;
    shortest: boolean;
}

export function head(data: Uint8Array, offset: number): Head {
    if (offset >= data.length) throw new FramingError("torn_tail");
    const first = data[offset];
    const major = first >> 5;
    const info = first & 31;
    let end = offset + 1;
    if (info < 24) {
        return {
            major,
            value: BigInt(info),
            end,
            indefinite: false,
            shortest: true,
        };
    }
    if (info === 31 && major >= 2 && major <= 5) {
        return { major, value: 0n, end, indefinite: true, shortest: false };
    }
    if (info > 27) throw new FramingError("undelimitable_input");
    const width = 1 << (info - 24);
    if (width > data.length - end) throw new FramingError("torn_tail");
    let value = 0n;
    for (let i = 0; i < width; i++) value = (value << 8n) | BigInt(data[end++]);
    if (major === 7 && info === 24 && value < 32n) {
        throw new FramingError("undelimitable_input");
    }
    const shortest = value >= [24n, 256n, 65536n, 4294967296n][info - 24];
    return { major, value, end, indefinite: false, shortest };
}

interface Container {
    major: number;
    remaining: bigint;
    indefinite: boolean;
    needsValue: boolean;
}

// Repeat counts describe pairs for maps. Byte offsets become numbers only after
// a byte or text length fits in the remaining input.
function itemEnd(data: Uint8Array, offset: number): number {
    const pending: Container[] = [
        { major: 4, remaining: 1n, indefinite: false, needsValue: false },
    ];
    while (pending.length > 0) {
        const parent = pending[pending.length - 1];
        if (!parent.indefinite && parent.remaining === 0n) {
            pending.pop();
            continue;
        }
        if (offset === data.length) throw new FramingError("torn_tail");
        const first = data[offset];
        if (first === 0xff) {
            if (!parent.indefinite || parent.needsValue) {
                throw new FramingError("undelimitable_input");
            }
            pending.pop();
            offset++;
            continue;
        }
        if (
            parent.indefinite &&
            (parent.major === 2 || parent.major === 3) &&
            (first >> 5 !== parent.major || (first & 31) === 31)
        ) {
            throw new FramingError("undelimitable_input");
        }
        const h = head(data, offset);
        offset = h.end;
        if (parent.major === 5) parent.needsValue = !parent.needsValue;
        if (!parent.indefinite && !parent.needsValue) parent.remaining--;
        if (h.indefinite || h.major === 4 || h.major === 5 || h.major === 6) {
            pending.push({
                major: h.major,
                remaining: h.major === 6 ? 1n : h.value,
                indefinite: h.indefinite,
                needsValue: false,
            });
        } else if (h.major === 2 || h.major === 3) {
            if (h.value > BigInt(data.length - offset))
                throw new FramingError("torn_tail");
            offset += Number(h.value);
        }
    }
    return offset;
}

function isVersionKey(key: Uint8Array): boolean {
    const h = head(key, 0);
    if (h.major !== 3) return false;
    if (!h.indefinite) return h.value === 1n && key[h.end] === 0x76;
    let found = false;
    for (let offset = h.end; key[offset] !== 0xff;) {
        const chunk = head(key, offset);
        if (chunk.value !== 0n) {
            if (found || chunk.value !== 1n || key[chunk.end] !== 0x76)
                return false;
            found = true;
        }
        offset = chunk.end + Number(chunk.value);
    }
    return found;
}

function recordKind(item: Uint8Array, array: Head): Head | undefined {
    if (array.major !== 4 || (!array.indefinite && array.value === 0n)) return;
    if (item[array.end] === 0xff) return;
    const kind = head(item, array.end);
    return kind.major === 0 ? kind : undefined;
}

type OtherDispatch =
    | { kind: "v1" | "empty" | "unknown" }
    | { kind: "unsupported"; generation: bigint }
    | { kind: "framing_error"; reason: FramingReason };

function dispatch(item: Uint8Array): OtherDispatch | { kind: "v2" } {
    let top = head(item, 0);
    const kind = recordKind(item, top);
    if ((kind?.value === 1n || kind?.value === 5n) && kind.end < item.length) {
        if (item[kind.end] === 0xff) return { kind: "unknown" };
        const version = head(item, kind.end);
        if (version.major === 0) {
            if (version.value === 2n) return { kind: "v2" };
            if (version.value > 2n)
                return { kind: "unsupported", generation: version.value };
        }
    }
    if (top.major === 6 && top.value === 55799n) top = head(item, top.end);
    if (top.major === 5) {
        const limit = item.length - Number(top.indefinite);
        for (let offset = top.end; offset < limit;) {
            const valueOffset = itemEnd(item, offset);
            if (isVersionKey(item.subarray(offset, valueOffset))) {
                const version = head(item, valueOffset);
                if (version.major !== 0) break;
                if (version.value === 1n) return { kind: "v1" };
                return { kind: "unsupported", generation: version.value };
            }
            offset = itemEnd(item, valueOffset);
        }
    }
    return { kind: "unknown" };
}

type RecordKind =
    | "header"
    | "resync"
    | "frame"
    | "locator"
    | "offsets"
    | "opaque"
    | "unchecked";

interface RecordEnvelope {
    kind: RecordKind;
    reason?: EnvelopeReason;
    registry?: bigint;
    hash?: bigint;
    frameType?: bigint;
    openerId?: Uint8Array;
    link?: FrameLink;
    storedMatches?: boolean;
    closingClaimFailure?: "index_offsets" | "index_locator";
    payload?: PayloadCheck;
    content?: Uint8Array;
}

interface WireRecord extends RecordEnvelope {
    frame?: FrameBytes;
}

function typedHead(item: Uint8Array, offset: number, major: number): Head {
    const h = head(item, offset);
    if (h.major <= 4 && !h.shortest)
        throw new EnvelopeError("not_deterministic");
    if (h.major !== major) throw new EnvelopeError("wrong_type");
    return h;
}

class EnvelopeError extends Error {
    constructor(readonly reason: EnvelopeReason) {
        super(reason);
    }
}

function opener(
    item: Uint8Array,
    array: Head,
    resync: boolean,
): RecordEnvelope {
    const kind = resync ? "resync" : "header";
    try {
        if (!array.shortest) throw new EnvelopeError("not_deterministic");
        if (array.value !== (resync ? 11n : 7n))
            throw new EnvelopeError("wrong_length");
        const code = typedHead(item, array.end, 0);
        const version = typedHead(item, code.end, 0);
        if (version.value !== 2n) throw new EnvelopeError("out_of_range");
        let registryOffset = version.end;
        if (resync) {
            const marker = typedHead(item, version.end, 2);
            if (
                marker.value !== BigInt(resyncMarker.length) ||
                !resyncMarker.every(
                    (byte, index) => item[marker.end + index] === byte,
                )
            ) {
                throw new EnvelopeError("out_of_range");
            }
            registryOffset = marker.end + resyncMarker.length;
        }
        const registry = typedHead(item, registryOffset, 0);
        if (registry.value === 0n) throw new EnvelopeError("out_of_range");
        const hash = typedHead(item, registry.end, 0);
        if (hash.value === 0n || (registry.value === 1n && hash.value !== 1n)) {
            throw new EnvelopeError("unregistered_value");
        }
        const identityEnd = checkOpenerFields(
            item,
            hash.end,
            hash.value === 1n ? 32n : undefined,
            registry.value > 1n,
            resync,
        );
        const openerId =
            hash.value !== 1n
                ? undefined
                : resync
                  ? contextHash(
                        "gts2 header",
                        Uint8Array.of(0x8a),
                        item.subarray(1, identityEnd),
                    )
                  : contextHash("gts2 header", item);
        return { kind, registry: registry.value, hash: hash.value, openerId };
    } catch (err) {
        if (!(err instanceof EnvelopeError)) throw err;
        return { kind, reason: err.reason };
    }
}

// Deterministic field checks operate only on complete, delimited records.
function uintField(item: Uint8Array): bigint {
    return typedHead(item, 0, 0).value;
}

function bytesField(item: Uint8Array, length?: bigint): Uint8Array {
    const h = typedHead(item, 0, 2);
    if (length !== undefined && h.value !== length) {
        throw new EnvelopeError("wrong_length");
    }
    return item.subarray(h.end);
}

function digestField(item: Uint8Array): Uint8Array {
    return bytesField(item, 32n);
}

function flagField(item: Uint8Array): boolean {
    typedHead(item, 0, 7);
    const octet = item[0];
    if (octet !== 0xf4 && octet !== 0xf5) throw new EnvelopeError("wrong_type");
    return octet === 0xf5;
}

function* children(
    item: Uint8Array,
    offset: number,
): IterableIterator<Uint8Array> {
    while (offset < item.length) {
        const end = itemEnd(item, offset);
        yield item.subarray(offset, end);
        offset = end;
    }
}

type CheckField = (item: Uint8Array) => void;

function checkRecord(item: Uint8Array, ...fields: CheckField[]): void {
    const h = typedHead(item, 0, 4);
    if (h.value !== BigInt(fields.length))
        throw new EnvelopeError("wrong_length");
    let index = 0;
    for (const field of children(item, h.end)) fields[index++](field);
}

function checkGrammar(item: Uint8Array, limit = item.length): void {
    for (let offset = 0; offset < item.length;) {
        if (offset >= limit) throw new EnvelopeError("out_of_range");
        const h = head(item, offset);
        if (h.major <= 4 && !h.shortest)
            throw new EnvelopeError("not_deterministic");
        if (
            h.major === 5 ||
            h.major === 6 ||
            (h.major === 7 && item[offset] !== 0xf4 && item[offset] !== 0xf5)
        ) {
            throw new EnvelopeError("wrong_type");
        }
        offset = h.end;
        if (h.major === 2 || h.major === 3) {
            offset += Number(h.value);
            if (h.major === 3 && !isUtf8(item.subarray(h.end, offset))) {
                throw new EnvelopeError("wrong_type");
            }
        }
    }
}

function checkOption(item: Uint8Array, check: CheckField): void {
    const h = typedHead(item, 0, 4);
    if (h.value === 0n) throw new EnvelopeError("wrong_length");
    const tag = typedHead(item, h.end, 0);
    if (tag.value !== 1n && tag.value !== 2n)
        throw new EnvelopeError("unregistered_value");
    if (h.value !== tag.value) throw new EnvelopeError("wrong_length");
    if (tag.value === 2n) check(item.subarray(tag.end));
}

function checkMarks(item: Uint8Array, limit: number): void {
    const h = typedHead(item, 0, 4);
    let offset = h.end;
    let count = 0;
    let previous: Uint8Array | undefined;
    for (const mark of children(item, offset)) {
        if (offset >= limit) throw new EnvelopeError("out_of_range");
        const array = typedHead(mark, 0, 4);
        if (array.value === 0n || head(mark, array.end).major !== 0) {
            throw new EnvelopeError("wrong_type");
        }
        if (count === 64) throw new EnvelopeError("out_of_range");
        if (previous !== undefined && compareBytes(previous, mark) >= 0) {
            throw new EnvelopeError("unordered_or_repeated");
        }
        checkGrammar(mark, limit - offset);
        count++;
        offset += mark.length;
        previous = mark;
    }
}

function checkOpenerFields(
    item: Uint8Array,
    offset: number,
    digestLength: bigint | undefined,
    newer: boolean,
    resync: boolean,
): number {
    const fields = children(item, offset);
    const bound = resync ? 4096 : item.length;
    // The opener's preceding arity check guarantees these fields exist.
    const next = (): Uint8Array => {
        if (offset >= bound) throw new EnvelopeError("out_of_range");
        const field = fields.next().value;
        if (field === undefined) {
            throw new RangeError(
                `missing delimited opener field at byte ${offset}`,
            );
        }
        offset += field.length;
        return field;
    };
    const digest: CheckField = (field) => bytesField(field, digestLength);
    checkOption(next(), (field) => {
        if (uintField(field) === 0n)
            throw new EnvelopeError("unregistered_value");
    });
    if (!resync) {
        checkOption(next(), (field) => {
            const value = uintField(field);
            if (value === 0n || (!newer && value > 1n)) {
                throw new EnvelopeError("unregistered_value");
            }
        });
    }
    checkOption(next(), (lineage) => {
        const h = typedHead(lineage, 0, 4);
        if (h.value !== 2n) throw new EnvelopeError("wrong_length");
        const [previousHead, ordinal] = children(lineage, h.end);
        digest(previousHead);
        if (offset - ordinal.length >= bound || uintField(ordinal) === 0n) {
            throw new EnvelopeError("out_of_range");
        }
    });
    if (!resync) return offset;
    uintField(next());
    checkOption(next(), digest);
    const marksLimit = bound - offset;
    checkMarks(next(), marksLimit);
    const identityEnd = offset;
    digest(next());
    if (offset > bound) throw new EnvelopeError("out_of_range");
    return identityEnd;
}

function checkTextOption(item: Uint8Array): void {
    const h = typedHead(item, 0, 4);
    if (h.value === 0n) throw new EnvelopeError("wrong_length");
    const tag = typedHead(item, h.end, 0);
    if ((tag.value === 1n || tag.value === 2n) && h.value !== tag.value) {
        throw new EnvelopeError("wrong_length");
    }
    if (tag.value === 2n) typedHead(item, tag.end, 3);
    checkGrammar(item);
}

function checkRuns(item: Uint8Array): void {
    const h = typedHead(item, 0, 4);
    for (const run of children(item, h.end))
        checkRecord(run, uintField, uintField);
}

function checkQualifiedName(item: Uint8Array): void {
    let algorithm: bigint;
    checkRecord(
        item,
        (field) => {
            algorithm = uintField(field);
        },
        (field) => bytesField(field, algorithm === 1n ? 32n : undefined),
    );
}

function checkContent(type: bigint, item: Uint8Array): void {
    switch (type) {
        case 1n:
            return checkRecord(
                item,
                uintField,
                checkRuns,
                uintField,
                uintField,
            );
        case 2n:
        case 3n:
        case 4n:
        case 13n:
            return checkRecord(item, uintField, uintField);
        case 5n:
        case 6n:
        case 7n:
            return checkRecord(item, uintField);
        case 8n:
            return checkRecord(item, digestField);
        case 9n:
            return checkRecord(item, digestField, checkTextOption);
        case 10n:
            return checkRecord(
                item,
                digestField,
                uintField,
                checkTextOption,
                uintField,
            );
        case 11n:
            return checkRecord(item, digestField, uintField, uintField);
        case 12n:
            return checkRecord(
                item,
                checkQualifiedName,
                uintField,
                checkTextOption,
            );
        case 14n:
        case 15n:
            return checkRecord(item, uintField, digestField);
        case 16n:
            return checkRecord(
                item,
                uintField,
                uintField,
                uintField,
                digestField,
                digestField,
            );
        default:
            return checkGrammar(item);
    }
}

function storageField(item: Uint8Array, newer: boolean): Storage {
    const h = typedHead(item, 0, 4);
    if (h.value === 0n) throw new EnvelopeError("wrong_length");
    const tag = typedHead(item, h.end, 0);
    if (tag.value === 0n || (!newer && tag.value > 2n)) {
        throw new EnvelopeError("unregistered_value");
    }
    if (
        (tag.value === 1n && h.value !== 1n) ||
        (tag.value === 2n && h.value !== 3n)
    ) {
        throw new EnvelopeError("wrong_length");
    }
    if (tag.value === 2n) {
        const codec = typedHead(item, tag.end, 0);
        if (codec.value === 0n || (!newer && codec.value !== 1n)) {
            throw new EnvelopeError("unregistered_value");
        }
        return {
            tag: tag.value,
            codec: codec.value,
            encodedCodec: item.subarray(tag.end, codec.end),
            check: bytesField(item.subarray(codec.end), 16n),
        };
    } else if (tag.value > 2n) {
        checkGrammar(item);
    }
    return { tag: tag.value };
}

function compareBytes(left: Uint8Array, right: Uint8Array): number {
    const length = Math.min(left.length, right.length);
    for (let index = 0; index < length; index++) {
        if (left[index] !== right[index]) return left[index] - right[index];
    }
    return left.length - right.length;
}

function checkSignatures(
    item: Uint8Array,
    newer: boolean,
    parity: boolean,
): void {
    const h = typedHead(item, 0, 4);
    let previous: Uint8Array | undefined;
    for (const signature of children(item, h.end)) {
        checkRecord(
            signature,
            (field) => {
                const algorithm = uintField(field);
                if (algorithm === 0n || (!newer && algorithm > 2n)) {
                    throw new EnvelopeError("unregistered_value");
                }
            },
            digestField,
            bytesField,
        );
        if (previous !== undefined && compareBytes(previous, signature) >= 0) {
            throw new EnvelopeError("unordered_or_repeated");
        }
        previous = signature;
    }
    if (parity && h.value !== 0n)
        throw new EnvelopeError("forbidden_combination");
}

export function frameContentProblem(
    type: bigint,
    content: Uint8Array,
): string | undefined {
    try {
        if (itemEnd(content, 0) !== content.length) return "wrong_length";
        checkGrammar(content);
        checkContent(type, content);
    } catch (error) {
        if (!(error instanceof EnvelopeError || error instanceof FramingError))
            throw error;
        return error.reason;
    }
}

export function framePayloadLengthProblem(
    type: bigint | undefined,
    length: bigint | undefined,
): "out_of_range" | undefined {
    if (
        (type === 8n && length === 0n) ||
        ((type === 12n || type === 14n) &&
            length !== undefined &&
            length !== 0n)
    )
        return "out_of_range";
}

function readField<T>(read: () => T): { value?: T; reason?: EnvelopeReason } {
    try {
        return { value: read() };
    } catch (error) {
        if (!(error instanceof EnvelopeError)) throw error;
        return { reason: error.reason };
    }
}

// A non-shortest or indefinite byte string can still supply observed bytes for
// a link or stored check even when it fails the deterministic field grammar.
function* byteChunks(item: Uint8Array): IterableIterator<Uint8Array> {
    const h = head(item, 0);
    if (h.major !== 2) return;
    if (!h.indefinite) {
        yield item.subarray(h.end);
        return;
    }
    for (let offset = h.end; item[offset] !== 0xff;) {
        const chunk = head(item, offset);
        offset = chunk.end + Number(chunk.value);
        yield item.subarray(chunk.end, offset);
    }
}

function observedId(
    item: Uint8Array,
    typed: Uint8Array | undefined,
): Uint8Array | undefined {
    if (typed !== undefined) return typed;
    const id = new Uint8Array(32);
    let length = 0;
    for (const chunk of byteChunks(item)) {
        if (chunk.length > id.length - length) return;
        id.set(chunk, length);
        length += chunk.length;
    }
    if (length === id.length) return id;
}

function checkFrame(
    item: Uint8Array,
    offset: number,
    registry: bigint,
): WireRecord {
    const [
        type,
        critical,
        transparent,
        content,
        length,
        digest,
        storage,
        payload,
        identifier,
        signatures,
    ] = children(item, offset);
    const newer = registry > 1n;
    const kindField = readField(() => uintField(type));
    const frameType = kindField.value;
    const criticalField = readField(() => flagField(critical));
    const transparentField = readField(() => flagField(transparent));
    const contentField = readField(() =>
        checkContent(frameType ?? 0n, content),
    );
    const lengthField = readField(() => uintField(length));
    const digestBytes = readField(() => digestField(digest));
    const storageBytes = readField(() => storageField(storage, newer));
    const payloadBytes = readField(() => bytesField(payload));
    const identifierBytes = readField(() => digestField(identifier));
    const parity = frameType === 16n;
    const signatureField = readField(() =>
        checkSignatures(signatures, newer, parity),
    );

    const identity =
        (kindField.reason ??
            criticalField.reason ??
            transparentField.reason ??
            contentField.reason ??
            lengthField.reason ??
            digestBytes.reason) === undefined
            ? item.subarray(offset, storage.byteOffset - item.byteOffset)
            : undefined;
    const typeRule =
        frameType === 0n ||
        (!newer && frameType !== undefined && frameType > 16n)
            ? "unregistered_value"
            : undefined;
    const known = frameType !== undefined && frameType > 0n && frameType <= 16n;
    const criticalRule =
        known && criticalField.value !== frameType < 14n
            ? "forbidden_combination"
            : undefined;
    const transparentRule =
        known && transparentField.value !== parity
            ? "forbidden_combination"
            : undefined;
    const storageTag = storageBytes.value?.tag;
    const storageRule =
        storageTag !== undefined &&
        ((storageTag === 2n && lengthField.value === 0n) ||
            (parity && storageTag !== 1n))
            ? "forbidden_combination"
            : undefined;
    const lengthRule = framePayloadLengthProblem(frameType, lengthField.value);
    const reason =
        kindField.reason ??
        typeRule ??
        criticalField.reason ??
        criticalRule ??
        transparentField.reason ??
        transparentRule ??
        contentField.reason ??
        lengthField.reason ??
        digestBytes.reason ??
        storageBytes.reason ??
        storageRule ??
        payloadBytes.reason ??
        identifierBytes.reason ??
        signatureField.reason ??
        lengthRule;
    return {
        kind: "frame",
        frameType,
        reason,
        frame: {
            identity,
            storedId: observedId(identifier, identifierBytes.value),
            transparent: transparentField.value === true,
            decodedLength: lengthField.value,
            payloadDigest: digestBytes.value,
            storage: storageBytes.value,
            storedPayload: payloadBytes.value,
            payloadChunks:
                payload[0] >> 5 === 2 ? () => byteChunks(payload) : undefined,
            content,
            signaturesSize: signatures.length,
        },
    };
}

function envelope(item: Uint8Array, registry: bigint | undefined): WireRecord {
    const array = head(item, 0);
    const kind = recordKind(item, array);
    if (kind?.value === 1n || kind?.value === 5n)
        return opener(item, array, kind.value === 5n);
    if (registry === undefined) return { kind: "unchecked" };
    if (kind === undefined) return { kind: "opaque", reason: "wrong_type" };
    if (!array.shortest || !kind.shortest)
        return { kind: "opaque", reason: "not_deterministic" };
    if (kind.value === 2n) {
        if (array.value !== 11n)
            return { kind: "opaque", reason: "wrong_length" };
        return checkFrame(item, kind.end, registry);
    }
    if (kind.value === 3n) return { kind: "locator" };
    if (kind.value === 4n) return { kind: "offsets" };
    const reason =
        kind.value !== 0n && registry > 1n
            ? "unknown_item_kind"
            : "unregistered_value";
    return { kind: "opaque", reason };
}

interface FramingStop {
    kind: "stop";
    offset: number;
    reason: FramingReason | "item_limit";
}

interface ByteRange {
    offset: number;
    end: number;
}

type InspectionEvent = (RecordEnvelope & ByteRange) | FramingStop;

export function inspectFraming(
    data: Uint8Array,
    includeDecoded = false,
): OtherDispatch | { kind: "v2"; records: Iterable<InspectionEvent> } {
    if (data.length === 0) return { kind: "empty" };
    let end: number;
    try {
        end = itemEnd(data, 0);
    } catch (err) {
        if (!(err instanceof FramingError)) throw err;
        return { kind: "framing_error", reason: err.reason };
    }
    const format = dispatch(data.subarray(0, end));
    return format.kind === "v2"
        ? { kind: "v2", records: records(data, end, includeDecoded) }
        : format;
}

function* delimitedRecords(
    data: Uint8Array,
    end: number,
): IterableIterator<(WireRecord & ByteRange) | FramingStop> {
    let registry: bigint | undefined;
    let count = 0;
    for (let offset = 0; offset < data.length; offset = end) {
        if (count === 2 ** 26) {
            yield { kind: "stop", offset, reason: "item_limit" };
            return;
        }
        if (offset !== 0) {
            try {
                end = itemEnd(data, offset);
            } catch (err) {
                if (!(err instanceof FramingError)) throw err;
                yield { kind: "stop", offset, reason: err.reason };
                return;
            }
        }
        const record = envelope(data.subarray(offset, end), registry);
        if (record.kind === "header" || record.kind === "resync")
            registry = record.hash === 1n ? record.registry : undefined;
        yield { ...record, offset, end };
        count++;
    }
}

function offsetsHold(
    data: Uint8Array,
    start: number,
    item: ByteRange,
    body: Uint8Array,
    argument: number,
): boolean {
    const list = typedHead(body, argument, 4);
    const prefix = data.subarray(0, item.offset);
    let observed = start;
    let matches = true;
    for (const length of children(body, list.end)) {
        const claimed = uintField(length);
        if (observed === item.offset) {
            matches = false;
            continue;
        }
        const end = itemEnd(prefix, observed);
        matches = BigInt(end - observed) === claimed && matches;
        observed = end;
    }
    return matches && observed === item.offset;
}

function closingClaimHolds(
    data: Uint8Array,
    kind: "offsets" | "locator",
    item: ByteRange,
    segmentStart: number,
    lastIndex: number | undefined,
): boolean {
    const body = data.subarray(item.offset, item.end);
    const array = head(body, 0);
    if (array.value !== 2n) throw new EnvelopeError("wrong_length");
    const argument = head(body, array.end).end;
    if (kind === "offsets")
        return offsetsHold(data, segmentStart, item, body, argument);
    const distance = head(body, argument);
    if (distance.major !== 0) typedHead(body, argument, 0);
    if (body[argument] !== 0x1b) throw new EnvelopeError("not_deterministic");
    return (
        lastIndex !== undefined &&
        distance.value === BigInt(item.offset - lastIndex)
    );
}

function segmentBoundary(record: InspectionEvent | undefined): boolean {
    return (
        record === undefined ||
        record.kind === "stop" ||
        record.kind === "header" ||
        record.kind === "resync"
    );
}

function* records(
    data: Uint8Array,
    end: number,
    includeDecoded: boolean,
): IterableIterator<InspectionEvent> {
    const delimited = delimitedRecords(data, end);
    const budget = new PayloadBudget();
    let predecessor: Uint8Array | undefined;
    let segmentStart = 0;
    let lastIndex: number | undefined;
    let offsetsSeen = false;
    let locatorSeen = false;
    let current = delimited.next();
    let next = delimited.next();
    while (!current.done) {
        if (current.value.kind === "stop") {
            yield current.value;
            return;
        }
        const { frame, ...record } = current.value;
        if (record.kind === "header" || record.kind === "resync") {
            predecessor = record.openerId;
            segmentStart = record.offset;
            lastIndex = undefined;
            offsetsSeen = false;
            locatorSeen = false;
        }
        let afterNext: typeof next | undefined;
        if (record.kind === "offsets" || record.kind === "locator") {
            const following = next.done ? undefined : next.value;
            if (record.kind === "offsets" && following?.kind === "locator")
                afterNext = delimited.next();
            const final = segmentBoundary(following);
            const followedByFinalLocator =
                afterNext !== undefined &&
                segmentBoundary(afterNext.done ? undefined : afterNext.value);
            const misplaced =
                record.kind === "offsets"
                    ? offsetsSeen ||
                      locatorSeen ||
                      !(final || followedByFinalLocator)
                    : locatorSeen || !final;
            if (misplaced) {
                record.reason = "unordered_or_repeated";
            } else {
                const kind = record.kind;
                const claim = readField(() =>
                    closingClaimHolds(
                        data,
                        kind,
                        record,
                        segmentStart,
                        lastIndex,
                    ),
                );
                record.reason = claim.reason;
                if (claim.value === false)
                    record.closingClaimFailure =
                        kind === "offsets" ? "index_offsets" : "index_locator";
            }
            if (record.kind === "offsets") offsetsSeen = true;
            else locatorSeen = true;
        }
        if (record.frameType === 15n) lastIndex = record.offset;
        if (frame !== undefined && predecessor !== undefined) {
            const witness =
                !next.done && next.value.kind !== "stop"
                    ? next.value.frame
                    : undefined;
            record.link = checkLink(frame, predecessor, witness);
            predecessor = record.link.effective ?? predecessor;
            record.storedMatches = checkStored(frame);
            record.payload = budget.check(frame, includeDecoded);
            if (includeDecoded) record.content = frame.content;
            if (
                record.payload.state === "damaged" &&
                record.payload.reason === "codec_constraint"
            ) {
                record.reason ??= "codec_constraint";
            }
        }
        yield record;
        current = next;
        next = afterNext ?? delimited.next();
    }
}
