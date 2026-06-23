// SPDX-FileCopyrightText: 2026 Blackcat Informatics(R) Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

import { decompress as zstdDecompress } from "fzstd";

import {
    Graph,
    type MetaEntry,
    TermKind,
    termKindFromWire,
    type Diagnostic,
    type LiteralDirection,
    type Quad,
    type Signature,
    type StreamableInfo,
    type Suppression,
    type Term,
    type Triple,
} from "./model.js";
import { DIGEST as STREAM_DIGEST } from "./stream.js";
import {
    BrowserCoseError,
    decrypt0WithWebCrypto,
    signatureKid,
    verifySign1WithWebCrypto,
    type BrowserKeyProvider,
} from "./browser_crypto.js";
import {
    MAGIC,
    VERSION,
    asBytes,
    asInt,
    asInt64,
    asText,
    bytesEqual,
    cborItemLength,
    concatBytes,
    contentId,
    decodeFirst,
    digestStr,
    headerId,
    isHeaderItem,
    mapGet,
    textOr,
    toBufferSource,
    tripleEqual,
    unwrapHeader,
} from "./browser_wire.js";

export { Graph, TermKind } from "./model.js";
export { toNQuads } from "./nquads.js";
export {
    BrowserCoseError,
    decrypt0WithWebCrypto,
    recipientKid,
    signatureKid,
    verifySign1WithWebCrypto,
} from "./browser_crypto.js";
export type {
    BlobEntry,
    Diagnostic,
    Graph as BrowserGraph,
    MetaEntry,
    Quad,
    Signature,
    StreamableInfo,
    Suppression,
    Term,
    Triple,
} from "./model.js";
export type {
    BrowserDecrypt0Reason,
    BrowserKeyProvider,
    BrowserSigStatus,
} from "./browser_crypto.js";

const MAX_ZSTD_DECODED_SIZE = 16 * 1024 * 1024;

type EventSink = (event: BrowserFoldEvent) => void | Promise<void>;

/** Options for browser ReadableStream folding. */
export interface BrowserReadOptions {
    /** Fold multiple concatenated GTS segments. Defaults to true. */
    allowSegments?: boolean;
    /** Expected final segment head. A mismatch records `TruncatedLog`. */
    expectedHead?: Uint8Array;
    /** Optional WebCrypto-backed keys for signatures and encrypted payloads. */
    keys?: BrowserKeyProvider;
    /** Progressive event callback fired as frames become available. */
    onEvent?: EventSink;
}

/** Final materialized result from browser stream folding. */
export interface BrowserFoldResult {
    /** Folded union graph for all accepted segments. */
    graph: Graph;
    /** Per-segment folded graphs in file order. */
    segments: Graph[];
    /** Byte offset of a torn final CBOR item, or -1 for a clean end. */
    torn: number;
}

/** Sidecar state returned by the sink-only browser stream fold. */
export interface BrowserStreamingFoldResult {
    /** Reader diagnostics in the same order as `foldStream` would report them. */
    diagnostics: Diagnostic[];
    /** Ordered per-segment head ids. */
    segmentHeads: Uint8Array[];
    /** Ordered per-segment profile names. */
    segmentProfiles: string[];
    /** Ordered per-segment metadata snapshots. */
    segmentMeta: MetaEntry[][];
    /** Ordered per-segment streamable-layout state. */
    segmentStreamable: StreamableInfo[];
    /** Byte offset of a torn final CBOR item, or -1 for a clean end. */
    torn: number;
}

/**
 * Progressive browser fold event.
 *
 * Events are emitted in CBOR item order and use segment-local term ids. They
 * expose progress before full materialization, but the browser API still
 * returns a materialized graph and should not be claimed as the full
 * `GTS Streaming Reader` tier.
 */
export type BrowserFoldEvent =
    | {
          kind: "segment-start";
          segmentIndex: number;
          frameIndex: number;
          profile: string;
          layout: string;
      }
    | {
          kind: "term";
          segmentIndex: number;
          frameIndex: number;
          termId: number;
          term: Term;
      }
    | {
          kind: "quad";
          segmentIndex: number;
          frameIndex: number;
          quad: Quad;
      }
    | {
          kind: "reifier";
          segmentIndex: number;
          frameIndex: number;
          rid: number;
          spo: Triple;
      }
    | {
          kind: "annotation";
          segmentIndex: number;
          frameIndex: number;
          annotation: Triple;
      }
    | {
          kind: "blob";
          segmentIndex: number;
          frameIndex: number;
          digest: string;
          size: number;
          described: boolean;
          meta?: unknown;
      }
    | {
          kind: "meta";
          segmentIndex: number;
          frameIndex: number;
          key: string;
          value: unknown;
      }
    | {
          kind: "suppression";
          segmentIndex: number;
          frameIndex: number;
          suppression: Suppression;
      }
    | {
          kind: "opaque";
          segmentIndex: number;
          frameIndex: number;
          frameType: string;
          reason: string;
      }
    | {
          kind: "signature";
          segmentIndex: number;
          frameIndex: number;
          signature: Signature;
      }
    | {
          kind: "diagnostic";
          segmentIndex: number;
          frameIndex?: number;
          diagnostic: Diagnostic;
      }
    | {
          kind: "streamable-layout";
          segmentIndex: number;
          frameIndex: number;
          streamable: StreamableInfo;
      }
    | {
          kind: "segment-head";
          segmentIndex: number;
          frameIndex: number;
          head: Uint8Array;
          profile: string;
      };

interface Codec {
    name: string;
    cls: string;
}

interface PayloadError {
    unavailable: boolean;
    reason: string;
    detail: string;
    damaged: boolean;
}

interface IndexRecord {
    pos: number;
    count: number;
    head: Uint8Array;
}

interface BlobEvent {
    pos: number;
    digest: string;
    described: boolean;
}

/**
 * Fold a browser `ReadableStream<Uint8Array>` and return only the final graph.
 *
 * Use `foldStream` when callers need progressive segment/frame events or the
 * torn-offset sidecar.
 */
export async function readStream(
    stream: ReadableStream<Uint8Array>,
    options: BrowserReadOptions = {},
): Promise<Graph> {
    const result = await foldStream(stream, options);
    return result.graph;
}

/**
 * Fold a browser `ReadableStream<Uint8Array>`.
 *
 * `onEvent` receives segment-local terms, quads, blobs, signatures, diagnostics,
 * and layout/head events as soon as the containing CBOR item is available. The
 * return value is still materialized graph state; this is a progressive browser
 * surface, not the Go-style `GTS Streaming Reader` tier.
 */
