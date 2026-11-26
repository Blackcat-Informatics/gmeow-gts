// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The typed result of reading a v2 file (P7 section 1.1): the records of
//! `read_result`, their deterministic encoding and their decoding.

import { decodeFirst, encodeCanonical } from "./browser_wire.js";

/** A value of the deterministic CBOR data model that the result records use. */
type Tree = bigint | boolean | Uint8Array | readonly Tree[];

/** The enumerations of P2 "Enumerations of the typed result", by code order. */
export const READERS_CLAIM = ["second_only", "first_and_second"] as const;
export const ACCESS = ["whole", "forward", "sparse"] as const;
export const LINK_STATE = ["linked", "healed", "broken"] as const;
export const STORED_STATE = ["matches", "mismatch"] as const;
export const INTEGRITY_STATE = [
    "intact",
    "intact_unchecked",
    "damaged",
    "not_checked",
] as const;
export const AUTHENTICITY_STATE = [
    "authenticated",
    "partly_authenticated",
    "unauthenticated",
] as const;
export const SIGNATURE_STATUS = ["valid", "invalid", "unverified"] as const;
export const UNCHECKED_REASON = [
    "unknown_codec",
    "exceeds_capacity",
    "labelling_refused",
    "encoder_not_held",
    "not_claimed",
    "access",
    "read_failed",
] as const;
export const OPAQUE_REASON = [
    "damaged",
    "malformed",
    "unknown_frame_type",
    "unsupported_critical",
    "stopped",
    "chain_break",
    "exceeds_capacity",
    "unknown_codec",
] as const;
export const CANONICAL_CHECK = [
    "header",
    "frame_sequence",
    "terms",
    "rows",
    "value_region",
    "blob_region",
    "bindings",
    "suppression",
    "commit",
    "index",
    "labelling",
    "content_digest",
] as const;
export const RESYNC_REACH = ["walked", "scanned"] as const;
export const PAST_STATE = [
    "absent",
    "matches",
    "differs",
    "not_checked",
] as const;
export const FILE_END = [
    "at_authenticated_commit",
    "at_valid_commit",
    "after_commit",
    "no_commit",
    "torn",
    "undelimitable",
    "not_read",
] as const;

export type Access = (typeof ACCESS)[number];
export type LinkState = (typeof LINK_STATE)[number];
export type StoredState = (typeof STORED_STATE)[number];
export type IntegrityState = (typeof INTEGRITY_STATE)[number];
export type AuthenticityState = (typeof AUTHENTICITY_STATE)[number];
export type SignatureStatus = (typeof SIGNATURE_STATUS)[number];
export type UncheckedReason = (typeof UNCHECKED_REASON)[number];
export type OpaqueReason = (typeof OPAQUE_REASON)[number];
export type CanonicalCheck = (typeof CANONICAL_CHECK)[number];
export type ResyncReach = (typeof RESYNC_REACH)[number];
export type PastState = (typeof PAST_STATE)[number];
export type FileEnd = (typeof FILE_END)[number];

/**
 * The finding codes of P2 section 5.1 in code order, each with its severity. A
 * finding's severity is the fixed column of its code, never a field.
 */
const FINDING_TABLE = {
    empty_input: "fatal",
    not_this_format: "fatal",
    unsupported_generation: "fatal",
    unsupported_registry_version: "error",
    unknown_enumeration_value: "warning",
    unsupported_critical: "error",
    malformed_value: "error",
    undelimitable_input: "fatal",
    torn_tail: "warning",
    read_failed: "fatal",
    chain_break: "error",
    healed_link: "warning",
    damaged_frame: "error",
    stored_check_mismatch: "warning",
    commit_count_mismatch: "error",
    commit_root_mismatch: "error",
    uncommitted_tail: "information",
    index_claim_failed: "warning",
    missing_dependency: "warning",
    exceeds_capacity: "warning",
    declared_order_violated: "error",
    forward_reference: "error",
    position_constraint: "error",
    ill_formed_term: "error",
    opaque_term: "warning",
    labelling_refused: "warning",
    canonical_violated: "error",
    suppression_unmatched: "information",
    binding_unanchored: "information",
    parity_damaged: "warning",
    parity_mismatch: "error",
    parity_unexplained: "warning",
    parity_stale: "warning",
    repair_failed: "error",
    repaired: "information",
    unprotected_tail: "information",
    proof_malformed: "error",
    proof_failed: "error",
    proof_holds: "information",
    signature_invalid: "error",
    signature_unverified: "warning",
    lineage_begins: "information",
    lineage_mismatch: "warning",
    unreferenced_unit: "information",
    conflicting_declaration: "error",
    profile_violation: "error",
    conversion_stopped: "fatal",
    profile_not_checked: "information",
    // Registry version 2 (P8 section 3.4): codes 49 to 53.
    segment_key_not_held: "warning",
    unwrap_failed: "warning",
    key_conflict: "error",
    seal_failed: "error",
    seal_scope_violated: "warning",
} as const;

/** The finding codes that registry version 1 defines (P2 section 5.1). */
const FINDING_CODES_OF_V1 = 48;

