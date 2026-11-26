// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Whole-input framing and frame observations for the GTS v2 reader.

use std::collections::VecDeque;

use super::{
    decode_header,
    durability::{check_closing, check_closing_grammar, check_prefixes},
    frame::{
        check_type_length, decode_frame_envelope, frame_identifier, Frame, FrameError, Signature,
        PARITY,
    },
    framing::{byte_chunks, generation, item_end, read_head, Generation},
    parity::{self, Verdict},
    parity_repair,
    payload::{malformed_payload, PayloadBudget},
    resync::{decode_resync, Candidate, ResyncScanner, RESYNC_MAX},
    signature::{verify_signature, HeldKeys, SignatureResult},
    DecodedHeader, Decoder, HeaderError, MalformedReason, REGISTRY_VERSION,
};

pub use super::{
    durability::{ClosingFinding, CommitCheck, CommitReadError, CommitReport, UncommittedTail},
    framing::{FramingFailure, FramingReason},
    index::{IndexCheck, IndexOutcome, IndexWireClaims},
    payload::{PayloadCheck, PayloadDamage, PayloadRefusal, PayloadState, ReadCapacity},
};

pub use super::parity::Judgement;
pub use super::parity_repair::{ParityCode, ParityFinding, ParityReason, Patch};
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
    pub(super) fn state(&self) -> (u64, Option<[u8; 32]>) {
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
    /// The stage 1 judgement of a frame of type `parity` (P5 6.6), or `None` for any other frame,
    /// for a frame in a segment without an identifier, and for a frame that the paragraph on a
    /// later registry version leaves to the unknown-type handling.
    pub judgement: Option<Judgement>,
    /// A later frame type with well-typed flags that stops content folding.
    pub unknown_type_stops_fold: bool,
    /// Whether the frame is detached (P1 3.4, step 4): it is ordinary and its predecessor is the
    /// `anchor` of the parity frame at which reading resumed, which is not the effective
    /// identifier of the nearest preceding ordinary frame that has one, or `h0` when none has.
    pub detached: bool,
    /// The outcome of each signature of the frame, in the order of its `signatures` list.
    ///
    /// The list is empty for a transparent frame, a frame without an effective identifier, a
    /// frame whose `signatures` field does not follow its grammar, a frame whose `signatures`
    /// element exceeds the stored-bytes capacity, and a frame in a segment whose opener has no
    /// identifier. A frame whose envelope is malformed in another field keeps its results.
    pub signatures: Vec<SignatureResult>,
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
    /// The regions of undelimitable input after which reading resumed at a parity frame (P5 7.2),
    /// in input order. Each is also a `DamagedRegion` item of the segment.
    pub resumptions: Vec<FramingFailure>,
    /// The parity candidate whose charge was the first that the total decoded length refused.
    /// Its `position` is that of the item the candidate begins in, or of the next item when it
    /// begins in bytes that cannot be delimited; its `length` is the length of the candidate
    /// item; its `count` is the total decoded length in bytes at the refusal.
    pub parity_refusal: Option<ResyncRefusal>,
    /// The `anchor` of the parity frame at which reading resumed, until the next ordinary frame
    /// that has an effective identifier: the predecessor of that frame.
    anchor: Option<[u8; 32]>,
    /// The parity frames read so far, as condition 6 of P5 6.6 compares them.
    earlier: parity::Earlier,
    /// Grammar errors observed during the framing test.
    pub closing_grammar: Vec<(usize, HeaderError)>,
    /// Closing findings after placement and claims are checked over retained items.
    pub closing_findings: Vec<ClosingFinding>,
    /// The commit report of the segment, or the error that prevented it.
    pub commits: Result<CommitReport, CommitReadError>,
    /// The index checks of the segment.
    pub indexes: Vec<IndexCheck>,
    /// The findings of the parity layer, in input order (P5 6.6, 6.7 and 7.2).
    pub parity_findings: Vec<ParityFinding>,
    /// The byte patches of the accepted repairs, which the repaired copy writes (P5 7.1).
    pub parity_patches: Vec<Patch>,
}

/// A resync item or a parity candidate that the total decoded length refused.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResyncRefusal {
    /// The position of the item the refused candidate begins in, or of the next item.
    pub position: usize,
    /// The offset of the refused candidate.
    pub offset: usize,
    /// The length of the refused span, in bytes.
    pub length: usize,
    /// The total decoded length in bytes that the refused charge would have made: the total
    /// kept so far plus the charge of the candidate. It is not a count of items.
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
    /// The repaired copy (P5 7.1): the input with the bytes of every accepted repair written in
    /// place, in segment order. It is `None` when no span is repaired.
    pub repaired_copy: Option<Vec<u8>>,
}

