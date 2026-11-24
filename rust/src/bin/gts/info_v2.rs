// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Report the v2 wire observations available to the info command.

use std::collections::BTreeMap;

use gmeow_gts::{
    v2::reader::{
        read_wire, Dispatch, ItemKind, LinkState, Opener, PayloadState, ReadCapacity, ReadStop,
        WireFrame, WireSegment,
    },
    wire::hex,
};

const FRAME_NAMES: [&str; 16] = [
    "terms",
    "quads",
    "reifier_bindings",
    "annotations",
    "empty_graphs",
    "bindings",
    "suppress",
    "chunk",
    "blob",
    "recipe",
    "value",
    "external",
    "pack",
    "commit",
    "index",
    "parity",
];

pub(super) fn inspect(path: &str, bytes: &[u8]) -> Option<bool> {
    let wire = read_wire(bytes, ReadCapacity::default());
    match wire.dispatch {
        Some(Dispatch::SecondGeneration) => {}
        Some(Dispatch::FirstGeneration) => return None,
        Some(Dispatch::Empty) => {
            println!("{path}: empty input, 0 segments");
            return Some(true);
        }
        Some(Dispatch::UnsupportedGeneration(generation)) => {
            eprintln!("gts info: {path}: unsupported GTS generation {generation}");
            return Some(false);
        }
        Some(Dispatch::NotThisFormat) => {
            eprintln!("gts info: {path}: input does not identify a GTS generation");
            return Some(false);
        }
        None => {
            println!("{path}: GTS generation unavailable");
            if let Some(stop) = wire.stop {
                print_stop(stop);
            }
            return Some(false);
        }
    }
    println!(
        "{path}: GTS v2 wire ledger, {} segment(s)",
        wire.segments.len()
    );
    let mut clean = true;
    for (ordinal, segment) in wire.segments.iter().enumerate() {
        clean &= print_segment(ordinal, segment);
    }
    if let Some(stop) = wire.stop {
        print_stop(stop);
        clean = false;
    }
    Some(clean)
}

fn print_stop(stop: ReadStop) {
    match stop {
        ReadStop::Framing(failure) => {
            println!(
                "  framing stopped at byte {}: {:?}",
                failure.offset, failure.reason
            );
        }
        ReadStop::ItemLimit { offset } => println!("  item limit reached at byte {offset}"),
    }
}

fn print_segment(ordinal: usize, segment: &WireSegment<'_>) -> bool {
    let (kind, decoded) = match segment.opener {
        Opener::Header(header) => (
            "header",
            header.map(|opener| (opener.header, opener.identifier)),
        ),
        Opener::Resync(resync) => (
            "resync",
            resync.map(|opener| (opener.header, opener.identifier)),
        ),
    };
    let mut clean = true;
    match decoded {
        Ok((header, identifier)) => {
            let head = segment
                .head
                .as_ref()
                .map_or_else(|| "<unavailable>".into(), |head| hex(head));
            let profile = header
                .profile
                .map_or_else(|| "<none>".into(), |profile| profile.to_string());
            println!(
                "  segment {ordinal}: {kind}, effective head {head}, registry {}, hash algorithm {}, profile {profile}",
                header.registry_version, header.hash_algorithm
            );
            if let Some(canonical) = header.canonical {
                println!("    canonical claim {canonical}");
            }
            if header.registry_version != 1 {
                println!(
                    "    unsupported registry version {}",
                    header.registry_version
                );
                clean = false;
            }
            if identifier.is_none() {
                println!("    opener identifier unavailable");
                clean = false;
            }
        }
        Err(error) => {
            println!("  segment {ordinal}: unreadable {kind}: {error}");
            clean = false;
        }
    }

    if let Opener::Resync(Ok(resync)) = segment.opener {
        println!("    resync claimed offset {}", resync.claimed_offset);
        if let Some(item) = segment
            .items
            .first()
            .filter(|item| resync.claimed_offset != item.offset as u64)
        {
            println!("    resync displaced to byte {}", item.offset);
        }
        if resync
            .identifier
            .is_some_and(|id| id.as_slice() != resync.stored_identifier)
        {
            println!("    stored resync identifier differs from computed identifier");
        }
    }

    let mut counts = BTreeMap::new();
    for frame in &segment.frames {
        if let Some(kind) = frame.frame_type {
            *counts.entry(kind).or_insert(0_usize) += 1;
        }
    }
    println!(
        "    {} item(s), {} frame(s)",
        segment.items.len(),
        segment.frames.len()
    );
    for (kind, count) in counts {
        if (1..=FRAME_NAMES.len() as u64).contains(&kind) {
            println!("    {} {count}", FRAME_NAMES[kind as usize - 1]);
        } else {
            println!("    frame type {kind}: {count}");
        }
    }

    for item in &segment.items {
        match item.kind {
            ItemKind::Malformed(reason) => {
                println!("    malformed item at byte {}: {reason:?}", item.offset);
                clean = false;
            }
            ItemKind::Unknown(kind) => {
                println!("    unknown item type {kind} at byte {}", item.offset);
                clean = false;
            }
            _ => {}
        }
    }
    for frame in &segment.frames {
        clean &= print_frame_problems(frame);
    }
    clean
}

fn print_frame_problems(frame: &WireFrame<'_>) -> bool {
    let position = frame.position;
    let mut clean = true;
    if let Err(error) = frame.envelope.as_ref() {
        println!("    item {position}: malformed frame: {error}");
        clean = false;
    }
    if frame.link.state != LinkState::Linked {
        println!("    item {position}: link {:?}", frame.link.state);
        clean = false;
    }
    if frame.payload.stored_matches == Some(false) {
        println!("    item {position}: stored check mismatch");
        clean = false;
    }
    match frame.payload.state {
        PayloadState::Verified(_) => {}
        PayloadState::Damaged(reason) => {
            println!("    item {position}: damaged payload: {reason:?}");
            clean = false;
        }
        PayloadState::Unchecked(reason) => {
            println!("    item {position}: unchecked payload: {reason:?}");
            clean = false;
        }
    }
    clean
}
