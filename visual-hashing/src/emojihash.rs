// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! A nameable 64-emoji visual hash.
//!
//! A BLAKE3-XOF digest is sliced into 6-bit symbols, each indexing a fixed
//! alphabet that favours common animals and then familiar foods over abstract,
//! confusable symbols — a fingerprint only helps if a human can read it back.

/// The 64-entry emoji alphabet (a 6-bit digit set).
pub const EMOJI: [&str; 64] = [
    "🐵", "🐶", "🐺", "🦊", "🐱", "🦁", "🐯", "🐴", "🦄", "🦓", "🦌", "🐮", "🐷", "🐗", "🐭", "🐹",
    "🐰", "🐻", "🐼", "🐨", "🐸", "🐲", "🐔", "🐧", "🦆", "🦅", "🦉", "🦇", "🐢", "🐍", "🦎", "🐊",
    "🐳", "🐬", "🐟", "🐠", "🐡", "🦈", "🐙", "🦑", "🦀", "🦞", "🦐", "🦋", "🐌", "🐞", "🐝", "🐜",
    "🦂", "🍎", "🍐", "🍊", "🍋", "🍌", "🍉", "🍇", "🍓", "🍒", "🍍", "🥝", "🍑", "🥥", "🥕", "🌽",
];

/// The stable label for each entry in [`EMOJI`], by index.
pub const LABELS: [&str; 64] = [
    "monkey",
    "dog",
    "wolf",
    "fox",
    "cat",
    "lion",
    "tiger",
    "horse",
    "unicorn",
    "zebra",
    "deer",
    "cow",
    "pig",
    "boar",
    "mouse",
    "hamster",
    "rabbit",
    "bear",
    "panda",
    "koala",
    "frog",
    "dragon",
    "chicken",
    "penguin",
    "duck",
    "eagle",
    "owl",
    "bat",
    "turtle",
    "snake",
    "lizard",
    "crocodile",
    "whale",
    "dolphin",
    "fish",
    "tropical-fish",
    "blowfish",
    "shark",
    "octopus",
    "squid",
    "crab",
    "lobster",
    "shrimp",
    "butterfly",
    "snail",
    "lady-beetle",
    "bee",
    "ant",
    "scorpion",
    "apple",
    "pear",
    "orange",
    "lemon",
    "banana",
    "watermelon",
    "grapes",
    "strawberry",
    "cherries",
    "pineapple",
    "kiwi",
    "peach",
    "coconut",
    "carrot",
    "corn",
];

/// The number of distinct emoji digits (a 6-bit alphabet).
pub const ALPHABET_SIZE: usize = 64;

/// Return `length` 6-bit digest symbols (each in `0..64`).
pub fn emoji_indices(data: &[u8], length: usize) -> Vec<usize> {
    let wanted = length.max(1);
    let nbytes = (wanted * 6).div_ceil(8);
    let mut digest = vec![0u8; nbytes];
    blake3::Hasher::new()
        .update(data)
        .finalize_xof()
        .fill(&mut digest);

    let mut out = Vec::with_capacity(wanted);
    let mut acc: u64 = 0;
    let mut bits: u32 = 0;
    for byte in digest {
        acc = (acc << 8) | u64::from(byte);
        bits += 8;
        while bits >= 6 && out.len() < wanted {
            bits -= 6;
            out.push(((acc >> bits) & 0x3f) as usize);
        }
        acc &= (1u64 << bits) - 1; // keep only the unconsumed low bits
    }
    out.truncate(wanted);
    out
}

/// Map `data` to a space-joined string of `length` emoji digits.
pub fn emojihash(data: &[u8], length: usize) -> String {
    emoji_indices(data, length)
        .into_iter()
        .map(|i| EMOJI[i])
        .collect::<Vec<_>>()
        .join(" ")
}

/// The stable label names for [`emojihash`] output (space-joined).
pub fn emojihash_labels(data: &[u8], length: usize) -> String {
    emoji_indices(data, length)
        .into_iter()
        .map(|i| LABELS[i])
        .collect::<Vec<_>>()
        .join(" ")
}