/// Write the byte patches of every segment into a copy of `data`, in segment order (P5 7.1).
fn repaired_copy(data: &[u8], segments: &[WireSegment<'_>]) -> Option<Vec<u8>> {
    let mut patches = segments
        .iter()
        .flat_map(|segment| segment.parity_patches.iter())
        .peekable();
    patches.peek()?;
    let mut output = data.to_vec();
    for patch in patches {
        output[patch.offset..patch.offset + patch.data.len()].copy_from_slice(&patch.data);
    }
    Some(output)
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
        resumptions: Vec::new(),
        parity_refusal: None,
        anchor: None,
        earlier: parity::Earlier::default(),
        closing_grammar: Vec::new(),
        closing_findings: Vec::new(),
        commits: Ok(CommitReport::default()),
        indexes: Vec::new(),
        parity_findings: Vec::new(),
        parity_patches: Vec::new(),
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
    /// Record `frame`, read at `item`, for condition 6 of P5 6.6 when it is a parity frame whose
    /// link state is not `broken`. A frame whose envelope is as the type states is the frame that
    /// a later registry version can set aside (see [`states_parity_envelope`]); any other parity
    /// frame is judged and counts, whether or not its envelope decodes as a whole.
    fn remember_parity(&mut self, data: &[u8], item: WireItem, frame: &WireFrame<'_>) {
        if frame.frame_type != Some(PARITY) || frame.link.state == LinkState::Broken {
            return;
        }
        let item_data = &data[..item.end];
        let Ok(offsets) = field_offsets(item_data, item.offset) else {
            return;
        };
        self.earlier.remember(
            &item_data[offsets[3]..offsets[4]],
            self.opener.state().0 > REGISTRY_VERSION,
            states_parity_envelope(item_data, &offsets),
        );
    }

    /// Check the claims of a finished segment and verify the signatures of its frames.
    fn finish(
        mut self,
        data: &'a [u8],
        capacity: ReadCapacity,
        keys: &HeldKeys,
        budget: &mut PayloadBudget,
    ) -> Self {
        let stored_limit = capacity.stored_bytes_per_frame;
        let (registry, identifier) = self.opener.state();
        if let Some(identifier) = identifier {
            self.verify_signatures(data, &identifier, stored_limit, keys);
            self.closing_findings = check_closing(data, &self.items, &self.frames);
            match check_prefixes(&identifier, &self.frames, stored_limit, registry) {
                Ok((commits, indexes)) => {
                    self.commits = Ok(commits);
                    self.indexes = indexes;
                }
                Err(error) => self.commits = Err(error),
            }
            let verification = parity_repair::verify(data, &self, budget);
            self.parity_findings = verification.findings;
            self.parity_patches = verification.patches;
        }
        self
    }

    /// Verify each signature of each eligible frame (P6 6.2) against `h0`.
    ///
    /// A frame is eligible when it is ordinary, has an effective identifier, has a `signatures`
    /// field in grammar and has that element within `stored_limit`. Its other fields do not
    /// matter: a frame whose envelope is malformed elsewhere has signature results too.
    fn verify_signatures(
        &mut self,
        data: &'a [u8],
        h0: &[u8; 32],
        stored_limit: u64,
        keys: &HeldKeys,
    ) {
        let newer = self.opener.state().0 > REGISTRY_VERSION;
        for frame in &mut self.frames {
            let Some(identifier) = frame
                .link
                .effective_identifier
                .filter(|_| !frame.transparent)
            else {
                continue;
            };
            let item = self.items[frame.position];
            let Some((records, size)) = signature_records(data, item, &frame.envelope, newer)
            else {
                continue;
            };
            if u64::try_from(size).is_ok_and(|size| size <= stored_limit) {
                frame.signatures = records
                    .iter()
                    .map(|signature| verify_signature(signature, h0, &identifier, keys))
                    .collect();
            }
        }
    }
}

/// The signature records of a frame item and the encoded size of its `signatures` element.
///
/// Returns `None` when the element is outside the grammar of P6 section 1.
fn signature_records<'a>(
    data: &'a [u8],
    item: WireItem,
    envelope: &Result<Frame<'a>, FrameError>,
    newer: bool,
) -> Option<(Vec<Signature<'a>>, usize)> {
    if let Ok(frame) = envelope {
        return Some((frame.signatures.clone(), frame.encoded_signatures_size));
    }
    let item_data = &data[..item.end];
    let offsets = field_offsets(item_data, item.offset).ok()?;
    let records = Decoder::new(item_data, offsets[9]).signatures(newer).ok()?;
    Some((records, offsets[10] - offsets[9]))
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
        parity: Option<&parity::Segment<'_>>,
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
        let outcome = match parity {
            Some(segment) => self.scan_both(scanner, start, lost, budget, segment),
            None => scanner.scan(start, lost.end, budget),
        };
        match outcome.as_ref() {
            Some(Candidate::Accepted { value, .. }) => {
                self.count += 1;
                self.held.clear();
                self.offset = value.next_offset;
            }
            Some(&Candidate::Parity { offset, .. }) => {
                // An incomplete item was not counted; the damaged region counts as one item.
                self.count += u64::from(lost.failure == Some(FramingReason::TornTail));
                self.held.clear();
                self.offset = offset;
            }
            _ => {}
        }
        outcome
    }

    /// Test the candidates of both kinds in one pass in ascending offset (P5 7.2, step 6).
    ///
    /// The resync candidates are tested over their own run, from `start`, and the parity
    /// candidates from the first byte that cannot be delimited. The first candidate that passes
    /// is returned, or the first refusal by the total decoded length.
    fn scan_both(
        &self,
        scanner: &mut ResyncScanner<'a>,
        start: usize,
        lost: Boundary,
        budget: &mut PayloadBudget,
        segment: &parity::Segment<'_>,
    ) -> Option<Candidate<'a>> {
        let mut resync_from = start;
        let mut parity_from = lost.offset;
        loop {
            let found = parity::next_prefix(self.data, parity_from);
            let bound = found.unwrap_or(lost.end);
            if let Some(candidate) = scanner.scan(resync_from, bound, budget) {
                return Some(candidate);
            }
            resync_from = resync_from.max(bound);
            let offset = found?;
            match parity::test(self.data, offset, segment, budget) {
                Verdict::Passed { anchor } => return Some(Candidate::Parity { offset, anchor }),
                Verdict::Refused { length, count } => {
                    return Some(Candidate::ParityRefused {
                        offset,
                        length,
                        count,
                    });
                }
                Verdict::Failed => parity_from = offset + 1,
            }
        }
    }

    fn item_limit(&self) -> Option<ReadStop> {
        (self.count == self.limit && self.offset < self.data.len()).then_some(ReadStop::ItemLimit {
            offset: self.offset,
        })
    }
}

