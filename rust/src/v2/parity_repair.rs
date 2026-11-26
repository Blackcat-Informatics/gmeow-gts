// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The parity findings, shard checks and repairs of a segment (P5 §6.6 stages 2 and 3, §6.7 and
//! §7.2 steps 3 to 5).

use std::collections::BTreeSet;

use super::{
    frame::PARITY,
    parity::{self, Lists, Record, Shape},
    parity_runs,
    parity_shards::{self, shard_at, shard_digest, Solved},
    payload::{PayloadBudget, PayloadRefusal, PayloadState},
    reader::{field_offsets, ItemKind, Judgement, LinkState, WireFrame, WireItem, WireSegment},
};

/// The code of a parity finding, in the registry of finding codes (P2).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ParityCode {
    /// A parity frame whose payload is above the reader's capacity (`exceeds_capacity`).
    ExceedsCapacity,
    /// A parity frame that is damaged (`parity_damaged`).
    ParityDamaged,
    /// A used parity frame whose shards fail their digests (`parity_mismatch`).
    ParityMismatch,
    /// A failing data shard that no failing item explains (`parity_unexplained`).
    ParityUnexplained,
    /// A used parity frame whose span no longer matches the bytes it covers (`parity_stale`).
    ParityStale,
    /// A span whose repair is refused (`repair_failed`).
    RepairFailed,
    /// A span whose repair is accepted (`repaired`).
    Repaired,
    /// The bytes after the last span that parity protects (`unprotected_tail`).
    UnprotectedTail,
}

/// The reason of a parity finding, in the registry of reason codes (P2).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ParityReason {
    /// The decoded payload exceeds the per-frame limit (`decoded_size`).
    DecodedSize,
    /// The stored payload exceeds the per-frame limit (`stored_size`).
    StoredSize,
    /// The total decoded length exceeds the per-read limit (`total_decoded`).
    TotalDecoded,
    /// Failing data shards (`data`).
    Data,
    /// Failing parity shards (`parity`).
    Parity,
    /// A stripe has more unknowns than its parity rows (`too_many_erasures`).
    TooManyErasures,
    /// A repair failed the check of the repaired bytes (`verify_failed`).
    VerifyFailed,
}

/// A parity finding: its code, and the reason, position, byte range and count that its code takes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ParityFinding {
    /// The code of the finding.
    pub code: ParityCode,
    /// The reason of the finding, when its code takes one.
    pub reason: Option<ParityReason>,
    /// The position of the item the finding concerns, when it concerns one.
    pub position: Option<usize>,
    /// The offset of the first byte the finding covers, when it covers bytes.
    pub offset: Option<usize>,
    /// The number of bytes the finding covers, when it covers bytes.
    pub length: Option<usize>,
    /// The count that the finding reports, when its code takes one.
    pub count: Option<u64>,
}

impl ParityFinding {
    fn bare(code: ParityCode) -> Self {
        Self {
            code,
            reason: None,
            position: None,
            offset: None,
            length: None,
            count: None,
        }
    }

    fn at(code: ParityCode, position: usize) -> Self {
        Self {
            position: Some(position),
            ..Self::bare(code)
        }
    }
}

/// A byte patch that an accepted repair writes at an offset of the input.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Patch {
    /// The offset of the first patched byte in the input.
    pub offset: usize,
    /// The bytes that replace the input at `offset`.
    pub data: Vec<u8>,
}

/// The repair of one used span, before its run recheck (P5 §7.2, step 5).
#[derive(Debug, Eq, PartialEq)]
pub(super) struct Repair {
    /// The position of the parity frame of the span.
    pub(super) position: usize,
    /// The number of shards solved and written.
    count: u64,
    /// Whether every solved shard matches its digest.
    pub(super) digests_hold: bool,
    /// The patches of the solved data shards.
    pub(super) data: Vec<Patch>,
    /// The patches of the solved parity shards.
    pub(super) parity: Vec<Patch>,
}

/// The findings and the accepted patches of the parity layer of one segment.
#[derive(Debug, Default, Eq, PartialEq)]
pub(super) struct Verification {
    /// The findings, in input order.
    pub(super) findings: Vec<ParityFinding>,
    /// The patches of the accepted repairs.
    pub(super) patches: Vec<Patch>,
}

/// The parts of the stream of a segment: its items without the transparent frames and the
/// offsets and locator items, each with its place in the stream.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Part {
    stream: usize,
    offset: usize,
    length: usize,
}

#[derive(Default)]
struct Stream {
    parts: Vec<Part>,
    end: usize,
    /// The stream ranges of the items that do not pass (P5 6.6, passing items).
    failed: Vec<(usize, usize)>,
}

