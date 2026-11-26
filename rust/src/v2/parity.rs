// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The parity candidate of the resumption scan (P5 §7.2, step 6).

use std::collections::HashSet;

use super::{
    frame::{decode_parity_candidate, frame_identifier, Frame, Storage, PARITY},
    payload::{ChargeRefusal, PayloadBudget, PayloadRefusal, PayloadState},
    reader::LinkState,
    Decoder,
};

/// The envelope bytes that begin every parity frame: the array of eleven, the item kind, the
/// frame type, `critical` false and `transparent` true.
const PREFIX: [u8; 5] = [0x8b, 0x02, 0x10, 0xf4, 0xf5];
const SPAN: u64 = 1 << 22;
const FULL_SHARD: u64 = 4096;
const MIN_SHARD: u64 = 64;
const COLUMNS: u64 = 64;
const ROWS: u64 = 4;
const DIGEST: u64 = 17;

/// The first offset at or after `from` where the bytes of a parity envelope begin.
pub(super) fn next_prefix(data: &[u8], from: usize) -> Option<usize> {
    data.get(from..)?
        .windows(PREFIX.len())
        .position(|window| window == PREFIX)
        .map(|found| found + from)
}

/// The inputs of a candidate test that come from the segment being read.
pub(super) struct Segment<'a> {
    /// The identifier of the segment's opener.
    pub h0: &'a [u8; 32],
    /// The registry version of the segment.
    pub registry: u64,
    /// The earlier parity frames of the segment, as condition 6 of P5 6.6 compares them.
    pub earlier: &'a Earlier,
}

/// What condition 6 of P5 6.6 keeps of the earlier parity frames of a segment: the `first` of
/// each, and whether any has a short span. A candidate is compared with this and not with each
/// frame, so the test of a candidate costs the same however many frames stand before it.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(super) struct Earlier {
    firsts: HashSet<u64>,
    short_seen: bool,
}

impl Earlier {
    /// Record the parity frame whose content record is `content`, read before a damaged region.
    ///
    /// The frame counts whatever its own state (damaged, above capacity or used). It counts
    /// only when its content record is well typed and its link state is not `broken`, which
    /// the caller decides. In a segment of a later registry version, a frame whose envelope
    /// is as the type states (`clean`: `critical`, `transparent`, `storage` and `signatures`
    /// as the type holds them, whatever else of the envelope fails) and whose `geometry` is
    /// above `1` is a frame of unknown type that condition 6 does not see (P5 6.6, the
    /// paragraph on a later version); a frame whose envelope is not as the type states is
    /// damaged and counts.
    pub(super) fn remember(&mut self, content: &[u8], newer: bool, clean: bool) {
        let Some(record) = Record::read(content) else {
            return;
        };
        if newer && clean && record.geometry > 1 {
            return;
        }
        self.firsts.insert(record.first);
        self.short_seen |= record
            .end
            .checked_sub(record.first)
            .is_some_and(|length| (1..SPAN).contains(&length));
    }

    fn admits(&self, first: u64) -> bool {
        !self.short_seen && !self.firsts.contains(&first)
    }
}

/// The outcome of the test of one parity candidate.
pub(super) enum Verdict {
    /// The candidate passes every part of the test; reading resumes at it.
    Passed {
        /// The `anchor` of the frame, the predecessor of the frame that follows the region.
        anchor: [u8; 32],
    },
    /// The candidate fails a part of the test, or the total refused it without a finding.
    Failed,
    /// The charge of the candidate is the first that the total decoded length refuses.
    Refused {
        /// The length of the candidate item in bytes.
        length: usize,
        /// The total kept so far plus the `decoded_length` of the candidate.
        count: u64,
    },
}

fn shard_length(span: u64) -> u64 {
    if span >= COLUMNS * FULL_SHARD {
        FULL_SHARD
    } else {
        span.div_ceil(COLUMNS).next_power_of_two().max(MIN_SHARD)
    }
}

