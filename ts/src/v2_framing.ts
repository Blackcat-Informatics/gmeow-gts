// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

import { isUtf8 } from "node:buffer";

type FramingReason = "torn_tail" | "undelimitable_input";
type EnvelopeReason =
    | "wrong_type"
    | "wrong_length"
    | "not_deterministic"
    | "out_of_range"
    | "unknown_item_kind"
    | "unregistered_value"
    | "forbidden_combination"
    | "unordered_or_repeated";

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

class FramingError extends Error {
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

function head(data: Uint8Array, offset: number): Head {
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
        checkOpenerFields(
            item,
            hash.end,
            hash.value === 1n ? 32n : undefined,
            registry.value > 1n,
            resync,
        );
        return { kind, registry: registry.value, hash: hash.value };
    } catch (err) {
        if (!(err instanceof EnvelopeError)) throw err;
        return { kind, reason: err.reason };
    }
}

// Deterministic field checks operate only on complete, delimited records.
function uintField(item: Uint8Array): bigint {
    return typedHead(item, 0, 0).value;
}

function bytesField(item: Uint8Array, length?: bigint): void {
    const h = typedHead(item, 0, 2);
    if (length !== undefined && h.value !== length) {
        throw new EnvelopeError("wrong_length");
    }
}

function digestField(item: Uint8Array): void {
    bytesField(item, 32n);
}

function flagField(item: Uint8Array, expected: boolean | undefined): void {
    typedHead(item, 0, 7);
    const octet = item[0];
    if (octet !== 0xf4 && octet !== 0xf5) throw new EnvelopeError("wrong_type");
    if (expected !== undefined && expected !== (octet === 0xf5)) {
        throw new EnvelopeError("forbidden_combination");
    }
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
): void {
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
    if (!resync) return;
    uintField(next());
    checkOption(next(), digest);
    const marksLimit = bound - offset;
    checkMarks(next(), marksLimit);
    digest(next());
    if (offset > bound) throw new EnvelopeError("out_of_range");
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

function checkStorage(
    item: Uint8Array,
    newer: boolean,
    length: bigint,
    parity: boolean,
): void {
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
        bytesField(item.subarray(codec.end), 16n);
    } else if (tag.value > 2n) {
        checkGrammar(item);
    }
    if ((tag.value === 2n && length === 0n) || (parity && tag.value !== 1n)) {
        throw new EnvelopeError("forbidden_combination");
    }
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

function checkFrame(
    item: Uint8Array,
    offset: number,
    registry: bigint,
): RecordEnvelope {
    let frameType: bigint | undefined;
    try {
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
        frameType = uintField(type);
        if (frameType === 0n || (!newer && frameType > 16n)) {
            throw new EnvelopeError("unregistered_value");
        }
        const known = frameType <= 16n;
        const parity = frameType === 16n;
        flagField(critical, known ? frameType < 14n : undefined);
        flagField(transparent, known ? parity : undefined);
        checkContent(frameType, content);
        const decodedLength = uintField(length);
        digestField(digest);
        checkStorage(storage, newer, decodedLength, parity);
        bytesField(payload);
        digestField(identifier);
        checkSignatures(signatures, newer, parity);
        if (
            (frameType === 8n && decodedLength === 0n) ||
            ((frameType === 12n || frameType === 14n) && decodedLength !== 0n)
        ) {
            throw new EnvelopeError("out_of_range");
        }
        return { kind: "frame", frameType };
    } catch (err) {
        if (!(err instanceof EnvelopeError)) throw err;
        return { kind: "frame", frameType, reason: err.reason };
    }
}

function envelope(
    item: Uint8Array,
    registry: bigint | undefined,
): RecordEnvelope {
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

type InspectionEvent =
    | (RecordEnvelope & { offset: number; end: number })
    | { kind: "stop"; offset: number; reason: FramingReason };

export function inspectFraming(
    data: Uint8Array,
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
        ? { kind: "v2", records: records(data, end) }
        : format;
}

function* records(
    data: Uint8Array,
    end: number,
): IterableIterator<InspectionEvent> {
    let registry: bigint | undefined;
    for (let offset = 0; offset < data.length; offset = end) {
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
    }
}