export async function foldStream(
    stream: ReadableStream<Uint8Array>,
    options: BrowserReadOptions = {},
): Promise<BrowserFoldResult> {
    const processor = new BrowserStreamProcessor(options, true);
    const torn = await consumeStreamItems(stream, processor);
    return processor.finish(torn);
}

/**
 * Consume a browser `ReadableStream<Uint8Array>` into a progressive event sink.
 *
 * This additive API uses the same event vocabulary and validation checks as
 * `foldStream`, but returns only reader sidecar state. Folded quads,
 * annotations, suppressions, opaque rows, signatures, and blob payloads are
 * emitted to `options.onEvent` instead of retained in a materialized `Graph`.
 */
export async function foldStreamToSink(
    stream: ReadableStream<Uint8Array>,
    options: BrowserReadOptions = {},
): Promise<BrowserStreamingFoldResult> {
    const processor = new BrowserStreamProcessor(options, false);
    const torn = await consumeStreamItems(stream, processor);
    return processor.finishToSink(torn);
}

async function consumeStreamItems(
    stream: ReadableStream<Uint8Array>,
    processor: BrowserStreamProcessor,
): Promise<number> {
    const reader = stream.getReader();
    let pending: Uint8Array = new Uint8Array();
    let consumed = 0;
    let torn = -1;

    for (;;) {
        const { value, done } = await reader.read();
        if (done) break;
        if (!value || value.length === 0) continue;
        pending = concatBytes([pending, value]);
        for (;;) {
            let length: number;
            try {
                length = cborItemLength(pending, 0);
            } catch {
                break;
            }
            const itemBytes = pending.subarray(0, length);
            let item: unknown;
            try {
                item = decodeFirst(itemBytes);
            } catch {
                torn = consumed;
                pending = new Uint8Array();
                break;
            }
            await processor.acceptItem(item);
            consumed += length;
            pending = pending.subarray(length);
        }
    }

    if (pending.length > 0 && torn < 0) torn = consumed;
    return torn;
}

class BrowserStreamProcessor {
    readonly segments: Graph[] = [];
    private current: SegmentProcessor | undefined;
    private stopped = false;
    private readonly allowSegments: boolean;
    private readonly emit: EventSink;
    private readonly keys: BrowserKeyProvider | undefined;
    private itemIndex = 0;
    private fatal: Graph | undefined;
    private pending: Promise<void>[] = [];

    constructor(
        private readonly options: BrowserReadOptions,
        private readonly materialize: boolean,
    ) {
        this.allowSegments = options.allowSegments ?? true;
        this.emit = options.onEvent ?? (() => undefined);
        this.keys = options.keys;
    }

    async acceptItem(item: unknown): Promise<void> {
        if (this.stopped) {
            this.itemIndex += 1;
            return;
        }
        if (isHeaderItem(item)) {
            await this.acceptHeader(item);
            this.itemIndex += 1;
            return;
        }
        if (!this.current) {
            const g = new Graph();
            this.pushDiagnostic(g, {
                code: "DamagedFrame",
                detail: "first item is not a header",
                frameIndex: 0,
            });
            this.fatal = g;
            this.stopped = true;
            this.itemIndex += 1;
            await this.flush();
            return;
        }
        await this.current.acceptFrame(item, this.itemIndex);
        this.itemIndex += 1;
    }

    async finish(torn: number): Promise<BrowserFoldResult> {
        if (this.current) {
            this.segments.push(await this.current.finish(this.itemIndex - 1));
            this.current = undefined;
        }

        let graph: Graph;
        if (this.fatal) {
            graph = this.fatal;
        } else if (this.segments.length === 0) {
            graph = new Graph();
            this.pushDiagnostic(graph, {
                code: "EmptyFile",
                detail: "no CBOR items",
                frameIndex: 0,
            });
        } else {
            graph =
                this.segments.length === 1
                    ? this.segments[0]
                    : unionSegments(this.segments);
        }

        const expectedHead = this.options.expectedHead;
        if (expectedHead) {
            const lastHead =
                graph.segmentHeads.length > 0
                    ? graph.segmentHeads[graph.segmentHeads.length - 1]
                    : new Uint8Array();
            if (!bytesEqual(lastHead, expectedHead)) {
                this.pushDiagnostic(graph, {
                    code: "TruncatedLog",
                    detail: "observed head does not match expected head",
                });
            }
        }
        if (torn >= 0) {
            this.pushDiagnostic(graph, {
                code: "TornAppendError",
                detail: `torn at offset ${torn}`,
            });
        }
        await this.flush();
        return { graph, segments: this.segments, torn };
    }

    private async acceptHeader(item: unknown): Promise<void> {
        if (!this.current && this.segments.length === 0) {
            this.current = new SegmentProcessor(
                item,
                this.itemIndex,
                this.segments.length,
                this.keys,
                this.emit,
                this.materialize,
            );
            await this.current.ready();
            return;
        }
        if (this.current) {
            const segment = await this.current.finish(this.itemIndex - 1);
            if (!this.allowSegments) {
                this.pushDiagnostic(segment, {
                    code: "SegmentBoundary",
                    detail: `segment boundary at item ${this.itemIndex} but reader is in pre-segment mode; remainder of file NOT folded`,
                    frameIndex: this.itemIndex,
                });
                this.segments.push(segment);
                this.current = undefined;
                this.stopped = true;
                await this.flush();
                return;
            }
            this.segments.push(segment);
        }
        this.current = new SegmentProcessor(
            item,
            this.itemIndex,
            this.segments.length,
            this.keys,
            this.emit,
            this.materialize,
        );
        await this.current.ready();
    }