export type FindingCode = keyof typeof FINDING_TABLE;
export type Severity = (typeof FINDING_TABLE)[FindingCode];
const FINDING_CODES = Object.keys(FINDING_TABLE) as FindingCode[];

/**
 * The reason enumeration of P2 section 6.1, in code order, each with the
 * finding codes it qualifies (the rows of the table, by their code numbers).
 */
const REASON_TABLE = {
    unknown_codec: [5],
    unknown_frame_type: [5],
    not_deterministic: [7],
    wrong_type: [7],
    wrong_length: [7],
    out_of_range: [7],
    unregistered_value: [7],
    unordered_or_repeated: [7],
    inconsistent_counts: [7],
    forbidden_combination: [7],
    codec_constraint: [7],
    payload_digest: [13],
    decoded_length: [13],
    blob_length: [13],
    blob_digest: [13],
    decode_failed: [13],
    index_head: [18],
    index_root: [18],
    index_offsets: [18],
    index_digest: [18],
    index_members: [18],
    index_directory: [18],
    index_locator: [18],
    chunk: [19],
    term: [28],
    value: [19],
    blob: [19, 28],
    frame: [19],
    state: [19],
    decoded_size: [20],
    stored_size: [20],
    refusal_width: [26],
    refusal_limit: [26],
    quad: [28],
    reifier: [28],
    graph: [28],
    reifier_binding: [28],
    annotation: [28],
    binding: [28],
    data: [31],
    parity: [31],
    too_many_erasures: [34],
    verify_failed: [34],
    count: [37],
    length: [37],
    order: [37],
    inclusion: [38, 39],
    consistency_old_root: [38],
    consistency_new_root: [38],
    consistency_diverged: [38],
    range: [38, 39],
    consistency: [39],
    unknown_signature_algorithm: [41],
    key_not_held: [41],
    head: [43],
    ordinal: [43],
    key_mismatch: [40],
    value_length: [40],
    non_canonical_scalar: [40],
    small_order_key: [40],
    small_order_nonce: [40],
    equation_failed: [40],
    composite_classical_failed: [40],
    composite_post_quantum_failed: [40],
    header: [27],
    frame_sequence: [27],
    terms: [27],
    rows: [27],
    value_region: [27],
    blob_region: [27],
    bindings: [27],
    suppression: [27],
    commit: [27],
    index: [27],
    labelling: [27],
    content_digest: [27],
    source_damaged: [47],
    source_not_decodable: [47],
    unrepresentable_term: [47],
    unrepresentable_frame: [47],
    source_not_deterministic: [47],
    source_exceeds_capacity: [47],
    content_mismatch: [47],
    unknown_item_kind: [5],
    refusal_size: [26],
    total_decoded: [20],
    item_count: [20],
    state_unavailable: [19],
    total_rebuilt: [20],
    total_formed: [20],
    refusal_weight: [26],
    refusal_term_node: [26],
    // Registry version 2 (P8 section 3.5): reasons 93 to 98.
    padding: [7],
    sealed_ordinal: [7],
    sealed_members: [18],
    failed_openings: [20],
    aead: [52],
    commitment: [52],
} as const;

/** The reasons that registry version 1 defines (P2 section 6.1). */
const REASONS_OF_V1 = 92;

export type Reason = keyof typeof REASON_TABLE;
const REASONS = Object.keys(REASON_TABLE) as Reason[];

/** The reasons each finding code lists (P2 section 5.1); a code with none takes no reason. */
const REASONS_OF_CODE = new Map<FindingCode, Reason[]>();
for (const reason of REASONS) {
    for (const number of REASON_TABLE[reason]) {
        const code = FINDING_CODES[number - 1];
        REASONS_OF_CODE.set(code, [
            ...(REASONS_OF_CODE.get(code) ?? []),
            reason,
        ]);
    }
}

/** The three reasons that a `rebuild_state` of tag `incomplete` can carry (P2 section 4.7). */
const INCOMPLETE_REASONS = ["chunk", "blob_length", "blob_digest"] as const;
export type IncompleteReason = (typeof INCOMPLETE_REASONS)[number];

/** A digest as its bytes (P1, "Every preimage, typed"). */
export type Digest = Uint8Array;

/** `read_result` (P7 section 1.1). */
export interface ReadResult {
    readers: (typeof READERS_CLAIM)[number];
    registryVersion: bigint;
    profiles: readonly ProfileChecks[];
    capacity: Capacity;
    marks: Limits | undefined;
    inputs: Inputs;
    file: FileResult;
}

export interface ProfileChecks {
    profile: bigint;
    checks: bigint;
}

/** The limits of a read, the eight `uint` fields of `capacity` but `items_per_read` and `access`. */
export interface Limits {
    storedBytesPerFrame: bigint;
    decodedBytesPerFrame: bigint;
    labellingUnits: bigint;
    labelledLines: bigint;
    decodedBytesPerRead: bigint;
    rebuiltBytesPerRead: bigint;
    formedBytesPerRead: bigint;
    labelledBytesPerRead: bigint;
}

/** The capacity of a read: the limits and the number of items per read, and the access. */
export interface Capacity extends Limits {
    itemsPerRead: bigint;
    access: Access;
}