/// The size of the head of a definite-length item with `count` as its argument.
fn head_size(count: u64) -> u64 {
    match count {
        0..=23 => 1,
        24..=0xff => 2,
        0x100..=0xffff => 3,
        0x1_0000..=0xffff_ffff => 5,
        _ => 9,
    }
}

/// The shape of a `parity_payload` for a span of `span` bytes (P5 §6.4).
struct Shape {
    shards: u64,
    parity_shards: u64,
    parity_bytes: u64,
}

impl Shape {
    fn of(span: u64) -> Self {
        let shard = shard_length(span);
        let shards = span.div_ceil(shard);
        let parity_shards = ROWS * shards.div_ceil(COLUMNS);
        Self {
            shards,
            parity_shards,
            parity_bytes: parity_shards * shard,
        }
    }

    /// The length of the deterministic encoding of the payload.
    fn encoded_length(&self) -> u64 {
        1 + head_size(self.shards)
            + DIGEST * self.shards
            + head_size(self.parity_shards)
            + DIGEST * self.parity_shards
            + head_size(self.parity_bytes)
            + self.parity_bytes
    }

    /// The end of the two digest lists when `payload` is one deterministic `parity_payload` of
    /// this shape.
    fn lists_end(&self, payload: &[u8]) -> Option<usize> {
        let mut cursor = Decoder::new(payload, 0);
        cursor.record(3).ok()?;
        for count in [self.shards, self.parity_shards] {
            if cursor.head(4).ok()? != count {
                return None;
            }
            for _ in 0..count {
                cursor.fixed::<16>().ok()?;
            }
        }
        let end = cursor.offset;
        cursor
            .byte_string(Some(usize::try_from(self.parity_bytes).ok()?))
            .ok()?;
        (cursor.offset == payload.len()).then_some(end)
    }

    /// Whether `payload` is one deterministic `parity_payload` of this shape whose table digest
    /// under `h0` is `table_digest` (P5 6.6, conditions 4 and 5).
    fn table_matches(&self, payload: &[u8], h0: &[u8; 32], table_digest: &[u8; 32]) -> bool {
        let Some(lists_end) = self.lists_end(payload) else {
            return false;
        };
        let mut table = blake3::Hasher::new_derive_key("gts2 parity table");
        table.update(&[0x83, 0x58, 0x20]);
        table.update(h0);
        table.update(&payload[1..lists_end]);
        table.finalize().as_bytes() == table_digest
    }
}

/// The content record of a parity frame.
struct Record<'a> {
    geometry: u64,
    first: u64,
    end: u64,
    anchor: &'a [u8; 32],
    table_digest: &'a [u8; 32],
}

impl<'a> Record<'a> {
    fn read(content: &'a [u8]) -> Option<Self> {
        let mut cursor = Decoder::new(content, 0);
        cursor.record(5).ok()?;
        Some(Self {
            geometry: cursor.uint().ok()?,
            first: cursor.uint().ok()?,
            end: cursor.uint().ok()?,
            anchor: cursor.fixed::<32>().ok()?,
            table_digest: cursor.fixed::<32>().ok()?,
        })
    }

    /// The length of the span when the record is within condition 1 of P5 §6.6.
    fn span(&self) -> Option<u64> {
        let length = self.end.checked_sub(self.first)?;
        (self.geometry == 1 && self.first.is_multiple_of(SPAN) && (1..=SPAN).contains(&length))
            .then_some(length)
    }
}

/// The stage 1 outcome of an in-order frame of type `parity` (P5 6.6, stage 1).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Judgement {
    /// Condition 1, 2, 3, 6 or 7 of P5 6.6 holds, or the payload check ran and failed (conditions
    /// 4 and 5): `parity_damaged`. The frame is not used.
    Damaged,
    /// The frame is damaged by none of those conditions, and its payload exceeds the reader's
    /// capacity (`exceeds_capacity`). The payload was not read, so the frame is not used.
    ExceedsCapacity,
    /// The frame is damaged by none of those conditions, and its payload uses a codec this reader
    /// does not implement, so the payload was not checked and nothing is verified.
    Unchecked,
    /// The frame passes stage 1 and its payload is verified.
    Verified,
}