    async finishToSink(torn: number): Promise<BrowserStreamingFoldResult> {
        if (this.current) {
            this.segments.push(await this.current.finish(this.itemIndex - 1));
            this.current = undefined;
        }

        let diagnostics: Diagnostic[] = [];
        let segmentHeads: Uint8Array[] = [];
        let segmentProfiles: string[] = [];
        let segmentMeta: MetaEntry[][] = [];
        let segmentStreamable: StreamableInfo[] = [];
        if (this.fatal) {
            diagnostics = [...this.fatal.diagnostics];
        } else if (this.segments.length === 0) {
            const diagnostic: Diagnostic = {
                code: "EmptyFile",
                detail: "no CBOR items",
                frameIndex: 0,
            };
            diagnostics.push(diagnostic);
            this.emitEvent({
                kind: "diagnostic",
                segmentIndex: 0,
                frameIndex: 0,
                diagnostic,
            });
        } else {
            for (const segment of this.segments) {
                diagnostics.push(...segment.diagnostics);
                segmentHeads.push(...segment.segmentHeads);
                segmentProfiles.push(...segment.segmentProfiles);
                segmentMeta.push(...segment.segmentMeta);
                segmentStreamable.push(...segment.segmentStreamable);
            }
        }

        const pushSidecarDiagnostic = (diagnostic: Diagnostic): void => {
            diagnostics.push(diagnostic);
            this.emitEvent({
                kind: "diagnostic",
                segmentIndex: segmentHeads.length,
                frameIndex: diagnostic.frameIndex,
                diagnostic,
            });
        };
        const expectedHead = this.options.expectedHead;
        if (expectedHead) {
            const lastHead =
                segmentHeads.length > 0
                    ? segmentHeads[segmentHeads.length - 1]
                    : new Uint8Array();
            if (!bytesEqual(lastHead, expectedHead)) {
                pushSidecarDiagnostic({
                    code: "TruncatedLog",
                    detail: "observed head does not match expected head",
                });
            }
        }
        if (torn >= 0) {
            pushSidecarDiagnostic({
                code: "TornAppendError",
                detail: `torn at offset ${torn}`,
            });
        }
        await this.flush();
        return {
            diagnostics,
            segmentHeads,
            segmentProfiles,
            segmentMeta,
            segmentStreamable,
            torn,
        };
    }

    private pushDiagnostic(graph: Graph, diagnostic: Diagnostic): void {
        graph.diagnostics.push(diagnostic);
        this.emitEvent({
            kind: "diagnostic",
            segmentIndex: this.segments.length,
            frameIndex: diagnostic.frameIndex,
            diagnostic,
        });
    }

    private emitEvent(event: BrowserFoldEvent): void {
        const res = this.emit(event);
        if (isPromiseLike(res)) this.pending.push(res);
    }

    private async flush(): Promise<void> {
        if (this.pending.length === 0) return;
        const pending = this.pending;
        this.pending = [];
        await Promise.all(pending);
    }
}

class SegmentProcessor {
    readonly g = new Graph();
    private header: Map<unknown, unknown> | undefined;
    private expectedPrev: Uint8Array = new Uint8Array();
    private catalog = new Map<number, Codec>();
    private folder: Folder | undefined;
    private readonly frameIds: Uint8Array[] = [];
    private pending: Promise<void>[] = [];

    constructor(
        private readonly headerItem: unknown,
        private readonly indexOffset: number,
        private readonly segmentIndex: number,
        private readonly keys: BrowserKeyProvider | undefined,
        private readonly emit: EventSink,
        private readonly materialize: boolean,
    ) {}

    async ready(): Promise<void> {
        try {
            this.header = unwrapHeader(this.headerItem);
        } catch (e) {
            this.diag(
                "DamagedFrame",
                `invalid header: ${(e as Error).message}`,
                this.indexOffset,
            );
            return this.flush();
        }
        const storedHid = asBytes(mapGet(this.header, "id"));
        if (!storedHid || !bytesEqual(storedHid, headerId(this.header))) {
            this.diag(
                "DamagedFrame",
                "header self-hash mismatch",
                this.indexOffset,
            );
        }
        const headerMagic = mapGet(this.header, "gts");
        const headerVersion = mapGet(this.header, "v");
        if (
            textOr(headerMagic, "") !== MAGIC ||
            asInt64(headerVersion) !== VERSION
        ) {
            this.diag(
                "DamagedFrame",
                `unsupported header magic/version ${String(headerMagic)}/${String(headerVersion)}`,
                this.indexOffset,
            );
        }
        this.expectedPrev = storedHid ?? new Uint8Array();
        this.catalog = catalogFrom(this.header);
        this.folder = new Folder(
            this.g,
            this.catalog,
            this.segmentIndex,
            this.keys,
            this.emitEvent.bind(this),
            this.materialize,
        );
        this.emitEvent({
            kind: "segment-start",
            segmentIndex: this.segmentIndex,
            frameIndex: this.indexOffset,
            profile: textOr(this.header.get("prof"), "generic"),
            layout: textOr(this.header.get("layout"), ""),
        });
        return this.flush();
    }

    async acceptFrame(item: unknown, absIndex: number): Promise<void> {
        const folder = this.folder;
        if (!folder) {
            this.diag(
                "DamagedFrame",
                "frame appears after invalid header",
                absIndex,
            );
            return this.flush();
        }
        const frame = item instanceof Map ? item : undefined;
        if (!frame) {
            folder.diag("DamagedFrame", "frame is not a map", absIndex);
            this.frameIds.push(new Uint8Array());
            return this.flush();
        }
        const storedId = asBytes(frame.get("id"));
        const computed = contentId(frame);
        if (!storedId || !bytesEqual(storedId, computed)) {
            folder.diag("DamagedFrame", "frame self-hash mismatch", absIndex);
            const ftype = textOr(frame.get("t"), "");
            folder.opaque(frame, ftype, "damaged", absIndex);
            this.expectedPrev = storedId ?? computed;
            this.frameIds.push(this.expectedPrev);
            return this.flush();
        }
        let prevOk = false;
        const prev = asBytes(frame.get("prev"));
        if (prev) prevOk = bytesEqual(prev, this.expectedPrev);
        if (!prevOk)
            folder.diag("BrokenChain", "prev does not match", absIndex);
        this.expectedPrev = computed;
        this.frameIds.push(this.expectedPrev);
        const sig = frame.get("sig");
        if (sig !== undefined) {
            await this.recordSignature(sig, computed, absIndex);
        }
        await folder.foldFrame(frame, absIndex);
        return this.flush();
    }

    async finish(lastFrameIndex: number): Promise<Graph> {
        if (!this.header || !this.folder) return this.g;
        this.g.segmentHeads.push(this.expectedPrev);
        this.g.segmentMeta.push([...this.g.meta]);
        this.g.segmentProfiles.push(textOr(this.header.get("prof"), "generic"));
        const streamable = layoutCheck(
            this.g,
            this.header,
            this.folder,
            this.frameIds,
            this.indexOffset,
        );
        this.g.segmentStreamable.push(streamable);
        this.emitEvent({
            kind: "streamable-layout",
            segmentIndex: this.segmentIndex,
            frameIndex: lastFrameIndex,
            streamable,
        });
        this.emitEvent({
            kind: "segment-head",
            segmentIndex: this.segmentIndex,
            frameIndex: lastFrameIndex,
            head: this.expectedPrev,
            profile: textOr(this.header.get("prof"), "generic"),
        });
        await this.flush();
        return this.g;
    }

