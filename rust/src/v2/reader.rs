// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Whole-input framing and frame observations for the GTS v2 reader.

use std::collections::VecDeque;

use super::{
    decode_header,
    durability::{check_closing, check_closing_grammar, check_prefixes},
    frame::{
        check_type_length, decode_frame_envelope, frame_identifier, Frame, FrameError, PARITY,
    },
    framing::{byte_chunks, generation, item_end, read_head, Generation},
    payload::{malformed_payload, PayloadBudget},
    resync::{decode_resync, Candidate, ResyncScanner, RESYNC_MAX},
    DecodedHeader, Decoder, HeaderError, MalformedReason, REGISTRY_VERSION,
};

pub use super::{
    durability::{ClosingFinding, CommitCheck, CommitReadError, CommitReport, UncommittedTail},
    framing::{FramingFailure, FramingReason},
    index::{IndexCheck, IndexOutcome, IndexWireClaims},
    payload::{PayloadCheck, PayloadDamage, PayloadRefusal, PayloadState, ReadCapacity},
};

pub use super::resync::DecodedResync;

/// The first item determines which generation reads the file.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Dispatch {
    /// The file has no items.
    Empty,
    /// The file uses the first generation.
    FirstGeneration,
    /// The file uses the second generation.
    SecondGeneration,
    /// The file uses a generation that this reader does not implement.
    UnsupportedGeneration(u64),
    /// The file is not in this format.
    NotThisFormat,
}

/// Why reading stopped before the end of the input.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReadStop {
    /// The framing of the input failed.
    Framing(FramingFailure),
    /// The item limit was reached at an item.
    ItemLimit {
        /// The offset of the item where the limit was reached.
        offset: usize,
    },
}

/// The kind of an item in a segment.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ItemKind {
    /// A header item.
    Header,
    /// A resync item.
    Resync,
    /// A frame item.
    Frame,
    /// A locator item.
    Locator,
    /// An offsets item.
    Offsets,
    /// A malformed item, with the reason it is malformed.
    Malformed(MalformedReason),
    /// An item with an unknown kind code.
    Unknown(u64),
    /// An item that was not checked.
    Unchecked,
    /// A region of the input that is damaged.
    DamagedRegion,
}

/// An item of a segment and the bytes it covers.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WireItem {
    /// The kind of the item.
    pub kind: ItemKind,
    /// The offset of the first byte of the item.
    pub offset: usize,
    /// The offset just after the last byte of the item.
    pub end: usize,
}

/// The opener of a segment, either a header or a resync item.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Opener<'a> {
    /// A header opener and the result of decoding it.
    Header(Result<DecodedHeader<'a>, HeaderError>),
    /// A resync opener and the result of decoding it.
    Resync(Result<DecodedResync<'a>, HeaderError>),
}

impl Opener<'_> {
    fn state(&self) -> (u64, Option<[u8; 32]>) {
        match self {
            Self::Header(Ok(value)) => (value.header.registry_version, value.identifier),
            Self::Resync(Ok(value)) => (value.header.registry_version, value.identifier),
            _ => (REGISTRY_VERSION, None),
        }
    }
}

/// Whether the link of a frame to its predecessor is intact.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LinkState {
    /// The link matches the predecessor.
    Linked,
    /// The link does not match, but the next frame confirms the computed identifier.
    Healed,
    /// The link does not match and is not confirmed.
    Broken,
}

/// The link of a frame to its predecessor.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FrameLink {
    /// The link state.
    pub state: LinkState,
    /// The identifier computed from the frame, if it can be computed.
    pub computed_identifier: Option<[u8; 32]>,
    /// The identifier that the link is checked against, if any.
    pub effective_identifier: Option<[u8; 32]>,
}

