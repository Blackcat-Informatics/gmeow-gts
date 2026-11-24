// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Closing-item claims and committed prefixes (P1 §§2.5–2.6, P5 §§1–2).

use super::{
    frame::{COMMIT, INDEX},
    index::{index_bytes, IndexCheck, IndexPrefix},
    range::{decode_claim, Digest, Range},
    reader::{ItemKind, LinkState, WireFrame, WireItem},
    Decoder, HeaderError, MalformedReason,
};

/// A closing item disagrees with the grammar, placement, or observed sequence.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClosingFinding {
    Malformed { position: usize, error: HeaderError },
    IndexOffsets { position: usize },
    IndexLocator { position: usize },
}

fn offsets(
    data: &[u8],
    item: WireItem,
    mut visit: impl FnMut(u64, u64),
) -> Result<u64, HeaderError> {
    let mut cursor = Decoder::new(&data[..item.end], item.offset);
    cursor.record(2)?;
    cursor.uint()?;
    let count = cursor.head(4)?;
    for ordinal in 0..count {
        visit(ordinal, cursor.uint()?);
    }
    Ok(count)
}

fn locator(data: &[u8], item: WireItem) -> Result<u64, HeaderError> {
    let mut cursor = Decoder::new(&data[..item.end], item.offset);
    cursor.record(2)?;
    cursor.uint()?;
    if data.get(cursor.offset) != Some(&0x1b) {
        let error = cursor.error(MalformedReason::NotDeterministic);
        cursor.uint()?;
        return Err(error);
    }
    // The locator is the exception to the shortest-head rule.
    cursor.take(1)?;
    let distance = cursor
        .take(8)?
        .try_into()
        .map_err(|_| cursor.error(MalformedReason::WrongLength))?;
    Ok(u64::from_be_bytes(distance))
}

pub(super) fn check_closing_grammar(data: &[u8], item: WireItem) -> Result<(), HeaderError> {
    if item.kind == ItemKind::Offsets {
        offsets(data, item, |_, _| {}).map(|_| ())
    } else {
        locator(data, item).map(|_| ())
    }
}

pub(super) fn check_closing(
    data: &[u8],
    items: &[WireItem],
    frames: &[WireFrame<'_>],
) -> Vec<ClosingFinding> {
    let mut findings = Vec::new();
    let mut offsets_seen = false;
    let mut locator_seen = false;
    let last_index = items
        .last()
        .filter(|item| item.kind == ItemKind::Locator)
        .and_then(|_| frames.iter().rfind(|frame| frame.frame_type == Some(INDEX)))
        .and_then(|frame| items.get(frame.position))
        .map(|item| item.offset as u64);
    for (position, &item) in items.iter().enumerate() {
        let rest = &items[position + 1..];
        let misplaced = match item.kind {
            ItemKind::Offsets => {
                let place_holds =
                    rest.is_empty() || matches!(rest, [next] if next.kind == ItemKind::Locator);
                let misplaced = offsets_seen || locator_seen || !place_holds;
                offsets_seen = true;
                misplaced
            }
            ItemKind::Locator => {
                let misplaced = locator_seen || !rest.is_empty();
                locator_seen = true;
                misplaced
            }
            _ => continue,
        };
        if misplaced {
            findings.push(ClosingFinding::Malformed {
                position,
                error: HeaderError {
                    reason: MalformedReason::UnorderedOrRepeated,
                    offset: item.offset,
                },
            });
            continue;
        }
        let (check, mismatch) = if item.kind == ItemKind::Offsets {
            let mut holds = true;
            let count = offsets(data, item, |ordinal, length| {
                holds &= usize::try_from(ordinal)
                    .ok()
                    .and_then(|ordinal| items[..position].get(ordinal))
                    .is_some_and(|covered| (covered.end - covered.offset) as u64 == length);
            });
            (
                count.map(|count| holds && count == position as u64),
                ClosingFinding::IndexOffsets { position },
            )
        } else {
            let check = locator(data, item).map(|distance| {
                distance != 0
                    && (item.offset as u64)
                        .checked_sub(distance)
                        .is_some_and(|candidate| Some(candidate) == last_index)
            });
            (check, ClosingFinding::IndexLocator { position })
        };
        match check {
            Ok(true) => {}
            Ok(false) => findings.push(mismatch),
            Err(error) => findings.push(ClosingFinding::Malformed { position, error }),
        }
    }
    findings
}

/// The claims a commit makes about its observed prefix.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CommitCheck {
    pub position: usize,
    /// Number of preceding ordinary frames with effective identifiers.
    pub ordinary_position: u64,
    pub count_matches: bool,
    pub root_matches: bool,
    pub valid: bool,
}