    private async recordSignature(
        raw: unknown,
        frameId: Uint8Array,
        absIndex: number,
    ): Promise<void> {
        const sigBytes = asBytes(raw);
        let signature: Signature;
        if (!sigBytes) {
            signature = { frameId, kid: "", status: "invalid" };
        } else if (this.keys?.verificationKey) {
            const verified = await verifySign1WithWebCrypto(
                sigBytes,
                frameId,
                this.keys,
            );
            signature = {
                frameId,
                kid: verified.kid,
                status: verified.status,
                cose: sigBytes,
            };
        } else {
            signature = {
                frameId,
                kid: signatureKid(sigBytes) ?? "",
                status: "unverified",
                cose: sigBytes,
            };
        }
        if (this.materialize) this.g.signatures.push(signature);
        this.emitEvent({
            kind: "signature",
            segmentIndex: this.segmentIndex,
            frameIndex: absIndex,
            signature,
        });
    }

    private diag(code: string, detail: string, frameIndex?: number): void {
        const diagnostic: Diagnostic = { code, detail };
        if (frameIndex !== undefined) diagnostic.frameIndex = frameIndex;
        this.g.diagnostics.push(diagnostic);
        this.emitEvent({
            kind: "diagnostic",
            segmentIndex: this.segmentIndex,
            frameIndex,
            diagnostic,
        });
    }

    private emitEvent(event: BrowserFoldEvent): void {
        const res = this.emit(event);
        if (isPromiseLike(res)) this.pending.push(res);
    }

    private async flush(): Promise<void> {
        if (this.pending.length === 0) return;
        const pending = this.pending;
        this.pending = [];
        await Promise.all(pending);
    }
}

class Folder {
    indexRecords: IndexRecord[] = [];
    described = new Set<string>();
    blobEvents: BlobEvent[] = [];

    constructor(
        private readonly g: Graph,
        private readonly catalog: Map<number, Codec>,
        private readonly segmentIndex: number,
        private readonly keys: BrowserKeyProvider | undefined,
        private readonly emit: (event: BrowserFoldEvent) => void,
        private readonly materialize: boolean,
    ) {}

    diag(code: string, detail: string, index?: number): void {
        const diagnostic: Diagnostic = { code, detail };
        if (index !== undefined) diagnostic.frameIndex = index;
        this.g.diagnostics.push(diagnostic);
        this.emit({
            kind: "diagnostic",
            segmentIndex: this.segmentIndex,
            frameIndex: index,
            diagnostic,
        });
    }

    resolveCodecs(ids: unknown[]): Codec[] | PayloadError {
        const chain: Codec[] = [];
        for (const cid of ids) {
            const n = asInt64(cid);
            if (n === undefined) {
                return {
                    unavailable: true,
                    reason: "unknown-codec",
                    detail: `codec id ${String(cid)} not an integer`,
                    damaged: false,
                };
            }
            const c = this.catalog.get(n);
            if (!c) {
                return {
                    unavailable: true,
                    reason: "unknown-codec",
                    detail: `codec id ${n} not in catalog`,
                    damaged: false,
                };
            }
            chain.push(c);
        }
        return chain;
    }

    async payload(
        frame: Map<unknown, unknown>,
        isBlob: boolean,
    ): Promise<{ value: unknown | null; err?: PayloadError }> {
        const d = mapGet(frame, "d");
        const x = mapGet(frame, "x");
        if (x !== undefined) {
            if (!Array.isArray(x)) {
                return {
                    value: null,
                    err: {
                        unavailable: false,
                        reason: "",
                        damaged: true,
                        detail: "transform field 'x' must be an array",
                    },
                };
            }
            if (x.length > 0) {
                const b = asBytes(d);
                if (!b) {
                    return {
                        value: null,
                        err: {
                            unavailable: false,
                            reason: "",
                            damaged: true,
                            detail: "transformed frame 'd' must be a byte string",
                        },
                    };
                }
                const chain = this.resolveCodecs(x);
                if (isPayloadError(chain)) return { value: null, err: chain };
                let decoded: Uint8Array;
                try {
                    decoded = await decodeChain(chain, b, this.keys);
                } catch (e) {
                    return { value: null, err: payloadErrorFromCodecError(e) };
                }
                if (isBlob) return { value: decoded };
                try {
                    return { value: decodeFirst(decoded) };
                } catch (e) {
                    return {
                        value: null,
                        err: {
                            unavailable: false,
                            reason: "",
                            damaged: true,
                            detail: `payload decode failed: ${(e as Error).message}`,
                        },
                    };
                }
            }
        }
        if (d === undefined) return { value: null };
        return { value: d };
    }

    async foldFrame(
        frame: Map<unknown, unknown>,
        index: number,
    ): Promise<void> {
        const ftype = textOr(mapGet(frame, "t"), "");
        const { value: payload, err: perr } = await this.payload(
            frame,
            ftype === "blob",
        );
        if (perr) {
            if (perr.unavailable) {
                this.opaque(frame, ftype, perr.reason, index);
                this.diag(diagCodeFor(perr.reason), perr.detail, index);
            } else {
                this.opaque(frame, ftype, "damaged", index);
                this.diag(
                    "DamagedFrame",
                    `payload decode failed: ${perr.detail}`,
                    index,
                );
            }
            return;
        }
        try {
            switch (ftype) {
                case "terms":
                    this.hTerms(payload, index);
                    break;
                case "quads":
                    this.hQuads(payload, index);
                    break;
                case "reifies":
                    this.hReifies(payload, index);
                    break;
                case "annot":
                    this.hAnnot(payload, index);
                    break;
                case "blob":
                    this.hBlob(payload as Uint8Array | null, frame, index);
                    break;
                case "meta":
                    this.hMeta(payload, index);
                    break;
                case "suppress":
                    this.hSuppress(payload, index);
                    break;
                case "snapshot":
                    this.hSnapshot(payload, index);
                    break;
                case "index":
                    this.hIndex(payload, index);
                    break;
                case "opaque":
                    this.hOpaque(payload, index);
                    break;
                default:
                    this.opaque(frame, ftype, "unknown-frame-type", index);
                    this.diag(
                        "UnknownFrameType",
                        `unsupported frame type '${ftype}'`,
                        index,
                    );
            }
        } catch (e) {
            this.opaque(frame, ftype, "damaged", index);
            this.diag(
                "DamagedFrame",
                `fold failed: ${e instanceof Error ? e.message : String(e)}`,
                index,
            );
        }
    }

