// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Human-friendly emoji visual hash: a BLAKE3-XOF digest sliced into 6-bit
// symbols indexing a fixed 64-emoji alphabet. Byte-compatible with the Python
// reference; gated by vectors/emojihash/*.json.

import { hash as blake3Hash } from "blake3";

const EMOJI: readonly string[] = ["🐵", "🐶", "🐺", "🦊", "🐱", "🦁", "🐯", "🐴", "🦄", "🦓", "🦌", "🐮", "🐷", "🐗", "🐭", "🐹", "🐰", "🐻", "🐼", "🐨", "🐸", "🐲", "🐔", "🐧", "🦆", "🦅", "🦉", "🦇", "🐢", "🐍", "🦎", "🐊", "🐳", "🐬", "🐟", "🐠", "🐡", "🦈", "🐙", "🦑", "🦀", "🦞", "🦐", "🦋", "🐌", "🐞", "🐝", "🐜", "🦂", "🍎", "🍐", "🍊", "🍋", "🍌", "🍉", "🍇", "🍓", "🍒", "🍍", "🥝", "🍑", "🥥", "🥕", "🌽"];

const LABELS: readonly string[] = ["monkey", "dog", "wolf", "fox", "cat", "lion", "tiger", "horse", "unicorn", "zebra", "deer", "cow", "pig", "boar", "mouse", "hamster", "rabbit", "bear", "panda", "koala", "frog", "dragon", "chicken", "penguin", "duck", "eagle", "owl", "bat", "turtle", "snake", "lizard", "crocodile", "whale", "dolphin", "fish", "tropical-fish", "blowfish", "shark", "octopus", "squid", "crab", "lobster", "shrimp", "butterfly", "snail", "lady-beetle", "bee", "ant", "scorpion", "apple", "pear", "orange", "lemon", "banana", "watermelon", "grapes", "strawberry", "cherries", "pineapple", "kiwi", "peach", "coconut", "carrot", "corn"];

/** Return `length` 6-bit digest symbols (each in 0..64). */
export function emojiIndices(data: Uint8Array, length = 11): number[] {
    const wanted = Math.max(1, length);
    const nbytes = Math.ceil((wanted * 6) / 8);
    const digest = blake3Hash(Buffer.from(data), { length: nbytes }) as Buffer;
    const out: number[] = [];
    let acc = 0;
    let bits = 0;
    for (const byte of digest) {
        acc = (acc << 8) | byte;
        bits += 8;
        while (bits >= 6 && out.length < wanted) {
            bits -= 6;
            out.push((acc >> bits) & 0x3f);
        }
        acc &= (1 << bits) - 1; // keep only the unconsumed low bits
    }
    return out.slice(0, wanted);
}

/** Map `data` to a space-joined string of `length` emoji digits. */
export function emojihash(data: Uint8Array, length = 11): string {
    return emojiIndices(data, length)
        .map((i) => EMOJI[i])
        .join(" ");
}

/** The stable label names for {@link emojihash} output. */
export function emojihashLabels(data: Uint8Array, length = 11): string {
    return emojiIndices(data, length)
        .map((i) => LABELS[i])
        .join(" ");
}