impl Stream {
    fn push(&mut self, offset: usize, length: usize, passes: bool) {
        self.parts.push(Part {
            stream: self.end,
            offset,
            length,
        });
        if !passes {
            self.failed.push((self.end, self.end + length));
        }
        self.end += length;
    }

    fn last_start(&self) -> Option<usize> {
        self.parts.last().map(|part| part.stream)
    }

    /// Each part that overlaps the stream range `start..end`, with the overlapping range of it.
    fn overlapping(&self, start: usize, end: usize) -> Vec<(Part, usize, usize)> {
        let first = self
            .parts
            .partition_point(|part| part.stream + part.length <= start);
        self.parts[first..]
            .iter()
            .take_while(|part| part.stream < end)
            .map(|part| {
                let left = start.max(part.stream);
                let right = end.min(part.stream + part.length);
                (*part, left, right)
            })
            .filter(|(_, left, right)| left < right)
            .collect()
    }

    /// The stream bytes of the range `start..end`, read from the input `data`.
    fn covered(&self, data: &[u8], start: usize, end: usize) -> Vec<u8> {
        let mut covered = Vec::with_capacity(end - start);
        for (part, left, right) in self.overlapping(start, end) {
            let from = part.offset + (left - part.stream);
            covered.extend_from_slice(&data[from..part.offset + (right - part.stream)]);
        }
        covered
    }

    /// Whether a failing item overlaps the stream range `start..end`.
    fn overlaps_failed(&self, start: usize, end: usize) -> bool {
        let index = self
            .failed
            .partition_point(|&(_, failed_end)| failed_end <= start);
        self.failed
            .get(index)
            .is_some_and(|&(failed_start, _)| failed_start < end)
    }

    /// The patches that write the shard bytes of the stream range `start..end` into the input.
    fn patches(&self, start: usize, end: usize, shard: &[u8]) -> Vec<Patch> {
        self.overlapping(start, end)
            .into_iter()
            .map(|(part, left, right)| Patch {
                offset: part.offset + (left - part.stream),
                data: shard[left - start..right - start].to_vec(),
            })
            .collect()
    }
}

/// The span of one used parity frame, with the bytes of the stream that it covers.
struct Span<'p> {
    first: usize,
    end: usize,
    size: usize,
    position: usize,
    item: WireItem,
    covered: Vec<u8>,
    lists: Lists<'p>,
}

/// The shard failures of one span: the failing data and parity shards, and the data shards that
/// no failing item explains.
struct ShardFailures {
    data: BTreeSet<usize>,
    parity: BTreeSet<usize>,
    unexplained: BTreeSet<usize>,
    stale: bool,
}

struct Judge<'s> {
    data: &'s [u8],
    segment: &'s WireSegment<'s>,
    stream: Stream,
    opener_passes: bool,
    findings: Vec<ParityFinding>,
    repairs: Vec<Repair>,
    protected_end: usize,
    judged: bool,
}

/// Judge the parity frames of `segment`, check and solve the spans they protect, and return the
/// findings in input order with the patches of the accepted repairs (P5 §§6.6, 6.7, 7.2).
pub(super) fn verify<'s>(
    data: &'s [u8],
    segment: &'s WireSegment<'s>,
    budget: &mut PayloadBudget,
) -> Verification {
    let mut by_position: Vec<Option<&WireFrame<'s>>> = vec![None; segment.items.len()];
    for frame in &segment.frames {
        if let Some(slot) = by_position.get_mut(frame.position) {
            *slot = Some(frame);
        }
    }
    let first_ordinary = segment.frames.iter().find(|frame| !frame.transparent);
    let mut judge = Judge {
        data,
        segment,
        stream: Stream::default(),
        opener_passes: first_ordinary
            .is_some_and(|frame| matches!(frame.link.state, LinkState::Linked | LinkState::Healed)),
        findings: Vec::new(),
        repairs: Vec::new(),
        protected_end: 0,
        judged: false,
    };
    for (position, item) in segment.items.iter().enumerate() {
        judge.item(position, item, by_position[position]);
    }
    judge.finish(budget)
}

impl<'s> Judge<'s> {
    /// Judge one item of the segment in item order.
    fn item(&mut self, position: usize, item: &WireItem, frame: Option<&WireFrame<'s>>) {
        if let Some(frame) = frame.filter(|frame| frame.frame_type == Some(PARITY)) {
            if let Some(judgement) = frame.judgement {
                self.judged = true;
                self.judge_parity(position, item, frame, judgement);
            }
        }
        if frame.is_some_and(|frame| frame.transparent)
            || matches!(item.kind, ItemKind::Offsets | ItemKind::Locator)
        {
            return;
        }
        let passes = if position == 0 {
            self.opener_passes
        } else {
            frame.is_some_and(passes)
        };
        self.stream
            .push(item.offset, item.end - item.offset, passes);
    }

