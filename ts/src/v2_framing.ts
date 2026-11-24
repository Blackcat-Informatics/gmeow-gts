// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

type FramingReason = "torn_tail" | "undelimitable_input";
type EnvelopeReason =
    | "wrong_type"
    | "wrong_length"
    | "not_deterministic"
    | "out_of_range"
    | "unknown_item_kind"
    | "unregistered_value";

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
        return { kind, registry: registry.value, hash: hash.value };
    } catch (err) {
        if (!(err instanceof EnvelopeError)) throw err;
        return { kind, reason: err.reason };
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
        try {
            const frameType = typedHead(item, kind.end, 0).value;
            const reason =
                frameType === 0n || (registry === 1n && frameType > 16n)
                    ? "unregistered_value"
                    : undefined;
            return { kind: "frame", frameType, reason };
        } catch (err) {
            if (!(err instanceof EnvelopeError)) throw err;
            return { kind: "frame", reason: err.reason };
        }
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
            registry = record.registry;
        yield { ...record, offset, end };
    }
}