/// A frame of a segment as read from the wire.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WireFrame<'a> {
    /// The position of the frame in its segment.
    pub position: usize,
    /// The registry code of the frame type, if it can be read.
    pub frame_type: Option<u64>,
    /// Whether the frame is transparent to readers of its content.
    pub transparent: bool,
    /// The parsed envelope, or the error that stopped parsing it.
    pub envelope: Result<Frame<'a>, FrameError>,
    /// The link of the frame to its predecessor.
    pub link: FrameLink,
    /// The payload check of the frame.
    pub payload: PayloadCheck<'a>,
    /// A later frame type with well-typed flags that stops content folding.
    pub unknown_type_stops_fold: bool,
}

/// A segment of the input as read from the wire.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WireSegment<'a> {
    /// The opener of the segment.
    pub opener: Opener<'a>,
    /// The items of the segment, in input order.
    pub items: Vec<WireItem>,
    /// The frames of the segment, in input order.
    pub frames: Vec<WireFrame<'a>>,
    /// The head identifier of the segment, if one can be read.
    pub head: Option<[u8; 32]>,
    /// The declared segment boundary can fall inside its last item after recovery.
    pub end_offset: usize,
    /// Whether the segment was scanned to its end.
    pub scanned: bool,
    /// The framing failure that ended the segment, if any.
    pub framing_failure: Option<FramingFailure>,
    /// The reason a resync item was refused, if any.
    pub resync_refusal: Option<ResyncRefusal>,
    /// Grammar errors observed during the framing test.
    pub closing_grammar: Vec<(usize, HeaderError)>,
    /// Closing findings after placement and claims are checked over retained items.
    pub closing_findings: Vec<ClosingFinding>,
    /// The commit report of the segment, or the error that prevented it.
    pub commits: Result<CommitReport, CommitReadError>,
    /// The index checks of the segment.
    pub indexes: Vec<IndexCheck>,
}

/// Why a resync item was refused.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResyncRefusal {
    /// The position of the refused resync item.
    pub position: usize,
    /// The offset that the refused resync item claims.
    pub offset: usize,
    /// The length of the refused span, in bytes.
    pub length: usize,
    /// The number of items counted in the refused span.
    pub count: u64,
}

/// The wire-level read of a whole input.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct WireRead<'a> {
    /// Generation dispatch after checking for a resync opener in the first item.
    pub dispatch: Option<Dispatch>,
    /// The segments of the input, in file order.
    pub segments: Vec<WireSegment<'a>>,
    /// The reason reading stopped before the end of the input, if it did.
    pub stop: Option<ReadStop>,
    /// Framing failure before the first accepted opener.
    pub prefix_failure: Option<FramingFailure>,
    /// First candidate refused before the read has a segment.
    pub resync_refusal: Option<ResyncRefusal>,
}

fn classify(data: &[u8], offset: usize, newer: bool) -> ItemKind {
    use ItemKind::{Frame, Header, Locator, Malformed, Offsets, Resync};
    use MalformedReason::{NotDeterministic, UnregisteredValue, WrongLength, WrongType};
    let Ok(array) = read_head(data, offset) else {
        return Malformed(WrongType);
    };
    if array.major != 4 || array.argument == Some(0) {
        return Malformed(WrongType);
    }
    let Ok(kind) = read_head(data, array.end) else {
        return Malformed(WrongType);
    };
    let Some(code) = kind.argument.filter(|_| kind.major == 0) else {
        return Malformed(WrongType);
    };
    match code {
        1 => Header,
        5 => Resync,
        _ if !array.shortest || !kind.shortest => Malformed(NotDeterministic),
        0 => Malformed(UnregisteredValue),
        2 if array.argument == Some(11) => Frame,
        2 => Malformed(WrongLength),
        3 => Locator,
        4 => Offsets,
        _ if newer => ItemKind::Unknown(code),
        _ => Malformed(UnregisteredValue),
    }
}

fn opener(data: &[u8], item: WireItem) -> WireSegment<'_> {
    let opener = if item.kind == ItemKind::Header {
        Opener::Header(decode_header(&data[..item.end], item.offset))
    } else {
        Opener::Resync(decode_resync(data, item.offset, item.end))
    };
    segment_at(opener, item, false)
}