    fn judge_parity(
        &mut self,
        position: usize,
        item: &WireItem,
        frame: &WireFrame<'s>,
        judgement: Judgement,
    ) {
        match judgement {
            Judgement::Damaged => self
                .findings
                .push(ParityFinding::at(ParityCode::ParityDamaged, position)),
            Judgement::ExceedsCapacity => {
                if let Some(finding) = exceeds_capacity(position, item, frame) {
                    self.findings.push(finding);
                }
            }
            Judgement::Unchecked => {}
            Judgement::DamagedUsed => {
                self.findings
                    .push(ParityFinding::at(ParityCode::ParityDamaged, position));
                self.use_span(position, item, frame);
            }
            Judgement::Verified => self.use_span(position, item, frame),
        }
    }

    /// Check the placement of a used span and, unless it is stale, its shards and its repair.
    fn use_span(&mut self, position: usize, item: &WireItem, frame: &WireFrame<'s>) {
        let Ok(envelope) = &frame.envelope else {
            return;
        };
        let Some(record) = Record::read(envelope.content) else {
            return;
        };
        let (Some(span), Ok(first), Ok(end)) = (
            record.span(),
            usize::try_from(record.first),
            usize::try_from(record.end),
        ) else {
            return;
        };
        let within =
            self.stream.last_start().is_some_and(|start| start < end) && end <= self.stream.end;
        if !within || (span < parity::SPAN && end != self.stream.end) {
            self.findings
                .push(ParityFinding::at(ParityCode::ParityStale, position));
            return;
        }
        let shape = Shape::of(span);
        let (Ok(size), Some(lists)) = (
            usize::try_from(shape.shard),
            shape.lists(envelope.stored_payload),
        ) else {
            return;
        };
        let span = Span {
            first,
            end,
            size,
            position,
            item: *item,
            covered: self.stream.covered(self.data, first, end),
            lists,
        };
        let failures = self.shard_failures(&span);
        if failures.stale {
            return;
        }
        self.protected_end = self.protected_end.max(end);
        if failures.data != failures.unexplained || !failures.parity.is_empty() {
            match self.repair(&span, &failures) {
                Some(repair) => self.repairs.push(repair),
                None => self.findings.push(ParityFinding {
                    reason: Some(ParityReason::TooManyErasures),
                    ..ParityFinding::at(ParityCode::RepairFailed, position)
                }),
            }
        }
    }

    /// Compare the shard digests of a span with its table and record the failures (P5 6.6,
    /// stage 3, and the explained and unexplained shards of P5 7.2).
    fn shard_failures(&mut self, span: &Span<'_>) -> ShardFailures {
        let mut data = BTreeSet::new();
        let mut unexplained = BTreeSet::new();
        for (index, digest) in span.lists.data_digests.iter().enumerate() {
            if shard_digest(&shard_at(&span.covered, index, span.size)) == *digest {
                continue;
            }
            data.insert(index);
            let start = span.first + index * span.size;
            if !self.stream.overlaps_failed(start, start + span.size) {
                unexplained.insert(index);
            }
        }
        let parity: BTreeSet<usize> = span
            .lists
            .parity_digests
            .iter()
            .enumerate()
            .filter(|(row, digest)| {
                shard_digest(&shard_at(span.lists.parity, *row, span.size)) != **digest
            })
            .map(|(row, _)| row)
            .collect();
        let position = span.position;
        if !data.is_empty() {
            self.findings.push(ParityFinding {
                reason: Some(ParityReason::Data),
                count: Some(count(data.len())),
                ..ParityFinding::at(ParityCode::ParityMismatch, position)
            });
        }
        if !parity.is_empty() {
            self.findings.push(ParityFinding {
                reason: Some(ParityReason::Parity),
                count: Some(count(parity.len())),
                ..ParityFinding::at(ParityCode::ParityMismatch, position)
            });
        }
        let stale = !data.is_empty() && data == unexplained;
        if stale {
            self.findings
                .push(ParityFinding::at(ParityCode::ParityStale, position));
        } else if !unexplained.is_empty() {
            self.findings.push(ParityFinding {
                count: Some(count(unexplained.len())),
                ..ParityFinding::at(ParityCode::ParityUnexplained, position)
            });
        }
        ShardFailures {
            data,
            parity,
            unexplained,
            stale,
        }
    }