/// Ordinary frames after the last valid commit, excluding index frames.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UncommittedTail {
    pub position: usize,
    pub count: u64,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CommitReport {
    pub checks: Vec<CommitCheck>,
    pub last_valid_position: Option<usize>,
    pub committed_frames: u64,
    pub uncommitted_tail: Option<UncommittedTail>,
}

/// Failure to construct the range of an already decoded segment.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CommitReadError {
    Content { position: usize, error: HeaderError },
    RangeFull,
}

fn commit_content<'a>(frame: &WireFrame<'a>, stored_limit: u64) -> Option<&'a [u8]> {
    let frame = frame.envelope.as_ref().ok()?;
    (frame.frame_type == COMMIT && frame.content.len() as u64 <= stored_limit)
        .then_some(frame.content)
}

pub(super) fn check_prefixes(
    opener: &Digest,
    frames: &[WireFrame<'_>],
    stored_limit: u64,
    registry: u64,
) -> Result<(CommitReport, Vec<IndexCheck>), CommitReadError> {
    let mut report = CommitReport::default();
    let mut indexes = Vec::new();
    let last_commit = frames
        .iter()
        .rposition(|frame| commit_content(frame, stored_limit).is_some());
    let last_index = frames
        .iter()
        .rposition(|frame| index_bytes(frame).is_some());
    // The same range covers both kinds of claim. Frames after them need no leaf.
    let last_claim = last_commit.max(last_index);
    let mut prefix = last_index.map(|_| IndexPrefix::new(registry, *opener));
    let mut range = Range::new(opener);
    for (index, frame) in frames.iter().enumerate() {
        let position = frame.position;
        if let Some(content) = commit_content(frame, stored_limit) {
            let (count, root) = decode_claim(content)
                .map_err(|error| CommitReadError::Content { position, error })?;
            let ordinary_position = range.ordinary_count();
            let count_matches = count == ordinary_position;
            let root_matches = *root == range.root();
            let valid = count_matches && root_matches && frame.link.state != LinkState::Broken;
            report.checks.push(CommitCheck {
                position,
                ordinary_position,
                count_matches,
                root_matches,
                valid,
            });
            if valid {
                report.last_valid_position = Some(position);
                report.committed_frames = ordinary_position + 1;
                report.uncommitted_tail = None;
            }
        }
        if let Some(prefix) = &mut prefix {
            if let Some((content, payload)) = index_bytes(frame) {
                let check = prefix
                    .check(position, content, payload, &range)
                    .map_err(|error| CommitReadError::Content { position, error })?;
                indexes.push(check);
            }
            if last_index.is_some_and(|last| index < last) {
                prefix.observe(frame);
            }
        }
        if let Some(identifier) = frame.link.effective_identifier {
            if last_claim.is_some_and(|last| index < last) {
                range
                    .append(&identifier)
                    .map_err(|_| CommitReadError::RangeFull)?;
            }
            if frame.frame_type != Some(INDEX) && report.last_valid_position != Some(position) {
                report
                    .uncommitted_tail
                    .get_or_insert(UncommittedTail { position, count: 0 })
                    .count += 1;
            }
        }
    }
    Ok((report, indexes))
}
