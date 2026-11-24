// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

import { isIPv6 } from "node:net";

const blankStart =
    /[A-Za-z0-9_\u00c0-\u00d6\u00d8-\u00f6\u00f8-\u02ff\u0370-\u037d\u037f-\u1fff\u200c-\u200d\u2070-\u218f\u2c00-\u2fef\u3001-\ud7ff\uf900-\ufdcf\ufdf0-\ufffd\u{10000}-\u{effff}]/u;
const blankExtra = /[-\u00b7\u0300-\u036f\u203f-\u2040]/u;

export function blankLabelLength(text: string): number {
    let offset = 0;
    let end = 0;
    for (const char of text) {
        if (
            !blankStart.test(char) &&
            (offset === 0 || (char !== "." && !blankExtra.test(char)))
        )
            break;
        offset += char.length;
        if (char !== ".") end = offset;
    }
    return end;
}

const normalTag =
    /^(?:(?:[a-z]{2,3}(?:-[a-z]{3}){0,3}|[a-z]{4,8})(?:-[a-z]{4})?(?:-(?:[a-z]{2}|[0-9]{3}))?(?:-(?:[a-z0-9]{5,8}|[0-9][a-z0-9]{3}))*(?:-[a-wy-z0-9](?:-[a-z0-9]{2,8})+)*(?:-x(?:-[a-z0-9]{1,8})+)?|x(?:-[a-z0-9]{1,8})+)$/i;
const grandfathered =
    /^(?:en-gb-oed|i-(?:ami|bnn|default|enochian|hak|klingon|lux|mingo|navajo|pwn|tao|tay|tsu)|sgn-(?:be-fr|be-nl|ch-de))$/i;

export function validLanguageTag(text: string): boolean {
    return normalTag.test(text) || grandfathered.test(text);
}

const asciiIri = /[a-zA-Z0-9._~!$&'()*+,;=-]/;

function component(text: string, extra: string, allowPrivate = false): boolean {
    for (let offset = 0; offset < text.length;) {
        const code = text.codePointAt(offset);
        if (code === undefined) return false;
        const char = String.fromCodePoint(code);
        if (char === "%") {
            if (!/^[0-9a-fA-F]{2}$/.test(text.slice(offset + 1, offset + 3)))
                return false;
            offset += 3;
            continue;
        }
        const unicode =
            (code >= 0xa0 && code <= 0xd7ff) ||
            (code >= 0xf900 && code <= 0xfdcf) ||
            (code >= 0xfdf0 && code <= 0xffef) ||
            (code >= 0x10000 && code <= 0xdfffd && (code & 0xffff) <= 0xfffd) ||
            (code >= 0xe1000 && code <= 0xefffd);
        const privateUse =
            allowPrivate &&
            ((code >= 0xe000 && code <= 0xf8ff) ||
                (code >= 0xf0000 && code <= 0xffffd) ||
                (code >= 0x100000 && code <= 0x10fffd));
        if (
            !asciiIri.test(char) &&
            !extra.includes(char) &&
            !unicode &&
            !privateUse
        )
            return false;
        offset += char.length;
    }
    return true;
}

function authority(text: string): boolean {
    const at = text.indexOf("@");
    if (at !== -1 && !component(text.slice(0, at), ":")) return false;
    const hostPort = text.slice(at + 1);
    if (hostPort.startsWith("[")) {
        const end = hostPort.indexOf("]");
        if (end === -1) return false;
        const host = hostPort.slice(1, end);
        const version = /^v[0-9a-f]+\./i.exec(host)?.[0];
        const ipvFuture =
            version !== undefined &&
            version.length < host.length &&
            [...host.slice(version.length)].every(
                (char) => char === ":" || asciiIri.test(char),
            );
        return (
            ((isIPv6(host) && !host.includes("%")) || ipvFuture) &&
            /^(?::[0-9]*)?$/.test(hostPort.slice(end + 1))
        );
    }
    const colon = hostPort.indexOf(":");
    return colon === -1
        ? component(hostPort, "")
        : component(hostPort.slice(0, colon), "") &&
              /^[0-9]*$/.test(hostPort.slice(colon + 1));
}

/** Validate the generic IRI syntax while preserving the original spelling. */
export function validIri(text: string): boolean {
    const match =
        /^[A-Za-z][A-Za-z0-9+.-]*:([^?#]*)(?:\?([^#]*))?(?:#([\s\S]*))?$/.exec(
            text,
        );
    if (match === null) return false;
    let [, path, query = "", fragment = ""] = match;
    if (path.startsWith("//")) {
        const end = path.indexOf("/", 2);
        if (!authority(end === -1 ? path.slice(2) : path.slice(2, end)))
            return false;
        path = end === -1 ? "" : path.slice(end);
    }
    return (
        component(path, ":@/") &&
        component(query, ":@/?", true) &&
        component(fragment, ":@/?")
    );
}