fn segment_at(opener: Opener<'_>, item: WireItem, scanned: bool) -> WireSegment<'_> {
    let (_, head) = opener.state();
    WireSegment {
        opener,
        items: vec![item],
        frames: Vec::new(),
        head,
        end_offset: item.end,
        scanned,
        framing_failure: None,
        resync_refusal: None,
        closing_grammar: Vec::new(),
        closing_findings: Vec::new(),
        commits: Ok(CommitReport::default()),
        indexes: Vec::new(),
    }
}

fn recovered_segment(offset: usize, value: DecodedResync<'_>) -> WireSegment<'_> {
    segment_at(
        Opener::Resync(Ok(value)),
        WireItem {
            kind: ItemKind::Resync,
            offset,
            end: value.next_offset,
        },
        true,
    )
}

impl WireSegment<'_> {
    fn with_claims(mut self, data: &[u8], stored_limit: u64) -> Self {
        let (registry, identifier) = self.opener.state();
        if let Some(identifier) = identifier {
            self.closing_findings = check_closing(data, &self.items, &self.frames);
            match check_prefixes(&identifier, &self.frames, stored_limit, registry) {
                Ok((commits, indexes)) => {
                    self.commits = Ok(commits);
                    self.indexes = indexes;
                }
                Err(error) => self.commits = Err(error),
            }
        }
        self
    }
}

#[derive(Clone, Copy)]
struct Boundary {
    offset: usize,
    end: usize,
    failure: Option<FramingReason>,
}

struct ItemWalk<'a> {
    data: &'a [u8],
    offset: usize,
    count: u64,
    limit: u64,
    held: VecDeque<Boundary>,
}

impl<'a> ItemWalk<'a> {
    fn new(data: &'a [u8], limit: u64) -> Self {
        Self {
            data,
            offset: 0,
            count: 0,
            limit,
            held: VecDeque::new(),
        }
    }

    fn delimit_to(&mut self, end: usize) -> Option<Boundary> {
        let offset = self.offset;
        if offset >= end || self.count == self.limit {
            return None;
        }
        let (end, failure) = match item_end(&self.data[..end], offset) {
            Ok(next) => (next, None),
            Err(FramingReason::TornTail) if end < self.data.len() => return None,
            Err(reason) => (self.data.len(), Some(reason)),
        };
        if failure != Some(FramingReason::TornTail) {
            self.count += 1;
        }
        self.offset = end;
        Some(Boundary {
            offset,
            end,
            failure,
        })
    }

    fn take(&mut self) -> Option<Boundary> {
        self.held
            .pop_front()
            .or_else(|| self.delimit_to(self.data.len()))
    }

    fn peek(&mut self) -> Option<Boundary> {
        if self.held.is_empty() {
            if let Some(next) = self.delimit_to(self.data.len()) {
                self.held.push_back(next);
            }
        }
        self.held.front().copied()
    }

    fn recover(
        &mut self,
        scanner: &mut ResyncScanner<'a>,
        start: usize,
        lost: Boundary,
        budget: &mut PayloadBudget,
    ) -> Option<Candidate<'a>> {
        if lost.failure.is_none() && self.peek().is_some() {
            let end = lost.end.saturating_add(RESYNC_MAX).min(self.data.len());
            while let Some(boundary) = self.delimit_to(end) {
                self.held.push_back(boundary);
            }
        }
        if self.count == self.limit {
            return None;
        }
        let outcome = scanner.scan(start, lost.end, budget);
        if let Some(Candidate::Accepted { value, .. }) = outcome.as_ref() {
            self.count += 1;
            self.held.clear();
            self.offset = value.next_offset;
        }
        outcome
    }

    fn item_limit(&self) -> Option<ReadStop> {
        (self.count == self.limit && self.offset < self.data.len()).then_some(ReadStop::ItemLimit {
            offset: self.offset,
        })
    }
}