/** The typed inputs of a verifier (P6 section 3.3). */
export interface Inputs {
    trustedKeys: readonly Digest[];
    heldKeys: readonly HeldKey[];
}

export interface HeldKey {
    keyId: Digest;
    algorithm: bigint;
    publicKey: Uint8Array;
}

/** `file_result` (P7 section 1.1): tag 1 holds the first-generation result as bytes. */
export type FileResult =
    | { tag: "first_generation"; firstGenerationResult: Uint8Array }
    | {
          tag: "second_generation";
          segments: readonly SegmentResult[];
          findings: readonly Finding[];
          contentDigest: Digest | undefined;
          visible: Visibility;
          end: FileEnd;
          leftOut: bigint;
      };

export type Visibility =
    | { state: "complete"; unauthenticatedSuppress: bigint }
    | {
          state: "incomplete";
          unfoldedSuppress: bigint;
          unopenedSealed: bigint;
          unauthenticatedSuppress: bigint;
      };

export type Opener =
    | { kind: "header" }
    | {
          kind: "resync";
          reached: ResyncReach;
          displaced: boolean;
          past: PastState;
          marksIgnored: bigint;
      };

export interface SegmentResult {
    ordinal: bigint;
    offset: bigint;
    length: bigint;
    opener: Opener;
    headerIdentifier: Digest | undefined;
    registryVersion: bigint | undefined;
    hashAlgorithm: bigint | undefined;
    profile: bigint | undefined;
    integrity: IntegrityState;
    authenticity: AuthenticityState;
    anchoredFrames: bigint;
    committedFrames: bigint;
    authenticatedFrames: bigint;
    transparentFrames: readonly bigint[];
    canonical: CanonicalResult;
    contentDigest: Digest | undefined;
    frames: readonly FrameResult[];
    findings: readonly Finding[];
}

export interface FrameResult {
    position: bigint;
    offset: bigint;
    length: bigint;
    frameType: bigint | undefined;
    link: LinkState;
    payload: PayloadState;
    stored: StoredState | undefined;
    signatures: readonly SignatureResult[];
    fold: FoldState;
    rebuilds: readonly RebuildResult[];
    findings: readonly Finding[];
}

export type PayloadState =
    | { state: "verified" }
    | { state: "damaged" }
    | { state: "unchecked"; reason: UncheckedReason };

export type FoldState =
    | { state: "folded" }
    | { state: "not_content" }
    | { state: "opaque"; reason: OpaqueReason };

export type RebuildState =
    | { state: "verified" }
    | { state: "incomplete"; reason: IncompleteReason }
    | { state: "refused" }
    | { state: "not_run" };

export interface RebuildResult {
    member: bigint | undefined;
    state: RebuildState;
}

/** `signature_result` (P6 section 6.1). The code of `algorithm` is kept as read. */
export interface SignatureResult {
    status: SignatureStatus;
    reason: Reason | undefined;
    algorithm: bigint;
    keyId: Digest;
    coverageFirst: bigint;
    coverageEnd: bigint;
    authenticated: boolean;
}

/** `finding` (P2 section 5). Its severity is that of its code (`severityOf`). */
export interface Finding {
    code: FindingCode;
    reason: Reason | undefined;
    subject: Subject | undefined;
    position: bigint | undefined;
    offset: bigint | undefined;
    length: bigint | undefined;
    count: bigint | undefined;
}

/** `subject_kind` (P2): a codec, frame type, signature algorithm, profile or item kind code, as read. */
export type Subject =
    | { kind: "codec"; code: bigint }
    | { kind: "frame_type"; code: bigint }
    | { kind: "signature_algorithm"; code: bigint }
    | { kind: "profile"; code: bigint; check: bigint | undefined }
    | { kind: "item_kind"; code: bigint };

export interface CanonicalResult {
    claimed: boolean;
    structure: CheckState;
    labelling: CheckState;
    storage: StorageCheckState;
    firstDifference: Difference | undefined;
}

export type CheckState =
    | { state: "verified" }
    | { state: "unchecked"; reason: UncheckedReason }
    | { state: "failed" };

export type StorageCheckState =
    | { state: "verified" }
    | { state: "unchecked"; reason: UncheckedReason }
    | { state: "differs" };

export interface Difference {
    check: CanonicalCheck;
    itemPosition: bigint;
}

// ---------------------------------------------------------------------------
// Tree access. Decoded values are checked against the structure they must hold.

function uint(value: Tree, name: string): bigint {
    if (typeof value !== "bigint" || value < 0n) {
        throw new TypeError(`${name} must be an unsigned integer`);
    }
    return value;
}

function flag(value: Tree, name: string): boolean {
    if (typeof value !== "boolean")
        throw new TypeError(`${name} must be a bool`);
    return value;
}

function bytes(value: Tree, name: string): Uint8Array {
    if (!(value instanceof Uint8Array)) {
        throw new TypeError(`${name} must be a byte string`);
    }
    return value;
}