/// Judge an in-order frame of type `parity` by stage 1 of P5 6.6, in item order.
///
/// `envelope` is `None` when the envelope does not decode; such a frame is damaged, since the
/// envelope is part of condition 7. The earlier frames of `segment` must not yet include `frame`.
/// Returns `None` for a frame that the paragraph on a later registry version hands to the
/// reader's unknown-type handling, which this function does not judge.
pub(super) fn judge(
    envelope: Option<&Frame<'_>>,
    link: LinkState,
    predecessor: &[u8; 32],
    payload: &PayloadState<'_>,
    segment: &Segment<'_>,
    newer: bool,
) -> Option<Judgement> {
    let Some(frame) = envelope else {
        return Some(Judgement::Damaged);
    };
    let clean = !frame.critical
        && frame.transparent
        && frame.storage == Storage::Plain
        && frame.signatures.is_empty();
    let Some(record) = Record::read(frame.content) else {
        return Some(Judgement::Damaged);
    };
    if newer && clean && record.geometry > 1 {
        return None;
    }
    let Some(span) = record.span() else {
        return Some(Judgement::Damaged);
    };
    if !clean
        || link != LinkState::Linked
        || record.anchor != predecessor
        || !segment.earlier.admits(record.first)
    {
        return Some(Judgement::Damaged);
    }
    let shape = Shape::of(span);
    Some(match payload {
        PayloadState::Damaged(_) => Judgement::Damaged,
        PayloadState::Unchecked(PayloadRefusal::UnknownCodec) => Judgement::Unchecked,
        PayloadState::Unchecked(_) => Judgement::ExceedsCapacity,
        PayloadState::Verified(_) => {
            let expected = shape.encoded_length();
            let fits =
                frame.stored_payload.len() as u64 == expected && frame.decoded_length == expected;
            if fits && shape.table_matches(frame.stored_payload, segment.h0, record.table_digest) {
                Judgement::Verified
            } else {
                Judgement::Damaged
            }
        }
    })
}

/// Test the parity candidate at `offset` by the four parts of P5 §7.2, step 6.
///
/// A candidate that passes has been charged to the total decoded length of `budget`, and its
/// charge is not made again when the item is read.
pub(super) fn test(
    data: &[u8],
    offset: usize,
    segment: &Segment<'_>,
    budget: &mut PayloadBudget,
) -> Verdict {
    let Ok((frame, _)) = decode_parity_candidate(data, offset, segment.registry) else {
        return Verdict::Failed;
    };
    let Some(record) = Record::read(frame.content).filter(|_| frame.frame_type == PARITY) else {
        return Verdict::Failed;
    };
    let Some(span) = record.span() else {
        return Verdict::Failed;
    };
    if !segment.earlier.admits(record.first) {
        return Verdict::Failed;
    }
    let shape = Shape::of(span);
    let expected = shape.encoded_length();
    if frame.stored_payload.len() as u64 != expected || frame.decoded_length != expected {
        return Verdict::Failed;
    }
    if frame_identifier(record.anchor, frame.identity_fields) != *frame.identifier {
        return Verdict::Failed;
    }
    match budget.charge_candidate(expected, expected) {
        Ok(()) => {}
        Err(ChargeRefusal::Frame | ChargeRefusal::Total(None)) => return Verdict::Failed,
        Err(ChargeRefusal::Total(Some(count))) => {
            return Verdict::Refused {
                length: frame.next_offset - offset,
                count,
            };
        }
    }
    if !frame.check_decoded_payload(frame.stored_payload) {
        return Verdict::Failed;
    }
    if !shape.table_matches(frame.stored_payload, segment.h0, record.table_digest) {
        return Verdict::Failed;
    }
    budget.prepay(expected);
    Verdict::Passed {
        anchor: *record.anchor,
    }
}