fn field_offsets(data: &[u8], offset: usize) -> Result<[usize; 11], FramingReason> {
    let mut offsets = [offset + 2; 11];
    for index in 1..offsets.len() {
        offsets[index] = item_end(data, offsets[index - 1])?;
    }
    Ok(offsets)
}

fn stored_identifier(data: &[u8], offset: usize) -> Option<[u8; 32]> {
    let mut result = [0; 32];
    let mut used = 0_usize;
    for chunk in byte_chunks(data, offset)? {
        let end = used.checked_add(chunk.len())?;
        result.get_mut(used..end)?.copy_from_slice(chunk);
        used = end;
    }
    (used == 32).then_some(result)
}

#[derive(Clone, Copy)]
struct LinkFields<'a> {
    transparent: bool,
    identity: Option<&'a [u8]>,
    stored: Option<[u8; 32]>,
}

impl<'a> LinkFields<'a> {
    fn inspect(data: &'a [u8], offsets: &[usize; 11]) -> Self {
        let mut decoder = Decoder::new(data, offsets[0]);
        Self {
            transparent: data[offsets[2]] == 0xf5,
            identity: decoder
                .frame_identity(None)
                .ok()
                .map(|fields| fields.encoded),
            stored: stored_identifier(data, offsets[8]),
        }
    }

    fn computed(self, predecessor: &[u8; 32]) -> Option<[u8; 32]> {
        self.identity
            .map(|encoded| frame_identifier(predecessor, encoded))
    }

    fn matches(self, predecessor: &[u8; 32]) -> bool {
        self.stored.is_some() && self.computed(predecessor) == self.stored
    }
}

fn read_frame<'a>(
    data: &'a [u8],
    item: WireItem,
    segment: &WireSegment<'_>,
    predecessor: &[u8; 32],
    walk: &mut ItemWalk<'a>,
    budget: &mut PayloadBudget,
) -> Result<(WireFrame<'a>, bool), FramingReason> {
    let (registry, _) = segment.opener.state();
    let envelope = decode_frame_envelope(data, item.offset, registry);
    let malformed_grammar = envelope.is_err();
    let mut offsets = None;
    let (frame_type, fields) = if let Ok((frame, _)) = &envelope {
        (
            Some(frame.frame_type),
            LinkFields {
                transparent: frame.transparent,
                identity: Some(frame.identity_fields),
                stored: Some(*frame.identifier),
            },
        )
    } else {
        let offsets = offsets.insert(field_offsets(data, item.offset)?);
        let frame_type = Decoder::new(data, offsets[0]).uint().ok();
        (frame_type, LinkFields::inspect(data, offsets))
    };
    let computed = fields.computed(predecessor);
    let state = if computed.is_some() && computed == fields.stored {
        LinkState::Linked
    } else if let Some(identifier) =
        computed.filter(|_| !fields.transparent && fields.stored.is_some())
    {
        let following = walk.peek().filter(|next| {
            next.failure.is_none() && data[next.offset..next.end].starts_with(&[0x8b, 0x02])
        });
        match following {
            Some(next)
                if LinkFields::inspect(data, &field_offsets(data, next.offset)?)
                    .matches(&identifier) =>
            {
                LinkState::Healed
            }
            _ => LinkState::Broken,
        }
    } else {
        LinkState::Broken
    };
    let effective_identifier = if fields.transparent {
        None
    } else if state == LinkState::Healed {
        computed
    } else {
        fields.stored.or(computed)
    };
    let payload = if let Ok((frame, _)) = &envelope {
        budget.check(&frame.into(), frame.check_storage())
    } else {
        let offsets = offsets.ok_or(FramingReason::UndelimitableInput)?;
        let (stored_matches, input) =
            malformed_payload(data, &offsets, registry > REGISTRY_VERSION);
        match input {
            Ok(input) => budget.check(&input, stored_matches),
            Err(_) => PayloadCheck {
                stored_matches,
                state: PayloadState::Damaged(PayloadDamage::MalformedFields),
            },
        }
    };
    let unknown_type_stops_fold = registry > REGISTRY_VERSION
        && frame_type.is_some_and(|kind| kind > PARITY)
        && match &envelope {
            Ok((frame, _)) => frame.critical,
            Err(_) => offsets
                .is_some_and(|raw| data[raw[1]] == 0xf5 && matches!(data[raw[2]], 0xf4 | 0xf5)),
        };
    let envelope =
        envelope.and_then(|(frame, length_offset)| check_type_length(frame, length_offset));
    Ok((
        WireFrame {
            position: segment.items.len(),
            frame_type,
            transparent: fields.transparent,
            envelope,
            link: FrameLink {
                state,
                computed_identifier: computed,
                effective_identifier,
            },
            payload,
            unknown_type_stops_fold,
        },
        malformed_grammar || state == LinkState::Broken,
    ))
}