    hTerms(payload: unknown, index: number): void {
        const rows = Array.isArray(payload) ? payload : undefined;
        if (!rows) return;
        for (const raw of rows) {
            const entries = raw instanceof Map ? raw : undefined;
            if (!entries) continue;
            const k = asInt64(mapGet(entries, "k"));
            const resolvedKind =
                typeof k === "number" ? termKindFromWire(k) : TermKind.Iri;
            let value = "";
            const v = mapGet(entries, "v");
            if (v !== undefined) {
                const s = asText(v);
                if (s !== undefined) value = s;
            }
            let lang = "";
            const l = mapGet(entries, "l");
            if (l !== undefined) {
                const s = asText(l);
                if (s !== undefined) lang = s;
            }
            let direction: LiteralDirection | undefined;
            const dir = mapGet(entries, "dir");
            if (dir !== undefined) {
                const s = asText(dir);
                if (s === "ltr" || s === "rtl") direction = s;
            }
            const dtRaw = mapGet(entries, "dt");
            const rfRaw = mapGet(entries, "rf");
            const tid = this.g.terms.length;
            const sanitize = (r: unknown): number | undefined => {
                if (r === undefined || r === null) return undefined;
                const n = asInt64(r);
                if (n === undefined || n < 0 || n >= tid) return undefined;
                return n;
            };
            const outOfRange = (r: unknown): boolean => {
                const n = asInt64(r);
                return n !== undefined && n >= tid;
            };
            const datatype = sanitize(dtRaw);
            const reifier = sanitize(rfRaw);
            if (outOfRange(dtRaw) || outOfRange(rfRaw)) {
                this.diag(
                    "ForwardReference",
                    `term ${tid} has an out-of-range ref`,
                    index,
                );
            }
            const term: Term = {
                kind: resolvedKind,
                value,
                datatype,
                lang,
                direction,
                reifier,
            };
            this.g.terms.push(term);
            this.emit({
                kind: "term",
                segmentIndex: this.segmentIndex,
                frameIndex: index,
                termId: tid,
                term,
            });
        }
    }

    hQuads(payload: unknown, index: number): void {
        const rows = Array.isArray(payload) ? payload : undefined;
        if (!rows) return;
        for (const row of rows) {
            const items = Array.isArray(row) ? row : undefined;
            if (!items || items.length < 3) continue;
            const s = asInt(items[0]);
            const p = asInt(items[1]);
            const o = asInt(items[2]);
            let gslot: number | undefined;
            const hasGraph = items.length >= 4;
            if (hasGraph) {
                const g = asInt(items[3]);
                if (g !== undefined) gslot = g;
            }
            if (
                s === undefined ||
                p === undefined ||
                o === undefined ||
                (hasGraph && gslot === undefined)
            ) {
                this.diag(
                    "DamagedFrame",
                    "quad has non-integer term ids",
                    index,
                );
                continue;
            }
            if (!this.checkPositions(s, p, o, gslot, index)) continue;
            const quad: Quad = { s, p, o, g: gslot };
            this.emit({
                kind: "quad",
                segmentIndex: this.segmentIndex,
                frameIndex: index,
                quad,
            });
            if (this.materialize) this.g.quads.push(quad);
            if (this.g.terms[p].value === STREAM_DIGEST) {
                const obj = this.g.terms[o];
                if (obj.value !== "") this.described.add(obj.value);
            }
        }
    }

    hReifies(payload: unknown, index: number): void {
        const entries = payload instanceof Map ? payload : undefined;
        if (!entries) return;
        for (const [k, spo] of entries) {
            const rid = asInt64(k);
            if (rid === undefined) continue;
            const items = Array.isArray(spo) ? spo : undefined;
            if (!items || items.length !== 3) continue;
            const s = asInt(items[0]);
            const p = asInt(items[1]);
            const o = asInt(items[2]);
            const n = this.g.terms.length;
            const ridOk = rid >= 0 && rid < n;
            const spoOk =
                s !== undefined &&
                p !== undefined &&
                o !== undefined &&
                s < n &&
                p < n &&
                o < n;
            if (!ridOk || !spoOk) {
                this.diag(
                    "DamagedFrame",
                    `reifier ${rid} has bad/out-of-range ids`,
                    index,
                );
                continue;
            }
            const irid = rid;
            const newSpo: Triple = { s, p, o };
            const existing = this.g.reifier(irid);
            if (existing && !tripleEqual(existing, newSpo)) {
                this.diag(
                    "ConflictingReifier",
                    `reifier ${irid} rebound`,
                    index,
                );
                continue;
            }
            this.g.setReifier(irid, newSpo);
            this.emit({
                kind: "reifier",
                segmentIndex: this.segmentIndex,
                frameIndex: index,
                rid: irid,
                spo: newSpo,
            });
        }
    }

    hAnnot(payload: unknown, index: number): void {
        const rows = Array.isArray(payload) ? payload : undefined;
        if (!rows) return;
        for (const row of rows) {
            const items = Array.isArray(row) ? row : undefined;
            if (!items || items.length !== 3) continue;
            const r = asInt(items[0]);
            const p = asInt(items[1]);
            const v = asInt(items[2]);
            const n = this.g.terms.length;
            if (
                r === undefined ||
                p === undefined ||
                v === undefined ||
                r >= n ||
                p >= n ||
                v >= n
            ) {
                this.diag(
                    "DamagedFrame",
                    "annot row has bad/out-of-range ids",
                    index,
                );
                continue;
            }
            if (this.g.terms[p].kind !== TermKind.Iri) {
                this.diag(
                    "PositionConstraint",
                    `annot predicate ${p} not an IRI`,
                    index,
                );
                continue;
            }
            const annotation: Triple = { s: r, p, o: v };
            this.emit({
                kind: "annotation",
                segmentIndex: this.segmentIndex,
                frameIndex: index,
                annotation,
            });
            if (this.materialize) this.g.annotations.push(annotation);
        }
    }

    hBlob(
        payload: Uint8Array | null,
        frame: Map<unknown, unknown>,
        index: number,
    ): void {
        if (!payload) return;
        const digest = digestStr(payload);
        const pub = mapGet(frame, "pub");
        if (pub instanceof Map) this.g.setBlobMeta(digest, pub);
        const described = this.described.has(digest);
        if (this.materialize) this.g.setBlob(digest, payload);
        this.blobEvents.push({ pos: index, digest, described });
        this.emit({
            kind: "blob",
            segmentIndex: this.segmentIndex,
            frameIndex: index,
            digest,
            size: payload.length,
            described,
            meta: pub,
        });
    }

