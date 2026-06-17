// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

/**
 * Minimal OpenPGP reader for Ed25519 armored public keys (`extract-key`, §9.2).
 *
 * Mirrors the Python `gts.openpgp` reference: it parses only the unencrypted
 * armored public-key certificates GPG emits for Ed25519 (OpenPGP algorithm 22)
 * keys, extracting the raw 32-byte key and computing the v4 fingerprint so GTS
 * tooling can show the embedded transport key without shelling out to `gpg`.
 * Everything else (other algorithms, encrypted secret keys, v5/v6 packets) is
 * rejected with a clear error.
 */

import { sha1 } from "@noble/hashes/sha1.js";
import { hex } from "./wire.js";

const ED25519_ALGO = 22;
/** The curve OID GPG writes for the Ed25519 signing curve (1.3.6.1.4.1.11591.15.1). */
const ED25519_OID = Uint8Array.from([
    0x2b, 0x06, 0x01, 0x04, 0x01, 0xda, 0x47, 0x0f, 0x01,
]);

/** Thrown when an armored OpenPGP key cannot be parsed or is unsupported. */
export class OpenPGPError extends Error {}

/** A parsed Ed25519 public key: the raw 32-byte key plus its v4 fingerprint. */
export interface TransportKey {
    /** The 32-byte raw Ed25519 public key (the `0x40` MPI marker stripped). */
    rawPublic: Uint8Array;
    /** Uppercase 40-hex-character OpenPGP v4 fingerprint. */
    fingerprint: string;
}

/** Decode the packet bytes from an ASCII-armored OpenPGP block. */
function stripArmor(text: string): Uint8Array {
    const lines = text.split("\n");
    let start = -1;
    let end = -1;
    for (let i = 0; i < lines.length; i++) {
        if (start === -1 && lines[i].startsWith("-----BEGIN PGP")) {
            start = i;
        } else if (start !== -1 && i > start && lines[i].startsWith("-----END PGP")) {
            end = i;
            break;
        }
    }
    if (start === -1) throw new OpenPGPError("missing armor BEGIN line");
    if (end === -1) throw new OpenPGPError("missing armor END line");

    let idx = start + 1;
    // Skip optional armor headers (Comment, Version, …) up to the blank line.
    while (idx < end && lines[idx].trim() !== "") {
        if (lines[idx].includes(":")) idx++;
        else break;
    }

    const body: string[] = [];
    for (; idx < end; idx++) {
        const line = lines[idx].replace(/\r$/, "");
        if (line.startsWith("=")) break; // CRC-24 checksum — end of base64 body.
        if (line) body.push(line);
    }
    if (body.length === 0) throw new OpenPGPError("empty armor body");
    try {
        return Uint8Array.from(Buffer.from(body.join(""), "base64"));
    } catch {
        throw new OpenPGPError("invalid base64 armor body");
    }
}

/** Read an OpenPGP multi-precision integer; returns `[bytes, nextOffset]`. */
function readMPI(data: Uint8Array, offset: number): [Uint8Array, number] {
    if (offset + 2 > data.length) throw new OpenPGPError("truncated MPI length");
    const bits = (data[offset] << 8) | data[offset + 1];
    const length = Math.ceil(bits / 8);
    const end = offset + 2 + length;
    if (end > data.length) throw new OpenPGPError("truncated MPI payload");
    return [data.subarray(offset + 2, end), end];
}

interface Packet {
    tag: number;
    body: Uint8Array;
}

/** Parse one OpenPGP packet; returns `[tag, body, nextOffset]`. */
function nextPacket(data: Uint8Array, offset: number): [number, Uint8Array, number] {
    if (offset >= data.length) throw new OpenPGPError("truncated packet header");
    const header = data[offset];
    if ((header & 0x80) === 0) throw new OpenPGPError("invalid packet tag octet");

    let tag: number;
    let length: number;
    if (header & 0x40) {
        // New-format packet.
        tag = header & 0x3f;
        offset += 1;
        if (offset >= data.length)
            throw new OpenPGPError("truncated new-format length octet");
        const lo = data[offset];
        if (lo < 192) {
            length = lo;
            offset += 1;
        } else if (lo < 224) {
            if (offset + 1 >= data.length)
                throw new OpenPGPError("truncated new-format 2-octet length");
            length = ((lo - 192) << 8) + data[offset + 1] + 192;
            offset += 2;
        } else if (lo === 255) {
            if (offset + 4 >= data.length)
                throw new OpenPGPError("truncated new-format 4-octet length");
            length =
                (data[offset + 1] * 0x1000000 +
                    (data[offset + 2] << 16) +
                    (data[offset + 3] << 8) +
                    data[offset + 4]) >>>
                0;
            offset += 5;
        } else {
            throw new OpenPGPError("partial body lengths are not supported");
        }
    } else {
        // Old-format packet.
        tag = (header >> 2) & 0x0f;
        const lengthType = header & 0x03;
        offset += 1;
        if (lengthType === 0) {
            if (offset >= data.length)
                throw new OpenPGPError("truncated old-format length octet");
            length = data[offset];
            offset += 1;
        } else if (lengthType === 1) {
            if (offset + 1 >= data.length)
                throw new OpenPGPError("truncated old-format 2-octet length");
            length = (data[offset] << 8) | data[offset + 1];
            offset += 2;
        } else if (lengthType === 2) {
            if (offset + 3 >= data.length)
                throw new OpenPGPError("truncated old-format 4-octet length");
            length =
                (data[offset] * 0x1000000 +
                    (data[offset + 1] << 16) +
                    (data[offset + 2] << 8) +
                    data[offset + 3]) >>>
                0;
            offset += 4;
        } else {
            throw new OpenPGPError("indeterminate-length packets are not supported");
        }
    }

    const end = offset + length;
    if (end > data.length) throw new OpenPGPError("packet body exceeds input");
    return [tag, data.subarray(offset, end), end];
}