fn damage_before(data: &[u8], start: usize, end: usize) -> Option<FramingFailure> {
    if start >= end {
        return None;
    }
    item_end(&data[..end], start)
        .err()
        .map(|reason| FramingFailure {
            reason,
            offset: start,
            length: end - start,
        })
}

fn first_segment<'a>(
    data: &'a [u8],
    walk: &mut ItemWalk<'a>,
    scanner: &mut ResyncScanner<'a>,
    budget: &mut PayloadBudget,
    result: &mut WireRead<'a>,
) -> Option<WireSegment<'a>> {
    if data.is_empty() {
        result.dispatch = Some(Dispatch::Empty);
        return None;
    }
    let Some(first) = walk.take() else {
        result.stop = walk.item_limit();
        return None;
    };
    let mut scope = None;
    if let Some(reason) = first.failure {
        result.stop = Some(ReadStop::Framing(FramingFailure {
            reason,
            offset: 0,
            length: first.end,
        }));
    } else {
        let dispatch = match generation(&data[..first.end]) {
            Some(Generation::Record(2)) => Dispatch::SecondGeneration,
            Some(Generation::Legacy(1)) => Dispatch::FirstGeneration,
            Some(Generation::Legacy(version) | Generation::Record(version @ 3..)) => {
                Dispatch::UnsupportedGeneration(version)
            }
            _ => Dispatch::NotThisFormat,
        };
        result.dispatch = Some(dispatch);
        match dispatch {
            Dispatch::SecondGeneration => {
                let kind = classify(&data[..first.end], 0, false);
                let segment = opener(
                    data,
                    WireItem {
                        kind,
                        offset: 0,
                        end: first.end,
                    },
                );
                if !unreadable(segment.opener) {
                    return Some(segment);
                }
                scope = Some(segment);
            }
            Dispatch::NotThisFormat => {}
            _ => return None,
        }
    }
    match walk.recover(scanner, 0, first, budget) {
        Some(Candidate::Accepted { offset, value }) => {
            result.dispatch = Some(Dispatch::SecondGeneration);
            result.stop = None;
            result.prefix_failure = damage_before(data, 0, offset);
            return Some(recovered_segment(offset, value));
        }
        Some(Candidate::Refused { offset, count }) => {
            let refusal = ResyncRefusal {
                position: 0,
                offset,
                length: RESYNC_MAX,
                count,
            };
            if let Some(segment) = &mut scope {
                segment.resync_refusal = Some(refusal);
            } else {
                result.resync_refusal = Some(refusal);
            }
        }
        None => {}
    }
    scope
}

fn unreadable(opener: Opener<'_>) -> bool {
    matches!(opener, Opener::Header(Err(_)) | Opener::Resync(Err(_)))
}