function items(value: Tree, name: string, count?: number): readonly Tree[] {
    if (!Array.isArray(value)) throw new TypeError(`${name} must be an array`);
    if (count !== undefined && value.length !== count) {
        throw new RangeError(
            `${name} must have ${count} items, found ${value.length}`,
        );
    }
    return value as readonly Tree[];
}

/** The fields after the tag of a variant, with the tag checked against the table. */
function variant(
    value: Tree,
    name: string,
): { tag: bigint; rest: readonly Tree[] } {
    const list = items(value, name);
    if (list.length === 0) throw new RangeError(`${name} has no tag`);
    return { tag: uint(list[0], `${name} tag`), rest: list.slice(1) };
}

function refuse(name: string, tag: bigint): never {
    throw new RangeError(`${name} tag ${tag} is not in its enumeration`);
}

function list<T>(
    value: Tree,
    name: string,
    read: (item: Tree, name: string) => T,
): T[] {
    return items(value, name).map((item) => read(item, name));
}

function option<T>(
    value: Tree,
    name: string,
    read: (item: Tree, name: string) => T,
): T | undefined {
    const { tag, rest } = variant(value, name);
    if (tag === 1n && rest.length === 0) return undefined;
    if (tag === 2n && rest.length === 1) return read(rest[0], name);
    throw new RangeError(`${name} must be absent [1] or present [2, value]`);
}

function enumeration<T extends readonly string[]>(
    value: Tree,
    name: string,
    table: T,
): T[number] {
    const code = uint(value, name);
    const label = table[Number(code) - 1];
    if (code < 1n || code > BigInt(table.length) || label === undefined) {
        throw new RangeError(`${name} code ${code} is outside its enumeration`);
    }
    return label;
}

function codeOf<T extends readonly string[]>(
    table: T,
    label: T[number],
): bigint {
    const index = table.indexOf(label);
    if (index < 0) throw new RangeError(`${label} is not in its enumeration`);
    return BigInt(index + 1);
}

// ---------------------------------------------------------------------------
// Records: reading (tree to typed value).

function readLimits(value: Tree, name: string): Limits {
    const [a, b, c, d, e, f, g, h] = items(value, name, 8).map((item) =>
        uint(item, name),
    ) as [bigint, bigint, bigint, bigint, bigint, bigint, bigint, bigint];
    return {
        storedBytesPerFrame: a,
        decodedBytesPerFrame: b,
        labellingUnits: c,
        labelledLines: d,
        decodedBytesPerRead: e,
        rebuiltBytesPerRead: f,
        formedBytesPerRead: g,
        labelledBytesPerRead: h,
    };
}

function readCapacity(value: Tree): Capacity {
    const f = items(value, "capacity", 10);
    return {
        storedBytesPerFrame: uint(f[0], "capacity"),
        decodedBytesPerFrame: uint(f[1], "capacity"),
        labellingUnits: uint(f[2], "capacity"),
        labelledLines: uint(f[3], "capacity"),
        decodedBytesPerRead: uint(f[4], "capacity"),
        itemsPerRead: uint(f[5], "capacity"),
        rebuiltBytesPerRead: uint(f[6], "capacity"),
        formedBytesPerRead: uint(f[7], "capacity"),
        labelledBytesPerRead: uint(f[8], "capacity"),
        access: enumeration(f[9], "capacity.access", ACCESS),
    };
}

function readHeldKey(value: Tree): HeldKey {
    const f = items(value, "held_key", 3);
    return {
        keyId: bytes(f[0], "held_key.key_id"),
        algorithm: uint(f[1], "held_key.algorithm"),
        publicKey: bytes(f[2], "held_key.public_key"),
    };
}

function readInputs(value: Tree): Inputs {
    const f = items(value, "inputs", 2);
    return {
        trustedKeys: list(f[0], "trusted_keys", (item, name) =>
            bytes(item, name),
        ),
        heldKeys: list(f[1], "held_keys", readHeldKey),
    };
}

function readVisibility(value: Tree): Visibility {
    const { tag, rest } = variant(value, "visibility");
    if (tag === 1n && rest.length === 1) {
        return {
            state: "complete",
            unauthenticatedSuppress: uint(rest[0], "visibility"),
        };
    }
    if (tag === 2n && rest.length === 3) {
        return {
            state: "incomplete",
            unfoldedSuppress: uint(rest[0], "visibility"),
            unopenedSealed: uint(rest[1], "visibility"),
            unauthenticatedSuppress: uint(rest[2], "visibility"),
        };
    }
    return refuse("visibility", tag);
}

function readOpener(value: Tree): Opener {
    const { tag, rest } = variant(value, "opener");
    if (tag === 1n && rest.length === 0) return { kind: "header" };
    if (tag === 2n && rest.length === 4) {
        return {
            kind: "resync",
            reached: enumeration(rest[0], "opener.reached", RESYNC_REACH),
            displaced: flag(rest[1], "opener.displaced"),
            past: enumeration(rest[2], "opener.past", PAST_STATE),
            marksIgnored: uint(rest[3], "opener.marks_ignored"),
        };
    }
    return refuse("opener", tag);
}

function readUncheckedReason(value: Tree): UncheckedReason {
    return enumeration(value, "unchecked_reason", UNCHECKED_REASON);
}

