// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Whole-input framing and frame observations for the GTS v2 reader.

use std::collections::VecDeque;

use super::{
    decode_header,
    durability::{check_closing, check_closing_grammar, check_commits},
    frame::{check_type_length, decode_frame_envelope, frame_identifier, Frame, FrameError},
    framing::{byte_chunks, generation, item_end, read_head, Generation},
    payload::{malformed_payload, PayloadBudget},
    resync::{decode_resync, Candidate, ResyncScanner, RESYNC_MAX},
    DecodedHeader, Decoder, HeaderError, MalformedReason, REGISTRY_VERSION,
};

pub use super::{
    durability::{ClosingFinding, CommitCheck, CommitReadError, CommitReport, UncommittedTail},
    framing::{FramingFailure, FramingReason},
    payload::{PayloadCheck, PayloadDamage, PayloadRefusal, PayloadState, ReadCapacity},
};

pub use super::resync::DecodedResync;

/// The first item determines which generation reads the file.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Dispatch {
    Empty,
    FirstGeneration,
    SecondGeneration,
    UnsupportedGeneration(u64),
    NotThisFormat,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReadStop {
    Framing(FramingFailure),
    ItemLimit { offset: usize },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ItemKind {
    Header,
    Resync,
    Frame,
    Locator,
    Offsets,
    Malformed(MalformedReason),
    Unknown(u64),
    Unchecked,
    DamagedRegion,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WireItem {
    pub kind: ItemKind,
    pub offset: usize,
    pub end: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Opener<'a> {
    Header(Result<DecodedHeader<'a>, HeaderError>),
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LinkState {
    Linked,
    Healed,
    Broken,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FrameLink {
    pub state: LinkState,
    pub computed_identifier: Option<[u8; 32]>,
    pub effective_identifier: Option<[u8; 32]>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WireFrame<'a> {
    pub position: usize,
    pub frame_type: Option<u64>,
    pub transparent: bool,
    pub envelope: Result<Frame<'a>, FrameError>,
    pub link: FrameLink,
    pub payload: PayloadCheck<'a>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WireSegment<'a> {
    pub opener: Opener<'a>,
    pub items: Vec<WireItem>,
    pub frames: Vec<WireFrame<'a>>,
    pub head: Option<[u8; 32]>,
    /// The declared segment boundary can fall inside its last item after recovery.
    pub end_offset: usize,
    pub scanned: bool,
    pub framing_failure: Option<FramingFailure>,
    pub resync_refusal: Option<ResyncRefusal>,
    /// Grammar errors observed during the framing test.
    pub closing_grammar: Vec<(usize, HeaderError)>,
    /// Closing findings after placement and claims are checked over retained items.
    pub closing_findings: Vec<ClosingFinding>,
    pub commits: Result<CommitReport, CommitReadError>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResyncRefusal {
    pub position: usize,
    pub offset: usize,
    pub length: usize,
    pub count: u64,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct WireRead<'a> {
    /// Generation dispatch after checking for a resync opener in the first item.
    pub dispatch: Option<Dispatch>,
    pub segments: Vec<WireSegment<'a>>,
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

impl<'a> WireSegment<'a> {
    fn with_claims(mut self, data: &[u8], stored_limit: u64) -> Self {
        if let Some(identifier) = self.opener.state().1 {
            self.closing_findings = check_closing(data, &self.items, &self.frames);
            self.commits = check_commits(&identifier, &self.frames, stored_limit);
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
        let scanned = scanner.scan(start, lost.end, budget);
        if let Some(Candidate::Accepted { value, .. }) = scanned.as_ref() {
            self.count += 1;
            self.held.clear();
            self.offset = value.next_offset;
        }
        scanned
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
    let payload = match &envelope {
        Ok((frame, _)) => budget.check(frame.into(), frame.check_storage()),
        Err(_) => {
            let offsets = offsets.ok_or(FramingReason::UndelimitableInput)?;
            let (stored_matches, input) =
                malformed_payload(data, &offsets, registry > REGISTRY_VERSION);
            match input {
                Ok(input) => budget.check(input, stored_matches),
                Err(_) => PayloadCheck {
                    stored_matches,
                    state: PayloadState::Damaged(PayloadDamage::MalformedFields),
                },
            }
        }
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
        let last = segment.items[frame.position];
        let stored = stored_identifier(data, field_offsets(data, last.offset)?[8]);
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
        let mut begins = None;
        let mut observed = None;
        let mut closing_error = None;
        let mut item = WireItem {
            kind: ItemKind::Unchecked,
            offset: boundary.offset,
            end: boundary.end,
        };
        let mut lost = boundary.failure.is_some();
        if !lost {
            let newer = segment.head.is_none() || segment.opener.state().0 > REGISTRY_VERSION;
            item.kind = classify(&data[..item.end], item.offset, newer);
            lost = match item.kind {
                ItemKind::Header | ItemKind::Resync => {
                    let next = begins.insert(opener(data, item));
                    unreadable(next.opener)
                }
                ItemKind::Malformed(_) => true,
                ItemKind::Frame | ItemKind::Offsets | ItemKind::Locator => {
                    if let Some(head) = segment.head {
                        if item.kind == ItemKind::Frame {
                            let (frame, failed) = match read_frame(
                                data,
                                item,
                                &segment,
                                &head,
                                &mut walk,
                                &mut budget,
                            ) {
                                Ok(observed) => observed,
                                Err(reason) => {
                                    result.stop = Some(framing_stop(
                                        &mut segment,
                                        reason,
                                        item.offset,
                                        data.len(),
                                    ));
                                    break;
                                }
                            };
                            observed = Some(frame);
                            failed
                        } else {
                            closing_error = check_closing_grammar(data, item).err();
                            closing_error.is_some()
                        }
                    } else {
                        false
                    }
                }
                _ => false,
            };
        }
        if lost {
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
            match walk.recover(&mut scanner, start, boundary, &mut budget) {
                Some(Candidate::Accepted { offset, value }) => {
                    if let Err(reason) = end_at_resync(&mut segment, data, boundary, offset) {
                        result.stop = Some(framing_stop(
                            &mut segment,
                            reason,
                            boundary.offset,
                            data.len(),
                        ));
                        break;
                    }
                    result
                        .segments
                        .push(segment.with_claims(data, capacity.stored_bytes_per_frame));
                    segment = recovered_segment(offset, value);
                    continue;
                }
                Some(Candidate::Refused { offset, count }) => {
                    let position = segment.items.len() - usize::from(offset < boundary.offset);
                    segment.resync_refusal = Some(ResyncRefusal {
                        position,
                        offset,
                        length: RESYNC_MAX,
                        count,
                    });
                }
                None => {}
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