fn end_at_resync(
    segment: &mut WireSegment<'_>,
    data: &[u8],
    lost: Boundary,
    offset: usize,
) -> Result<(), FramingReason> {
    if let Some(frame) = segment.frames.last_mut().filter(|frame| {
        frame.position + 1 == segment.items.len() && frame.link.state == LinkState::Healed
    }) {
        let last_item = segment.items[frame.position];
        let stored = stored_identifier(data, field_offsets(data, last_item.offset)?[8]);
        frame.link.state = LinkState::Broken;
        frame.link.effective_identifier = stored;
        segment.head = stored;
    }
    segment.end_offset = offset;
    segment.framing_failure = damage_before(data, lost.offset, offset);
    if let Some(failure) = segment.framing_failure {
        match failure.reason {
            FramingReason::TornTail => segment.end_offset = failure.offset,
            FramingReason::UndelimitableInput => segment.items.push(WireItem {
                kind: ItemKind::DamagedRegion,
                offset: failure.offset,
                end: offset,
            }),
        }
    }
    Ok(())
}

fn framing_stop(
    segment: &mut WireSegment<'_>,
    reason: FramingReason,
    offset: usize,
    end: usize,
) -> ReadStop {
    let failure = FramingFailure {
        reason,
        offset,
        length: end - offset,
    };
    segment.framing_failure = Some(failure);
    ReadStop::Framing(failure)
}

/// What the classification of one walk item found.
struct Classified<'a> {
    /// Whether the item cannot continue the current segment.
    lost: bool,
    /// The segment begun by a header or resync opener item.
    begins: Option<WireSegment<'a>>,
    /// The frame read from a frame item.
    observed: Option<WireFrame<'a>>,
    /// The closing-grammar error of an offsets or locator item.
    closing_error: Option<HeaderError>,
}

/// Classify the item that `boundary` delimits within the current segment.
///
/// # Errors
///
/// Returns the [`FramingReason`] when a frame item cannot be read.
fn classify_boundary<'a>(
    data: &'a [u8],
    segment: &WireSegment<'a>,
    walk: &mut ItemWalk<'a>,
    budget: &mut PayloadBudget,
    boundary: Boundary,
) -> Result<(WireItem, Classified<'a>), FramingReason> {
    let mut item = WireItem {
        kind: ItemKind::Unchecked,
        offset: boundary.offset,
        end: boundary.end,
    };
    let mut classified = Classified {
        lost: boundary.failure.is_some(),
        begins: None,
        observed: None,
        closing_error: None,
    };
    if !classified.lost {
        let newer = segment.head.is_none() || segment.opener.state().0 > REGISTRY_VERSION;
        item.kind = classify(&data[..item.end], item.offset, newer);
        classified.lost = match item.kind {
            ItemKind::Header | ItemKind::Resync => {
                let next = classified.begins.insert(opener(data, item));
                unreadable(next.opener)
            }
            ItemKind::Malformed(_) => true,
            ItemKind::Frame | ItemKind::Offsets | ItemKind::Locator => {
                if let Some(head) = segment.head {
                    if item.kind == ItemKind::Frame {
                        let (frame, failed) =
                            read_frame(data, item, segment, &head, walk, budget)?;
                        classified.observed = Some(frame);
                        failed
                    } else {
                        classified.closing_error = check_closing_grammar(data, item).err();
                        classified.closing_error.is_some()
                    }
                } else {
                    false
                }
            }
            _ => false,
        };
    }
    Ok((item, classified))
}