    hMeta(payload: unknown, index: number): void {
        const entries = payload instanceof Map ? payload : undefined;
        if (!entries) return;
        for (const [k, v] of entries) {
            let key = String(k);
            const s = asText(k);
            if (s !== undefined) key = s;
            this.g.setMeta(key, v);
            this.emit({
                kind: "meta",
                segmentIndex: this.segmentIndex,
                frameIndex: index,
                key,
                value: v,
            });
        }
    }

    hSuppress(payload: unknown, index: number): void {
        const entries = payload instanceof Map ? payload : undefined;
        if (!entries) return;
        const targetsRaw = mapGet(entries, "targets");
        if (!Array.isArray(targetsRaw)) return;
        const filtered: unknown[] = [];
        for (const t of targetsRaw) {
            if (t instanceof Map) filtered.push(t);
        }
        const sup: Suppression = {
            targets: filtered,
            reason: textOr(mapGet(entries, "reason"), ""),
        };
        const by = asInt(mapGet(entries, "by"));
        if (by !== undefined) sup.by = by;
        this.emit({
            kind: "suppression",
            segmentIndex: this.segmentIndex,
            frameIndex: index,
            suppression: sup,
        });
        if (this.materialize) this.g.suppressions.push(sup);
    }

    hSnapshot(payload: unknown, index: number): void {
        const entries = payload instanceof Map ? payload : undefined;
        if (!entries) return;
        const base = this.g.terms.length;
        const shift = (v: unknown): unknown => {
            const n = asInt(v);
            if (n !== undefined) return n + base;
            return v;
        };
        const shiftRow = (row: unknown): unknown => {
            const items = Array.isArray(row) ? row : undefined;
            if (!items) return row;
            return items.map((it) => shift(it));
        };

        const snapTerms = mapGet(entries, "terms");
        if (Array.isArray(snapTerms)) {
            const shifted = snapTerms.map((raw) => {
                const termMap = raw instanceof Map ? raw : undefined;
                if (!termMap) return raw;
                const newEntries = new Map<unknown, unknown>();
                for (const [k, v] of termMap) {
                    let nv = v;
                    const sk = asText(k);
                    if (sk === "dt" || sk === "rf") nv = shift(v);
                    newEntries.set(k, nv);
                }
                return newEntries;
            });
            this.hTerms(shifted, index);
        }
        const quads = mapGet(entries, "quads");
        if (Array.isArray(quads))
            this.hQuads(
                quads.map((row) => shiftRow(row)),
                index,
            );
        const reifies = mapGet(entries, "reifies");
        if (reifies instanceof Map) {
            const shifted = new Map<unknown, unknown>();
            for (const [k, v] of reifies) shifted.set(shift(k), shiftRow(v));
            this.hReifies(shifted, index);
        }
        const annot = mapGet(entries, "annot");
        if (Array.isArray(annot)) {
            this.hAnnot(
                annot.map((row) => shiftRow(row)),
                index,
            );
        }
        const blobs = mapGet(entries, "blobs");
        if (blobs instanceof Map) {
            for (const v of blobs.values()) {
                const b = asBytes(v);
                if (b && this.materialize) this.g.setBlob(digestStr(b), b);
            }
        }
        const meta = mapGet(entries, "meta");
        if (meta instanceof Map) {
            for (const [k, v] of meta) {
                let key = String(k);
                const s = asText(k);
                if (s !== undefined) key = s;
                this.g.setMeta(key, v);
            }
        }
    }

    hIndex(payload: unknown, index: number): void {
        const entries = payload instanceof Map ? payload : undefined;
        if (!entries) return;
        const count = asInt(mapGet(entries, "count"));
        const head = asBytes(mapGet(entries, "head"));
        if (count !== undefined && head)
            this.indexRecords.push({ pos: index, count, head });
    }

    hOpaque(payload: unknown, index: number): void {
        const entries = payload instanceof Map ? payload : undefined;
        if (!entries) return;
        let id: Uint8Array | undefined;
        const b = asBytes(mapGet(entries, "id"));
        if (b) id = b;
        const opaque = {
            id: id ?? new Uint8Array(),
            frameType: textOr(mapGet(entries, "type"), "opaque"),
            reason: textOr(mapGet(entries, "reason"), "unknown-codec"),
            sigStat: textOr(mapGet(entries, "sigstat"), "none"),
            pubMeta: mapGet(entries, "pub"),
            recipients: [],
        };
        if (this.materialize) this.g.opaque.push(opaque);
        this.emit({
            kind: "opaque",
            segmentIndex: this.segmentIndex,
            frameIndex: index,
            frameType: opaque.frameType,
            reason: opaque.reason,
        });
    }

    checkPositions(
        s: number,
        p: number,
        o: number,
        g: number | undefined,
        index: number,
    ): boolean {
        const n = this.g.terms.length;
        const inBounds = s < n && p < n && o < n && (g === undefined || g < n);
        if (!inBounds) {
            this.diag(
                "PositionConstraint",
                `quad (${s},${p},${o},${g === undefined ? "None" : g}) has out-of-range term ids`,
                index,
            );
            return false;
        }
        let ok = this.g.terms[p].kind === TermKind.Iri;
        if (this.g.terms[s].kind === TermKind.Literal) ok = false;
        if (g !== undefined) {
            const kind = this.g.terms[g].kind;
            if (kind === TermKind.Literal || kind === TermKind.Triple)
                ok = false;
        }
        if (!ok) {
            this.diag(
                "PositionConstraint",
                `quad (${s},${p},${o},${g === undefined ? "None" : g}) violates positions`,
                index,
            );
        }
        return ok;
    }

    opaque(
        frame: Map<unknown, unknown>,
        ftype: string,
        reason: string,
        index: number,
    ): void {
        let id: Uint8Array | undefined;
        const b = asBytes(frame.get("id"));
        if (b) id = b;
        let sigstat = "none";
        if (frame.has("sig")) sigstat = "unverified";
        const recipients: unknown[] = [];
        const to = frame.get("to");
        if (Array.isArray(to)) {
            for (const it of to) {
                if (it instanceof Map) recipients.push(it);
            }
        }
        const opaque = {
            id: id ?? new Uint8Array(),
            frameType: ftype,
            reason,
            sigStat: sigstat,
            pubMeta: frame.get("pub"),
            recipients,
        };
        if (this.materialize) this.g.opaque.push(opaque);
        this.emit({
            kind: "opaque",
            segmentIndex: this.segmentIndex,
            frameIndex: index,
            frameType: ftype,
            reason,
        });
    }
}