function readPayload(value: Tree): PayloadState {
    const { tag, rest } = variant(value, "payload_state");
    if (tag === 1n && rest.length === 0) return { state: "verified" };
    if (tag === 2n && rest.length === 0) return { state: "damaged" };
    if (tag === 3n && rest.length === 1) {
        return {
            state: "unchecked",
            reason: readUncheckedReason(rest[0]),
        };
    }
    return refuse("payload_state", tag);
}

function readFold(value: Tree): FoldState {
    const { tag, rest } = variant(value, "fold_state");
    if (tag === 1n && rest.length === 0) return { state: "folded" };
    if (tag === 2n && rest.length === 0) return { state: "not_content" };
    if (tag === 3n && rest.length === 1) {
        return {
            state: "opaque",
            reason: enumeration(rest[0], "opaque_reason", OPAQUE_REASON),
        };
    }
    return refuse("fold_state", tag);
}

/** The severity of a finding code: the fixed column of P2 section 5.1, never a field of a finding. */
export function severityOf(code: FindingCode): Severity {
    return FINDING_TABLE[code];
}

/**
 * P2-35: a finding carries a reason exactly when its code lists reasons, and
 * then the reason is one of them. Throws otherwise.
 */
function checkReason(
    code: FindingCode,
    reason: Reason | undefined,
    name: string,
): void {
    const listed = REASONS_OF_CODE.get(code) ?? [];
    if (listed.length === 0) {
        if (reason !== undefined) {
            throw new RangeError(`${name}: finding ${code} takes no reason`);
        }
        return;
    }
    if (reason === undefined || !listed.includes(reason)) {
        throw new RangeError(
            `${name}: ${reason ?? "no reason"} is not a reason of finding ${code}`,
        );
    }
}

/**
 * P6 section 6.2: `invalid` carries one of the eight reasons of finding code
 * `signature_invalid`, `unverified` one of the two of `signature_unverified`,
 * and `valid` carries no reason.
 */
function checkSignatureReason(
    status: SignatureStatus,
    reason: Reason | undefined,
): void {
    if (status === "valid") {
        if (reason !== undefined) {
            throw new RangeError("a valid signature carries no reason");
        }
        return;
    }
    checkReason(
        status === "invalid" ? "signature_invalid" : "signature_unverified",
        reason,
        "signature_result",
    );
}

function isIncompleteReason(reason: Reason): reason is IncompleteReason {
    return (INCOMPLETE_REASONS as readonly string[]).includes(reason);
}

function readRebuildState(value: Tree): RebuildState {
    const { tag, rest } = variant(value, "rebuild_state");
    if (tag === 1n && rest.length === 0) return { state: "verified" };
    if (tag === 2n && rest.length === 1) {
        const reason = reasonOf(rest[0], "rebuild reason");
        if (!isIncompleteReason(reason)) {
            throw new RangeError(`${reason} is not a reason of rebuild_state`);
        }
        return { state: "incomplete", reason };
    }
    if (tag === 3n && rest.length === 0) return { state: "refused" };
    if (tag === 4n && rest.length === 0) return { state: "not_run" };
    return refuse("rebuild_state", tag);
}

function readRebuild(value: Tree): RebuildResult {
    const f = items(value, "rebuild_result", 2);
    return {
        member: option(f[0], "rebuild_result.member", uint),
        state: readRebuildState(f[1]),
    };
}

function readSubject(value: Tree): Subject {
    const { tag, rest } = variant(value, "subject");
    if (tag === 4n && rest.length === 2) {
        return {
            kind: "profile",
            code: uint(rest[0], "subject.profile"),
            check: option(rest[1], "subject.check", uint),
        };
    }
    if (rest.length !== 1) return refuse("subject", tag);
    const code = uint(rest[0], "subject");
    switch (tag) {
        case 1n:
            return { kind: "codec", code };
        case 2n:
            return { kind: "frame_type", code };
        case 3n:
            return { kind: "signature_algorithm", code };
        case 5n:
            return { kind: "item_kind", code };
        default:
            return refuse("subject", tag);
    }
}

function reasonOf(value: Tree, name: string): Reason {
    return enumeration(value, name, REASONS);
}

function readFinding(value: Tree): Finding {
    const f = items(value, "finding", 7);
    const code = enumeration(f[0], "finding_code", FINDING_CODES);
    const reason = option(f[1], "finding.reason", reasonOf);
    checkReason(code, reason, "finding");
    return {
        code,
        reason,
        subject: option(f[2], "finding.subject", readSubject),
        position: option(f[3], "finding.position", uint),
        offset: option(f[4], "finding.offset", uint),
        length: option(f[5], "finding.length", uint),
        count: option(f[6], "finding.count", uint),
    };
}

function readSignature(value: Tree): SignatureResult {
    const f = items(value, "signature_result", 7);
    const status = enumeration(f[0], "signature_status", SIGNATURE_STATUS);
    const reason = option(f[1], "signature_result.reason", reasonOf);
    checkSignatureReason(status, reason);
    return {
        status,
        reason,
        algorithm: uint(f[2], "signature_result.algorithm"),
        keyId: bytes(f[3], "signature_result.key_id"),
        coverageFirst: uint(f[4], "signature_result.coverage_first"),
        coverageEnd: uint(f[5], "signature_result.coverage_end"),
        authenticated: flag(f[6], "signature_result.authenticated"),
    };
}