pub(super) fn field_offsets(data: &[u8], offset: usize) -> Result<[usize; 11], FramingReason> {
    let mut offsets = [offset + 2; 11];
    for index in 1..offsets.len() {
        offsets[index] = item_end(data, offsets[index - 1])?;
    }
    Ok(offsets)
}

/// Whether the envelope of a parity frame is as the type states, read from the raw fields (P5
/// 6.6, the paragraph on a later version; P5 6.5).
///
/// The first of the conditions of the later-version case. It holds when `critical` is `false`
/// (element 2, byte `f4`), `transparent` is `true` (element 3, byte `f5`), `storage` is the
/// array of the one `uint` `1`, `stored_plain` (element 7), and `signatures` is an empty array
/// (element 10). It reads these four fields and no other, so a frame whose envelope fails
/// elsewhere (a field of another rule, such as `identifier`) is still judged by them.
///
/// A head is read by value. A second form of a head (`81 18 01` for `storage`, `98 00` for
/// `signatures`) has the value the type states and is a field-rule failure that condition 7
/// judges for a frame that 6.6 judges; the later-version case does not judge it (6.6: a frame
/// handled by that paragraph "is not judged by this section", and the case is "only" its five
/// conditions). A field of another major type, a `storage` of another tag and a non-empty
/// `signatures` list are not as the type states.
///
/// `offsets` are the field offsets of the frame item in `data` (see `field_offsets`).
fn states_parity_envelope(data: &[u8], offsets: &[usize; 11]) -> bool {
    data.get(offsets[1]) == Some(&0xf4)
        && data.get(offsets[2]) == Some(&0xf5)
        && is_plain_storage(data, offsets[6])
        && is_empty_array(data, offsets[9])
}

/// Whether the array at `offset` is `[1]` in any head form of its length and its element.
fn is_plain_storage(data: &[u8], offset: usize) -> bool {
    let Ok(array) = read_head(data, offset) else {
        return false;
    };
    let Ok(tag) = read_head(data, array.end) else {
        return false;
    };
    let closed = match array.argument {
        Some(count) => count == 1,
        None => data.get(tag.end) == Some(&0xff),
    };
    array.major == 4 && closed && tag.major == 0 && tag.argument == Some(1)
}