class BrowserCodecError extends Error {
    constructor(
        readonly reason: string,
        readonly detail: string,
        readonly failed: boolean,
    ) {
        super(detail);
        this.name = "BrowserCodecError";
    }
}

function payloadErrorFromCodecError(err: unknown): PayloadError {
    if (err instanceof BrowserCodecError) {
        return {
            unavailable: !err.failed,
            reason: err.reason,
            detail: err.detail,
            damaged: err.failed,
        };
    }
    return {
        unavailable: false,
        reason: "",
        detail: err instanceof Error ? err.message : String(err),
        damaged: true,
    };
}

function isPayloadError(x: Codec[] | PayloadError): x is PayloadError {
    return "unavailable" in x;
}

async function decodeChain(
    chain: Codec[],
    data: Uint8Array,
    keys: BrowserKeyProvider | undefined,
): Promise<Uint8Array> {
    let current = data;
    for (let i = chain.length - 1; i >= 0; i--) {
        current = await decodeOne(chain[i], current, keys);
    }
    return current;
}

async function decodeOne(
    codec: Codec,
    data: Uint8Array,
    keys: BrowserKeyProvider | undefined,
): Promise<Uint8Array> {
    if (codec.cls === "encrypt") {
        if (codec.name !== "cose-encrypt0") {
            throw new BrowserCodecError(
                "missing-key",
                `no key for encrypt codec '${codec.name}'`,
                false,
            );
        }
        if (!keys?.contentKey) {
            throw new BrowserCodecError(
                "missing-key",
                `no key for encrypt codec '${codec.name}'`,
                false,
            );
        }
        try {
            return await decrypt0WithWebCrypto(data, keys);
        } catch (e) {
            if (e instanceof BrowserCoseError) {
                if (e.reason === "missing-key") {
                    throw new BrowserCodecError(
                        "missing-key",
                        e.message,
                        false,
                    );
                }
                if (e.reason === "unsupported") {
                    throw new BrowserCodecError(
                        "unknown-codec",
                        e.message,
                        false,
                    );
                }
                throw new BrowserCodecError("damaged", e.message, true);
            }
            throw e;
        }
    }
    switch (codec.name) {
        case "identity":
            return data;
        case "gzip":
            return inflateGzip(data);
        case "zstd":
        case "zstd-rsyncable":
            try {
                const decoded = zstdDecompress(data);
                if (decoded.length > MAX_ZSTD_DECODED_SIZE) {
                    throw new BrowserCodecError(
                        "damaged",
                        "zstd decode failed: decompressed size exceeds safety bound",
                        true,
                    );
                }
                return decoded;
            } catch (e) {
                if (e instanceof BrowserCodecError) throw e;
                throw new BrowserCodecError(
                    "damaged",
                    `zstd decode failed: ${(e as Error).message}`,
                    true,
                );
            }
        default:
            throw new BrowserCodecError(
                "unknown-codec",
                `unknown codec '${codec.name}'`,
                false,
            );
    }
}

async function inflateGzip(data: Uint8Array): Promise<Uint8Array> {
    if (typeof DecompressionStream === "undefined") {
        throw new BrowserCodecError(
            "unknown-codec",
            "gzip decode requires the browser DecompressionStream API",
            false,
        );
    }
    try {
        const stream = new DecompressionStream("gzip");
        const writer = stream.writable.getWriter();
        await writer.write(toBufferSource(data));
        await writer.close();
        return readAll(stream.readable);
    } catch (e) {
        throw new BrowserCodecError(
            "damaged",
            `gzip decode failed: ${(e as Error).message}`,
            true,
        );
    }
}

async function readAll(
    stream: ReadableStream<Uint8Array>,
): Promise<Uint8Array> {
    const reader = stream.getReader();
    const chunks: Uint8Array[] = [];
    for (;;) {
        const { value, done } = await reader.read();
        if (done) break;
        if (value) chunks.push(value);
    }
    return concatBytes(chunks);
}

function diagCodeFor(reason: string): string {
    if (reason === "missing-key") return "MissingKey";
    return "UnknownCodec";
}

function catalogFrom(header: Map<unknown, unknown>): Map<number, Codec> {
    const out = new Map<number, Codec>();
    const cat = mapGet(header, "cat");
    if (!(cat instanceof Map)) return out;
    for (const [cid, entry] of cat) {
        const n = asInt64(cid);
        if (n === undefined || !(entry instanceof Map)) continue;
        out.set(n, {
            name: textOr(entry.get("name"), ""),
            cls: textOr(entry.get("cls"), "encode"),
        });
    }
    return out;
}

function layoutCheck(
    g: Graph,
    header: Map<unknown, unknown>,
    fld: Folder,
    frameIds: Uint8Array[],
    indexOffset: number,
): StreamableInfo {
    const claimed = mapGet(header, "layout") === "streamable";
    const total = frameIds.length;
    if (!claimed) return { claimed: false, covered: 0, tail: 0 };
    if (fld.indexRecords.length === 0) {
        g.diagnostics.push({
            code: "StreamableLayoutError",
            detail:
                "segment claims layout 'streamable' but carries no intact " +
                "index footer (§3.3)",
        });
        return { claimed: true, covered: 0, tail: total };
    }
    const last = fld.indexRecords[fld.indexRecords.length - 1];
    const relPos = last.pos - indexOffset;
    const tail = total - relPos;
    if (
        last.count !== relPos - 1 ||
        last.count < 1 ||
        !bytesEqual(frameIds[last.count - 1], last.head)
    ) {
        g.diagnostics.push({
            code: "StreamableLayoutError",
            detail:
                `index footer contradicts the frames it covers: count ${last.count} ` +
                "must name the frame immediately before the footer and head " +
                "must be that frame's id (§3.3)",
            frameIndex: last.pos,
        });
    }
    for (const ev of fld.blobEvents) {
        const blobRel = ev.pos - indexOffset;
        if (blobRel <= last.count && !ev.described) {
            g.diagnostics.push({
                code: "StreamableLayoutError",
                detail:
                    `covered blob ${ev.digest} delivered before its stream:digest ` +
                    "description (catalog-before-payload, §3.3)",
                frameIndex: ev.pos,
            });
        }
    }
    return { claimed: true, covered: last.count, tail, head: last.head };
}