    /// Solve the erased shards of a span and build its repair (P5 7.2, step 5, substep 1).
    ///
    /// Returns `None` when a stripe holds more unknowns than its parity rows.
    fn repair(&self, span: &Span<'_>, failures: &ShardFailures) -> Option<Repair> {
        let solved: Solved = parity_shards::solve(
            &span.covered,
            span.lists.parity,
            span.size,
            &failures.data,
            &failures.parity,
        )?;
        let mut digests_hold = true;
        let mut data = Vec::new();
        for (index, shard) in &solved.data {
            digests_hold &= span
                .lists
                .data_digests
                .get(*index)
                .is_some_and(|digest| shard_digest(shard) == *digest);
            if failures.unexplained.contains(index) {
                continue;
            }
            let start = span.first + index * span.size;
            let end = (start + span.size).min(span.end);
            data.extend(self.stream.patches(start, end, shard));
        }
        let parity = self.parity_patches(span, &solved)?;
        digests_hold &= parity.holds;
        let written =
            solved.data.len().saturating_sub(failures.unexplained.len()) + solved.parity.len();
        Some(Repair {
            position: span.position,
            count: count(written),
            digests_hold,
            data,
            parity: parity.patches,
        })
    }

    /// The patches of the solved parity shards, and whether each matches its digest.
    fn parity_patches(&self, span: &Span<'_>, solved: &Solved) -> Option<ParityPatches> {
        let offsets = field_offsets(&self.data[..span.item.end], span.item.offset).ok()?;
        let parity_start = offsets[8].checked_sub(span.lists.parity.len())?;
        let mut holds = true;
        let mut patches = Vec::with_capacity(solved.parity.len());
        for (row, shard) in &solved.parity {
            holds &= span
                .lists
                .parity_digests
                .get(*row)
                .is_some_and(|digest| shard_digest(shard) == *digest);
            patches.push(Patch {
                offset: parity_start + row * span.size,
                data: shard.clone(),
            });
        }
        Some(ParityPatches { holds, patches })
    }

    fn finish(mut self, budget: &mut PayloadBudget) -> Verification {
        let repairs = std::mem::take(&mut self.repairs);
        let mut patches = Vec::new();
        if !repairs.is_empty() {
            let rejected = parity_runs::rejected(self.data, self.segment, &repairs, budget);
            for repair in repairs {
                if rejected.contains(&repair.position) {
                    self.findings.push(ParityFinding {
                        reason: Some(ParityReason::VerifyFailed),
                        ..ParityFinding::at(ParityCode::RepairFailed, repair.position)
                    });
                } else {
                    self.findings.push(ParityFinding {
                        count: Some(repair.count),
                        ..ParityFinding::at(ParityCode::Repaired, repair.position)
                    });
                    patches.extend(repair.data);
                    patches.extend(repair.parity);
                }
            }
        }
        self.unprotected_tail();
        Verification {
            findings: self.findings,
            patches,
        }
    }

    /// The bytes after the largest protected end, when a frame of the segment was judged (P5 6.7).
    fn unprotected_tail(&mut self) {
        let opener_length = self.stream.parts.first().map_or(0, |part| part.length);
        let unprotected = self.protected_end.max(opener_length);
        let end = self.stream.end;
        if !self.judged || unprotected >= end {
            return;
        }
        let index = self
            .stream
            .parts
            .partition_point(|part| part.stream <= unprotected)
            .saturating_sub(1);
        if let Some(part) = self.stream.parts.get(index) {
            self.findings.push(ParityFinding {
                offset: Some(part.offset + unprotected - part.stream),
                length: Some(end - unprotected),
                ..ParityFinding::bare(ParityCode::UnprotectedTail)
            });
        }
    }
}

/// The solved parity patches of a span, with whether each of their digests holds.
struct ParityPatches {
    holds: bool,
    patches: Vec<Patch>,
}

/// Whether an item passes its checks: a frame that is linked, and whose payload or stored check
/// holds (P5 6.6, passing items).
fn passes(frame: &WireFrame<'_>) -> bool {
    let Ok(envelope) = &frame.envelope else {
        return false;
    };
    if frame.link.state != super::reader::LinkState::Linked {
        return false;
    }
    if envelope.storage == super::frame::Storage::Plain {
        matches!(frame.payload.state, PayloadState::Verified(_))
    } else {
        frame.payload.stored_matches == Some(true)
    }
}

/// The `exceeds_capacity` finding of a parity frame whose payload a size limit refused.
fn exceeds_capacity(
    position: usize,
    item: &WireItem,
    frame: &WireFrame<'_>,
) -> Option<ParityFinding> {
    let PayloadState::Unchecked(refusal) = frame.payload.state else {
        return None;
    };
    let (reason, count) = match refusal {
        PayloadRefusal::StoredSize(count) => (ParityReason::StoredSize, count),
        PayloadRefusal::DecodedSize(count) => (ParityReason::DecodedSize, count),
        PayloadRefusal::TotalDecoded(count) => (ParityReason::TotalDecoded, count),
        PayloadRefusal::UnknownCodec => return None,
    };
    Some(ParityFinding {
        reason: Some(reason),
        count: Some(count),
        offset: Some(item.offset),
        length: Some(item.end - item.offset),
        ..ParityFinding::at(ParityCode::ExceedsCapacity, position)
    })
}

fn count(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}
