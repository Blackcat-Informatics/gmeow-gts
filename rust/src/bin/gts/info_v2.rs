// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Report the v2 wire observations available to the info command.

use std::collections::BTreeMap;

use gmeow_gts::{
    v2::reader::{
        read_wire, Dispatch, FramingFailure, ItemKind, LinkState, Opener, PayloadState,
        ReadCapacity, ReadStop, ResyncRefusal, WireFrame, WireSegment,
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

pub(super) fn inspect(path: &str, bytes: &[u8], capacity: ReadCapacity) -> Option<bool> {
    let wire = read_wire(bytes, capacity);
    match wire.dispatch {
        Some(Dispatch::SecondGeneration) => {}
        Some(Dispatch::FirstGeneration) => return None,
        Some(Dispatch::Empty) => {
            stdout_line!("{path}: empty input, 0 segments");
            return Some(true);
        }
        Some(Dispatch::UnsupportedGeneration(generation)) => {
            stderr_line!("gts info: {path}: unsupported GTS generation {generation}");
            return Some(false);
        }
        Some(Dispatch::NotThisFormat) => {
            stderr_line!("gts info: {path}: input does not identify a GTS generation");
            return Some(false);
        }
        None => {
            stdout_line!("{path}: GTS generation unavailable");
            if let Some(stop) = wire.stop {
                print_stop(stop);
            }
            return Some(false);
        }
    }
    stdout_line!(
        "{path}: GTS v2 wire ledger, {} segment(s)",
        wire.segments.len()
    );
    let mut clean = true;
    if let Some(failure) = wire.prefix_failure {
        print_damage("bytes before the first segment", failure);
        clean = false;
    }
    if let Some(refusal) = wire.resync_refusal {
        print_refusal("resync", refusal);
        clean = false;
    }
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
            stdout_line!(
                "  framing stopped at byte {}: {:?}",
                failure.offset,
                failure.reason
            );
        }
        ReadStop::ItemLimit { offset } => stdout_line!("  item limit reached at byte {offset}"),
    }
}

fn print_damage(place: &str, failure: FramingFailure) {
    stdout_line!(
        "  damaged framing in {place} at byte {}, {} byte(s): {:?}",
        failure.offset,
        failure.length,
        failure.reason
    );
}

fn print_refusal(kind: &str, refusal: ResyncRefusal) {
    stdout_line!(
        "  {kind} candidate refused at byte {}, {} byte(s), decoded byte total {}",
        refusal.offset,
        refusal.length,
        refusal.count
    );
}

fn print_segment(ordinal: usize, segment: &WireSegment<'_>) -> bool {
    let mut clean = print_opener(ordinal, segment);
    print_resync_claim(segment);
    print_frame_counts(segment);
    if let Some(failure) = segment.framing_failure {
        print_damage("segment", failure);
        clean = false;
    }
    if let Some(refusal) = segment.resync_refusal {
        print_refusal("resync", refusal);
        clean = false;
    }
    if let Some(refusal) = segment.parity_refusal {
        print_refusal("parity", refusal);
        clean = false;
    }
    clean &= print_items(segment);
    for frame in &segment.frames {
        clean &= print_frame_problems(frame);
    }
    clean
}

/// Prints the opener of a segment and reports whether it is well formed.
fn print_opener(ordinal: usize, segment: &WireSegment<'_>) -> bool {
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
            stdout_line!(
                "  segment {ordinal}: {kind}, effective head {head}, registry {}, hash algorithm {}, profile {profile}",
                header.registry_version, header.hash_algorithm
            );
            if let Some(canonical) = header.canonical {
                stdout_line!("    canonical claim {canonical}");
            }
            if header.registry_version != 1 {
                stdout_line!(
                    "    unsupported registry version {}",
                    header.registry_version
                );
                clean = false;
            }
            if identifier.is_none() {
                stdout_line!("    opener identifier unavailable");
                clean = false;
            }
        }
        Err(error) => {
            stdout_line!("  segment {ordinal}: unreadable {kind}: {error}");
            clean = false;
        }
    }
    clean
}

/// Prints the claims made by a resync opener and whether they hold.
fn print_resync_claim(segment: &WireSegment<'_>) {
    if let Opener::Resync(Ok(resync)) = segment.opener {
        stdout_line!("    resync claimed offset {}", resync.claimed_offset);
        if let Some(item) = segment
            .items
            .first()
            .filter(|item| resync.claimed_offset != item.offset as u64)
        {
            stdout_line!("    resync displaced to byte {}", item.offset);
        }
        if resync
            .identifier
            .is_some_and(|id| id.as_slice() != resync.stored_identifier)
        {
            stdout_line!("    stored resync identifier differs from computed identifier");
        }
    }
}

/// Prints the item and frame totals of a segment, then the count of each known frame type.
fn print_frame_counts(segment: &WireSegment<'_>) {
    let mut counts = BTreeMap::new();
    for frame in &segment.frames {
        if let Some(kind) = frame.frame_type {
            *counts.entry(kind).or_insert(0_usize) += 1;
        }
    }
    stdout_line!(
        "    {} item(s), {} frame(s)",
        segment.items.len(),
        segment.frames.len()
    );
    for (kind, count) in counts {
        match frame_name(kind) {
            Some(name) => stdout_line!("    {name} {count}"),
            None => stdout_line!("    frame type {kind}: {count}"),
        }
    }
}

/// Returns the name of a frame type, or `None` for a type this program does not name.
fn frame_name(kind: u64) -> Option<&'static str> {
    let index = usize::try_from(kind).ok()?.checked_sub(1)?;
    FRAME_NAMES.get(index).copied()
}

/// Prints each damaged, malformed or unknown item of a segment and reports whether all items are clean.
fn print_items(segment: &WireSegment<'_>) -> bool {
    let mut clean = true;
    for item in &segment.items {
        match item.kind {
            ItemKind::DamagedRegion => {
                stdout_line!(
                    "    damaged region at byte {}, {} byte(s)",
                    item.offset,
                    item.end - item.offset
                );
                clean = false;
            }
            ItemKind::Malformed(reason) => {
                stdout_line!("    malformed item at byte {}: {reason:?}", item.offset);
                clean = false;
            }
            ItemKind::Unknown(kind) => {
                stdout_line!("    unknown item type {kind} at byte {}", item.offset);
                clean = false;
            }
            ItemKind::Header
            | ItemKind::Resync
            | ItemKind::Frame
            | ItemKind::Locator
            | ItemKind::Offsets
            | ItemKind::Unchecked => {}
        }
    }
    clean
}

fn print_frame_problems(frame: &WireFrame<'_>) -> bool {
    let position = frame.position;
    let mut clean = true;
    if let Err(error) = frame.envelope.as_ref() {
        stdout_line!("    item {position}: malformed frame: {error}");
        clean = false;
    }
    if frame.link.state != LinkState::Linked {
        stdout_line!("    item {position}: link {:?}", frame.link.state);
        clean = false;
    }
    if frame.payload.stored_matches == Some(false) {
        stdout_line!("    item {position}: stored check mismatch");
        clean = false;
    }
    match frame.payload.state {
        PayloadState::Verified(_) => {}
        PayloadState::Damaged(reason) => {
            stdout_line!("    item {position}: damaged payload: {reason:?}");
            clean = false;
        }
        PayloadState::Unchecked(reason) => {
            stdout_line!("    item {position}: unchecked payload: {reason:?}");
            clean = false;
        }
    }
    clean
}