function isPromiseLike(value: unknown): value is Promise<void> {
    return (
        typeof value === "object" &&
        value !== null &&
        "then" in value &&
        typeof (value as { then?: unknown }).then === "function"
    );
}

interface InternKey {
    typ: number;
    a: string;
    b: string;
    c: string;
    d?: string;
    seg?: number;
    rf?: number;
    bnodeTid?: number;
    bnodeLabeled?: boolean;
}

class Unioner {
    out = new Graph();
    intern = new Map<string, number>();

    keyString(k: InternKey): string {
        return JSON.stringify(k);
    }

    keyFor(seg: Graph, segIdx: number, tid: number): InternKey {
        const t = seg.terms[tid];
        switch (t.kind) {
            case TermKind.Iri:
                return { typ: 0, a: t.value, b: "", c: "" };
            case TermKind.Literal:
                return {
                    typ: 1,
                    a: t.value,
                    b: seg.datatypeIri(t),
                    c: t.lang ?? "",
                    d: t.direction ?? "",
                };
            case TermKind.Bnode:
                if (t.value !== "") {
                    return {
                        typ: 2,
                        a: t.value,
                        b: "",
                        c: "",
                        seg: segIdx,
                        bnodeLabeled: true,
                    };
                }
                return {
                    typ: 2,
                    a: "",
                    b: "",
                    c: "",
                    seg: segIdx,
                    bnodeTid: tid,
                };
            case TermKind.Triple: {
                let rf: number | undefined;
                if (t.reifier !== undefined)
                    rf = this.mapTerm(seg, segIdx, t.reifier);
                return { typ: 3, a: "", b: "", c: "", rf };
            }
        }
    }

    mapTerm(seg: Graph, segIdx: number, tid: number): number {
        const key = this.keyFor(seg, segIdx, tid);
        const ks = this.keyString(key);
        if (this.intern.has(ks)) return this.intern.get(ks)!;
        const t = seg.terms[tid];
        let datatype: number | undefined;
        if (t.datatype !== undefined)
            datatype = this.mapTerm(seg, segIdx, t.datatype);
        let reifier: number | undefined;
        if (t.reifier !== undefined)
            reifier = this.mapTerm(seg, segIdx, t.reifier);
        let value = t.value;
        if (t.kind === TermKind.Bnode) {
            if (value !== "") value = `s${segIdx}.${value}`;
            else value = `s${segIdx}._anon${this.out.terms.length}`;
        }
        this.out.terms.push({
            kind: t.kind,
            value,
            datatype,
            lang: t.lang,
            direction: t.direction,
            reifier,
        });
        const newId = this.out.terms.length - 1;
        this.intern.set(ks, newId);
        return newId;
    }

    remapSuppression(
        seg: Graph,
        segIdx: number,
        sup: Suppression,
    ): Suppression {
        const n = seg.terms.length;
        const newTargets: unknown[] = [];
        for (const target of sup.targets) {
            if (!(target instanceof Map)) {
                newTargets.push(target);
                continue;
            }
            const kind = textOr(target.get("kind"), "");
            if (kind === "frame" || kind === "blob") {
                newTargets.push(target);
                continue;
            }
            const newMap = new Map<unknown, unknown>();
            for (const [k, v] of target) {
                newMap.set(k, v);
                const key = asText(k) ?? "";
                if ((kind === "term" || kind === "reifier") && key === "id") {
                    const tid = asInt(v);
                    if (tid !== undefined && tid < n)
                        newMap.set(k, this.mapTerm(seg, segIdx, tid));
                } else if (kind === "quad" && key === "q") {
                    const ids = Array.isArray(v) ? v : undefined;
                    if (ids) {
                        newMap.set(
                            k,
                            ids.map((x) => {
                                const tid = asInt(x);
                                if (tid !== undefined && tid < n) {
                                    return this.mapTerm(seg, segIdx, tid);
                                }
                                return x;
                            }),
                        );
                    }
                }
            }
            newTargets.push(newMap);
        }
        const out: Suppression = { targets: newTargets, reason: sup.reason };
        if (sup.by !== undefined && sup.by < n)
            out.by = this.mapTerm(seg, segIdx, sup.by);
        return out;
    }
}

function unionQuadKey(q: Quad): string {
    return q.g === undefined
        ? `${q.s},${q.p},${q.o}`
        : `${q.s},${q.p},${q.o},${q.g}`;
}

function unionSegments(segments: Graph[]): Graph {
    const u = new Unioner();
    const seen = new Set<string>();
    for (let segIdx = 0; segIdx < segments.length; segIdx++) {
        const seg = segments[segIdx];
        for (const q of seg.quads) {
            const uq: Quad = {
                s: u.mapTerm(seg, segIdx, q.s),
                p: u.mapTerm(seg, segIdx, q.p),
                o: u.mapTerm(seg, segIdx, q.o),
            };
            if (q.g !== undefined) uq.g = u.mapTerm(seg, segIdx, q.g);
            const key = unionQuadKey(uq);
            if (!seen.has(key)) {
                seen.add(key);
                u.out.quads.push(uq);
            }
        }
        for (const r of seg.reifiers) {
            const newRf = u.mapTerm(seg, segIdx, r.rid);
            const spo: Triple = {
                s: u.mapTerm(seg, segIdx, r.spo.s),
                p: u.mapTerm(seg, segIdx, r.spo.p),
                o: u.mapTerm(seg, segIdx, r.spo.o),
            };
            u.out.setReifier(newRf, spo);
        }
        for (const a of seg.annotations) {
            u.out.annotations.push({
                s: u.mapTerm(seg, segIdx, a.s),
                p: u.mapTerm(seg, segIdx, a.p),
                o: u.mapTerm(seg, segIdx, a.o),
            });
        }
        for (const b of seg.blobs) u.out.setBlob(b.digest, b.data);
        for (const bm of seg.blobMeta) u.out.setBlobMeta(bm.digest, bm.meta);
        for (const m of seg.meta) u.out.setMeta(m.key, m.value);
        u.out.segmentMeta.push(...seg.segmentMeta);
        for (const sup of seg.suppressions) {
            u.out.suppressions.push(u.remapSuppression(seg, segIdx, sup));
        }
        u.out.opaque.push(...seg.opaque);
        u.out.signatures.push(...seg.signatures);
        u.out.diagnostics.push(...seg.diagnostics);
        u.out.segmentHeads.push(...seg.segmentHeads);
        u.out.segmentProfiles.push(...seg.segmentProfiles);
        u.out.segmentStreamable.push(...seg.segmentStreamable);
    }
    return u.out;
}