function readFrame(value: Tree): FrameResult {
    const f = items(value, "frame_result", 11);
    return {
        position: uint(f[0], "frame_result.position"),
        offset: uint(f[1], "frame_result.offset"),
        length: uint(f[2], "frame_result.length"),
        frameType: option(f[3], "frame_result.frame_type", uint),
        link: enumeration(f[4], "link_state", LINK_STATE),
        payload: readPayload(f[5]),
        stored: option(f[6], "frame_result.stored", (item, name) =>
            enumeration(item, name, STORED_STATE),
        ),
        signatures: list(f[7], "frame_result.signatures", readSignature),
        fold: readFold(f[8]),
        rebuilds: list(f[9], "frame_result.rebuilds", readRebuild),
        findings: list(f[10], "frame_result.findings", readFinding),
    };
}

function readCheck(value: Tree, name: string): CheckState {
    const { tag, rest } = variant(value, name);
    if (tag === 1n && rest.length === 0) return { state: "verified" };
    if (tag === 2n && rest.length === 1) {
        return { state: "unchecked", reason: readUncheckedReason(rest[0]) };
    }
    if (tag === 3n && rest.length === 0) return { state: "failed" };
    return refuse(name, tag);
}

function readStorageCheck(value: Tree): StorageCheckState {
    const { tag, rest } = variant(value, "storage_check_state");
    if (tag === 1n && rest.length === 0) return { state: "verified" };
    if (tag === 2n && rest.length === 1) {
        return {
            state: "unchecked",
            reason: readUncheckedReason(rest[0]),
        };
    }
    if (tag === 3n && rest.length === 0) return { state: "differs" };
    return refuse("storage_check_state", tag);
}

function readCanonical(value: Tree): CanonicalResult {
    const f = items(value, "canonical_result", 5);
    return {
        claimed: flag(f[0], "canonical_result.claimed"),
        structure: readCheck(f[1], "check_state"),
        labelling: readCheck(f[2], "check_state"),
        storage: readStorageCheck(f[3]),
        firstDifference: option(
            f[4],
            "canonical_result.first_difference",
            (item, name) => {
                const d = items(item, name, 2);
                return {
                    check: enumeration(
                        d[0],
                        "canonical_check",
                        CANONICAL_CHECK,
                    ),
                    itemPosition: uint(d[1], name),
                };
            },
        ),
    };
}

function readSegment(value: Tree): SegmentResult {
    const f = items(value, "segment_result", 18);
    return {
        ordinal: uint(f[0], "segment_result.ordinal"),
        offset: uint(f[1], "segment_result.offset"),
        length: uint(f[2], "segment_result.length"),
        opener: readOpener(f[3]),
        headerIdentifier: option(
            f[4],
            "segment_result.header_identifier",
            bytes,
        ),
        registryVersion: option(f[5], "segment_result.registry_version", uint),
        hashAlgorithm: option(f[6], "segment_result.hash_algorithm", uint),
        profile: option(f[7], "segment_result.profile", uint),
        integrity: enumeration(f[8], "integrity_state", INTEGRITY_STATE),
        authenticity: enumeration(
            f[9],
            "authenticity_state",
            AUTHENTICITY_STATE,
        ),
        anchoredFrames: uint(f[10], "segment_result.anchored_frames"),
        committedFrames: uint(f[11], "segment_result.committed_frames"),
        authenticatedFrames: uint(f[12], "segment_result.authenticated_frames"),
        transparentFrames: list(
            f[13],
            "segment_result.transparent_frames",
            uint,
        ),
        canonical: readCanonical(f[14]),
        contentDigest: option(f[15], "segment_result.content_digest", bytes),
        frames: list(f[16], "segment_result.frames", readFrame),
        findings: list(f[17], "segment_result.findings", readFinding),
    };
}

function readFile(value: Tree): FileResult {
    const { tag, rest } = variant(value, "file_result");
    if (tag === 1n && rest.length === 1) {
        return {
            tag: "first_generation",
            firstGenerationResult: bytes(rest[0], "first_generation_result"),
        };
    }
    if (tag === 2n && rest.length === 6) {
        return {
            tag: "second_generation",
            segments: list(rest[0], "file_result.segments", readSegment),
            findings: list(rest[1], "file_result.findings", readFinding),
            contentDigest: option(rest[2], "file_result.content_digest", bytes),
            visible: readVisibility(rest[3]),
            end: enumeration(rest[4], "file_end", FILE_END),
            leftOut: uint(rest[5], "file_result.left_out"),
        };
    }
    return refuse("file_result", tag);
}