/** Return every `{tag, body}` packet in the de-armored data. */
function iterPackets(data: Uint8Array): Packet[] {
    const packets: Packet[] = [];
    let offset = 0;
    while (offset < data.length) {
        const [tag, body, next] = nextPacket(data, offset);
        packets.push({ tag, body });
        offset = next;
    }
    return packets;
}

function bytesEqual(a: Uint8Array, b: Uint8Array): boolean {
    if (a.length !== b.length) return false;
    for (let i = 0; i < a.length; i++) if (a[i] !== b[i]) return false;
    return true;
}

/**
 * Parse the OID and raw key from a v4 public-key packet body; returns
 * `[rawPublicKey, endOffsetOfPublicMaterial]`.
 */
function parseEd25519PublicMaterial(body: Uint8Array): [Uint8Array, number] {
    if (body.length < 6 || body[0] !== 4)
        throw new OpenPGPError("only OpenPGP v4 public keys are supported");
    if (body[5] !== ED25519_ALGO)
        throw new OpenPGPError(`unsupported public-key algorithm ${body[5]}`);

    let offset = 6;
    if (offset >= body.length) throw new OpenPGPError("truncated public-key packet");
    const oidLen = body[offset];
    offset += 1;
    if (offset + oidLen > body.length) throw new OpenPGPError("truncated OID");
    const oid = body.subarray(offset, offset + oidLen);
    offset += oidLen;
    if (!bytesEqual(oid, ED25519_OID))
        throw new OpenPGPError(`unsupported curve OID ${hex(oid)}`);

    const [mpi, end] = readMPI(body, offset);
    // GPG encodes the Ed25519 public key as a 33-byte MPI (0x40 || 32-byte key);
    // a bare 32-byte MPI is also valid when the high bit is clear.
    if (mpi.length === 33) return [mpi.subarray(1), end];
    if (mpi.length === 32) return [mpi.slice(), end];
    throw new OpenPGPError(`unexpected Ed25519 public MPI length ${mpi.length}`);
}

/**
 * Compute the OpenPGP v4 fingerprint of a public-key packet body:
 * SHA-1(0x99 || u16-be(len(body)) || body), uppercased. SHA-1 is mandated by
 * RFC 4880 for v4 fingerprints; it is not used here as a security primitive.
 */
function fingerprint(pubKeyBody: Uint8Array): string {
    const prefix = Uint8Array.from([
        0x99,
        (pubKeyBody.length >> 8) & 0xff,
        pubKeyBody.length & 0xff,
    ]);
    const buf = new Uint8Array(prefix.length + pubKeyBody.length);
    buf.set(prefix, 0);
    buf.set(pubKeyBody, prefix.length);
    return hex(sha1(buf)).toUpperCase();
}

/**
 * Parse an armored OpenPGP certificate into its raw Ed25519 key + v4 fingerprint.
 *
 * Accepts either a public-key certificate (tag 6) or an unencrypted secret-key
 * block (tag 5); the fingerprint always covers only the public material.
 */
export function parseTransportKey(armored: string): TransportKey {
    const data = stripArmor(armored);
    for (const { tag, body } of iterPackets(data)) {
        let raw: Uint8Array;
        let pubBody: Uint8Array;
        if (tag === 6) {
            [raw] = parseEd25519PublicMaterial(body);
            pubBody = body;
        } else if (tag === 5) {
            let end: number;
            [raw, end] = parseEd25519PublicMaterial(body);
            pubBody = body.subarray(0, end);
        } else {
            continue;
        }
        return { rawPublic: raw, fingerprint: fingerprint(pubBody) };
    }
    throw new OpenPGPError("no public-key packet found");
}