/// Recover from a lost item by scanning for the next resync opener.
///
/// A refused candidate is recorded on `segment` and yields `None`. An accepted
/// candidate ends `segment` at the lost boundary and is returned so that the
/// caller starts the next segment from it.
///
/// # Errors
///
/// Returns the [`FramingReason`] when the accepted opener cannot end `segment`.
fn recover_lost_item<'a>(
    segment: &mut WireSegment<'a>,
    walk: &mut ItemWalk<'a>,
    scanner: &mut ResyncScanner<'a>,
    budget: &mut PayloadBudget,
    boundary: Boundary,
    data: &'a [u8],
) -> Result<Option<Candidate<'a>>, FramingReason> {
    let previous_healed = segment.frames.last().is_some_and(|frame| {
        frame.position + 1 == segment.items.len() && frame.link.state == LinkState::Healed
    });
    let start = if previous_healed {
        boundary.offset
    } else {
        segment
            .items
            .last()
            .map_or(boundary.offset, |item| item.offset + 1)
    };
    match walk.recover(scanner, start, boundary, budget) {
        Some(candidate @ Candidate::Accepted { offset, .. }) => {
            end_at_resync(segment, data, boundary, offset)?;
            Ok(Some(candidate))
        }
        Some(Candidate::Refused { offset, count }) => {
            let position = segment.items.len() - usize::from(offset < boundary.offset);
            segment.resync_refusal = Some(ResyncRefusal {
                position,
                offset,
                length: RESYNC_MAX,
                count,
            });
            Ok(None)
        }
        None => Ok(None),
    }
}


/// Read whole-input wire observations, including resync at lost framing.
///
/// A scanned opener is admitted by the same decoded byte budget as frames.
/// The original item positions are held even when a candidate starts inside
/// the complete item before the failure.
///
/// Framing damage before an accepted resync opener is not in `stop`. It is
/// reported on the segment (`framing_failure`, with a `DamagedRegion` item) or,
/// before the first segment, in `prefix_failure`. Refused candidates are in
/// `resync_refusal`. `stop` is set only when no candidate resumes the read.
#[must_use]
pub fn read_wire(data: &[u8], capacity: ReadCapacity) -> WireRead<'_> {
    let mut result = WireRead::default();
    let mut walk = ItemWalk::new(data, capacity.items_per_read);
    let mut scanner = ResyncScanner::new(data);
    let mut budget = PayloadBudget::new(capacity);
    let Some(mut segment) = first_segment(data, &mut walk, &mut scanner, &mut budget, &mut result)
    else {
        return result;
    };
    while let Some(boundary) = walk.take() {
        let (mut item, classified) =
            match classify_boundary(data, &segment, &mut walk, &mut budget, boundary) {
                Ok(classified) => classified,
                Err(reason) => {
                    result.stop = Some(framing_stop(
                        &mut segment,
                        reason,
                        boundary.offset,
                        data.len(),
                    ));
                    break;
                }
            };
        let Classified {
            lost,
            begins,
            observed,
            closing_error,
        } = classified;
        if lost {
            match recover_lost_item(
                &mut segment,
                &mut walk,
                &mut scanner,
                &mut budget,
                boundary,
                data,
            ) {
                Ok(Some(Candidate::Accepted { offset, value })) => {
                    result
                        .segments
                        .push(segment.with_claims(data, capacity.stored_bytes_per_frame));
                    segment = recovered_segment(offset, value);
                    continue;
                }
                Ok(_) => {}
                Err(reason) => {
                    result.stop = Some(framing_stop(
                        &mut segment,
                        reason,
                        boundary.offset,
                        data.len(),
                    ));
                    break;
                }
            }
        }
        if let Some(reason) = boundary.failure {
            result.stop = Some(framing_stop(
                &mut segment,
                reason,
                boundary.offset,
                boundary.end,
            ));
            break;
        }
        if let Some(next) = begins {
            segment.end_offset = item.offset;
            result
                .segments
                .push(segment.with_claims(data, capacity.stored_bytes_per_frame));
            segment = next;
            continue;
        }
        if let Some(frame) = observed {
            if let Some(identifier) = frame.link.effective_identifier {
                segment.head = Some(identifier);
            }
            segment.frames.push(frame);
        }
        if let Some(error) = closing_error {
            segment.closing_grammar.push((segment.items.len(), error));
        }
        if segment.opener.state().1.is_none() {
            item.kind = ItemKind::Unchecked;
        }
        segment.end_offset = item.end;
        segment.items.push(item);
    }
    result.stop = result.stop.or_else(|| walk.item_limit());
    result
        .segments
        .push(segment.with_claims(data, capacity.stored_bytes_per_frame));
    result
}