/// Whether the array at `offset` has no element in any head form of its length.
fn is_empty_array(data: &[u8], offset: usize) -> bool {
    let Ok(array) = read_head(data, offset) else {
        return false;
    };
    array.major == 4
        && match array.argument {
            Some(count) => count == 0,
            None => data.get(array.end) == Some(&0xff),
        }
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

/// The stage 1 judgement of an in-order frame of type `parity` (P5 6.6), or `None` for any other
/// frame and for a segment without an identifier.
fn judge_parity(
    frame_type: Option<u64>,
    segment: &WireSegment<'_>,
    envelope: Option<&Frame<'_>>,
    link: LinkState,
    predecessor: &[u8; 32],
    payload: &PayloadState<'_>,
) -> Option<Judgement> {
    let (version, identifier) = segment.opener.state();
    match (frame_type, identifier) {
        (Some(PARITY), Some(h0)) => parity::judge(
            envelope,
            link,
            predecessor,
            payload,
            &parity::Segment {
                h0: &h0,
                registry: version,
                earlier: &segment.earlier,
            },
            version > REGISTRY_VERSION,
        ),
        _ => None,
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
    let judgement = judge_parity(
        frame_type,
        segment,
        envelope.as_ref().ok().map(|(frame, _)| frame),
        state,
        predecessor,
        &payload.state,
    );
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
            judgement,
            unknown_type_stops_fold,
            detached: !fields.transparent
                && segment
                    .anchor
                    .is_some_and(|anchor| Some(anchor) != segment.head),
            signatures: Vec::new(),
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
    match walk.recover(scanner, 0, first, budget, None) {
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
        // Without a parity context the scan makes no parity candidate.
        Some(Candidate::Parity { .. } | Candidate::ParityRefused { .. }) | None => {}
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
                        let predecessor = segment.anchor.unwrap_or(head);
                        let (frame, failed) =
                            read_frame(data, item, segment, &predecessor, walk, budget)?;
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

/// Recover from a lost item by scanning for the next resync opener or parity frame.
///
/// A refused candidate is recorded on `segment` and yields `None`. An accepted
/// resync candidate ends `segment` at the lost boundary and is returned so that the
/// caller starts the next segment from it. A parity candidate that passes keeps the
/// segment: the bytes before it become one damaged region item, the segment records its
/// `anchor` as the predecessor of the next ordinary frame, and the candidate is returned
/// so that the caller reads on from it.
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
    let (registry, opener) = segment.opener.state();
    let scans_parity = boundary.failure.and(opener);
    let context = scans_parity.as_ref().map(|h0| parity::Segment {
        h0,
        registry,
        earlier: &segment.earlier,
    });
    match walk.recover(scanner, start, boundary, budget, context.as_ref()) {
        Some(candidate @ Candidate::Accepted { offset, .. }) => {
            end_at_resync(segment, data, boundary, offset)?;
            Ok(Some(candidate))
        }
        Some(candidate @ Candidate::Parity { offset, anchor }) => {
            segment.resumptions.push(FramingFailure {
                reason: FramingReason::UndelimitableInput,
                offset: boundary.offset,
                length: offset - boundary.offset,
            });
            segment.items.push(WireItem {
                kind: ItemKind::DamagedRegion,
                offset: boundary.offset,
                end: offset,
            });
            segment.end_offset = offset;
            segment.anchor = Some(anchor);
            Ok(Some(candidate))
        }
        Some(Candidate::ParityRefused {
            offset,
            length,
            count,
        }) => {
            segment.parity_refusal = Some(ResyncRefusal {
                position: segment.items.len(),
                offset,
                length,
                count,
            });
            Ok(None)
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
    read_wire_with_keys(data, capacity, &HeldKeys::default())
}

/// Read whole-input wire observations as [`read_wire`] does, verifying signatures.
///
/// Each signature of each eligible frame gets its outcome in [`WireFrame::signatures`],
/// judged against `keys` by the procedure of P6 section 6.2. A frame with no effective
/// identifier, a transparent frame and a frame of a segment whose opener has no identifier
/// have no results.
#[must_use]
pub fn read_wire_with_keys<'a>(
    data: &'a [u8],
    capacity: ReadCapacity,
    keys: &HeldKeys,
) -> WireRead<'a> {
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
                        .push(segment.finish(data, capacity, keys, &mut budget));
                    segment = recovered_segment(offset, value);
                    continue;
                }
                Ok(Some(Candidate::Parity { .. })) => continue,
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
                .push(segment.finish(data, capacity, keys, &mut budget));
            segment = next;
            continue;
        }
        if let Some(frame) = observed {
            if let Some(identifier) = frame.link.effective_identifier {
                segment.head = Some(identifier);
                segment.anchor = None;
            }
            segment.remember_parity(data, item, &frame);
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
        .push(segment.finish(data, capacity, keys, &mut budget));
    result.repaired_copy = repaired_copy(data, &result.segments);
    result
}