function readReadResult(value: Tree): ReadResult {
    const f = items(value, "read_result", 7);
    return {
        readers: enumeration(f[0], "readers_claim", READERS_CLAIM),
        registryVersion: uint(f[1], "read_result.registry_version"),
        profiles: list(f[2], "read_result.profiles", (item, name) => {
            const p = items(item, name, 2);
            return { profile: uint(p[0], name), checks: uint(p[1], name) };
        }),
        capacity: readCapacity(f[3]),
        marks: option(f[4], "read_result.marks", readLimits),
        inputs: readInputs(f[5]),
        file: readFile(f[6]),
    };
}

// ---------------------------------------------------------------------------
// Records: writing (typed value to tree). Every field is written in its order.

function optionOf<T>(value: T | undefined, write: (item: T) => Tree): Tree {
    return value === undefined ? [1n] : [2n, write(value)];
}

function enumCode<T extends readonly string[]>(
    table: T,
    label: T[number],
): bigint {
    return codeOf(table, label);
}

const id = (value: Digest | Uint8Array): Tree => value;

function writeLimits(l: Limits): Tree {
    return [
        l.storedBytesPerFrame,
        l.decodedBytesPerFrame,
        l.labellingUnits,
        l.labelledLines,
        l.decodedBytesPerRead,
        l.rebuiltBytesPerRead,
        l.formedBytesPerRead,
        l.labelledBytesPerRead,
    ];
}

function writeCapacity(c: Capacity): Tree {
    return [
        c.storedBytesPerFrame,
        c.decodedBytesPerFrame,
        c.labellingUnits,
        c.labelledLines,
        c.decodedBytesPerRead,
        c.itemsPerRead,
        c.rebuiltBytesPerRead,
        c.formedBytesPerRead,
        c.labelledBytesPerRead,
        enumCode(ACCESS, c.access),
    ];
}

function writeInputs(i: Inputs): Tree {
    return [
        i.trustedKeys.map(id),
        i.heldKeys.map((k) => [k.keyId, k.algorithm, k.publicKey]),
    ];
}

function writeVisibility(v: Visibility): Tree {
    return v.state === "complete"
        ? [1n, v.unauthenticatedSuppress]
        : [2n, v.unfoldedSuppress, v.unopenedSealed, v.unauthenticatedSuppress];
}

function writeOpener(o: Opener): Tree {
    return o.kind === "header"
        ? [1n]
        : [
              2n,
              enumCode(RESYNC_REACH, o.reached),
              o.displaced,
              enumCode(PAST_STATE, o.past),
              o.marksIgnored,
          ];
}

function writeUncheckedReason(reason: UncheckedReason): Tree {
    return enumCode(UNCHECKED_REASON, reason);
}

function writePayload(p: PayloadState): Tree {
    switch (p.state) {
        case "verified":
            return [1n];
        case "damaged":
            return [2n];
        case "unchecked":
            return [3n, writeUncheckedReason(p.reason)];
    }
}

function writeFold(f: FoldState): Tree {
    switch (f.state) {
        case "folded":
            return [1n];
        case "not_content":
            return [2n];
        case "opaque":
            return [3n, enumCode(OPAQUE_REASON, f.reason)];
    }
}

function writeRebuildState(s: RebuildState): Tree {
    switch (s.state) {
        case "verified":
            return [1n];
        case "incomplete":
            return [2n, enumCode(REASONS, s.reason)];
        case "refused":
            return [3n];
        case "not_run":
            return [4n];
    }
}

function writeRebuild(r: RebuildResult): Tree {
    return [optionOf(r.member, (m) => m), writeRebuildState(r.state)];
}

function writeSubject(s: Subject): Tree {
    switch (s.kind) {
        case "codec":
            return [1n, s.code];
        case "frame_type":
            return [2n, s.code];
        case "signature_algorithm":
            return [3n, s.code];
        case "profile":
            return [4n, s.code, optionOf(s.check, (c) => c)];
        case "item_kind":
            return [5n, s.code];
    }
}

function writeFinding(f: Finding): Tree {
    checkReason(f.code, f.reason, "finding");
    return [
        enumCode(FINDING_CODES, f.code),
        optionOf(f.reason, (r) => enumCode(REASONS, r)),
        optionOf(f.subject, writeSubject),
        optionOf(f.position, (p) => p),
        optionOf(f.offset, (o) => o),
        optionOf(f.length, (l) => l),
        optionOf(f.count, (c) => c),
    ];
}

function writeSignature(s: SignatureResult): Tree {
    checkSignatureReason(s.status, s.reason);
    return [
        enumCode(SIGNATURE_STATUS, s.status),
        optionOf(s.reason, (r) => enumCode(REASONS, r)),
        s.algorithm,
        s.keyId,
        s.coverageFirst,
        s.coverageEnd,
        s.authenticated,
    ];
}

function writeFrame(f: FrameResult): Tree {
    return [
        f.position,
        f.offset,
        f.length,
        optionOf(f.frameType, (t) => t),
        enumCode(LINK_STATE, f.link),
        writePayload(f.payload),
        optionOf(f.stored, (s) => enumCode(STORED_STATE, s)),
        f.signatures.map(writeSignature),
        writeFold(f.fold),
        f.rebuilds.map(writeRebuild),
        f.findings.map(writeFinding),
    ];
}

function writeCheck(c: CheckState | StorageCheckState): Tree {
    switch (c.state) {
        case "verified":
            return [1n];
        case "unchecked":
            return [2n, writeUncheckedReason(c.reason)];
        case "failed":
            return [3n];
        case "differs":
            return [3n];
    }
}

function writeCanonical(c: CanonicalResult): Tree {
    return [
        c.claimed,
        writeCheck(c.structure),
        writeCheck(c.labelling),
        writeCheck(c.storage),
        optionOf(c.firstDifference, (d) => [
            enumCode(CANONICAL_CHECK, d.check),
            d.itemPosition,
        ]),
    ];
}

function writeSegment(s: SegmentResult): Tree {
    return [
        s.ordinal,
        s.offset,
        s.length,
        writeOpener(s.opener),
        optionOf(s.headerIdentifier, id),
        optionOf(s.registryVersion, (v) => v),
        optionOf(s.hashAlgorithm, (v) => v),
        optionOf(s.profile, (v) => v),
        enumCode(INTEGRITY_STATE, s.integrity),
        enumCode(AUTHENTICITY_STATE, s.authenticity),
        s.anchoredFrames,
        s.committedFrames,
        s.authenticatedFrames,
        [...s.transparentFrames],
        writeCanonical(s.canonical),
        optionOf(s.contentDigest, id),
        s.frames.map(writeFrame),
        s.findings.map(writeFinding),
    ];
}

function writeFile(f: FileResult): Tree {
    return f.tag === "first_generation"
        ? [1n, f.firstGenerationResult]
        : [
              2n,
              f.segments.map(writeSegment),
              f.findings.map(writeFinding),
              optionOf(f.contentDigest, id),
              writeVisibility(f.visible),
              enumCode(FILE_END, f.end),
              f.leftOut,
          ];
}

function writeReadResult(r: ReadResult): Tree {
    return [
        enumCode(READERS_CLAIM, r.readers),
        r.registryVersion,
        r.profiles.map((p) => [p.profile, p.checks]),
        writeCapacity(r.capacity),
        optionOf(r.marks, writeLimits),
        writeInputs(r.inputs),
        writeFile(r.file),
    ];
}

// ---------------------------------------------------------------------------
// Deterministic encoding and the decoding that refuses anything it cannot read.

/** Converts the decoder's numbers to `bigint` and copies byte strings, recursively. */
function normalize(value: unknown): Tree {
    if (typeof value === "number") {
        if (!Number.isSafeInteger(value)) {
            throw new RangeError("integer outside the safe range");
        }
        return BigInt(value);
    }
    if (typeof value === "bigint" || typeof value === "boolean") return value;
    if (value instanceof Uint8Array) return new Uint8Array(value);
    if (Array.isArray(value)) return value.map(normalize);
    throw new TypeError("value outside the result data model");
}

/**
 * P2-36 and P8-5: a reader of registry version 1 reports no finding code and no
 * reason of version 2, and a reader implements only versions 1 and 2.
 */
function refuseOutsideRegistry(result: ReadResult): void {
    const version = result.registryVersion;
    if (version !== 1n && version !== 2n) {
        throw new RangeError(`registry version ${version} is not implemented`);
    }
    if (version === 2n || result.file.tag === "first_generation") return;
    const findings: Finding[] = [...result.file.findings];
    const reasons: (Reason | undefined)[] = [];
    for (const segment of result.file.segments) {
        findings.push(...segment.findings);
        for (const frame of segment.frames) {
            findings.push(...frame.findings);
            reasons.push(...frame.signatures.map((s) => s.reason));
        }
    }
    for (const finding of findings) {
        if (FINDING_CODES.indexOf(finding.code) + 1 > FINDING_CODES_OF_V1) {
            throw new RangeError(
                `finding ${finding.code} is not in registry version 1`,
            );
        }
        reasons.push(finding.reason);
    }
    for (const reason of reasons) {
        if (
            reason !== undefined &&
            REASONS.indexOf(reason) + 1 > REASONS_OF_V1
        ) {
            throw new RangeError(
                `reason ${reason} is not in registry version 1`,
            );
        }
    }
}

/**
 * The deterministic encoding of a `read_result` (P7 section 1.1, P1). The
 * caller states every field; the encoding is the sole output.
 */
export function encodeReadResult(result: ReadResult): Uint8Array {
    refuseOutsideRegistry(result);
    return encodeCanonical(writeReadResult(result));
}

/**
 * Decodes the deterministic encoding of a `read_result`.
 *
 * Throws when a field has the wrong shape, when an enumeration code is outside
 * its table, or when the bytes are not exactly the deterministic encoding of
 * the decoded value (which also refuses trailing bytes).
 */
export function decodeReadResult(bytes: Uint8Array): ReadResult {
    const result = readReadResult(normalize(decodeFirst(bytes)));
    const again = encodeReadResult(result);
    if (!equalBytes(again, bytes)) {
        throw new RangeError("the bytes are not the deterministic encoding");
    }
    return result;
}

function equalBytes(left: Uint8Array, right: Uint8Array): boolean {
    if (left.length !== right.length) return false;
    return left.every((byte, index) => byte === right[index]);
}
