// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Whole-input framing and frame observations for the GTS v2 reader.

use super::{
    decode_header,
    frame::{decode_frame, frame_identifier, Frame, FrameError},
    framing::{byte_chunks, generation, item_end, read_head, Generation},
    payload::{malformed_payload, PayloadBudget},
    DecodedHeader, Decoder, Header, HeaderError, MalformedReason, HEADER_CONTEXT, REGISTRY_VERSION,
};

pub use super::{
    framing::{FramingFailure, FramingReason},
    payload::{PayloadCheck, PayloadDamage, PayloadRefusal, PayloadState, ReadCapacity},
};

const RESYNC_MARKER: [u8; 16] = [
    0xfc, 0xf3, 0xeb, 0x32, 0xa2, 0xf0, 0x26, 0x7d, 0xd1, 0x6a, 0x53, 0xf0, 0x9b, 0xff, 0x87, 0x92,
];

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
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WireItem {
    pub kind: ItemKind,
    pub offset: usize,
    pub end: usize,
}

/// A resync opener reached by walking from an established item boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DecodedResync<'a> {
    pub header: Header<'a>,
    pub claimed_offset: u64,
    pub past: Option<&'a [u8]>,
    pub marks: &'a [u8],
    pub stored_identifier: &'a [u8],
    pub identifier: Option<[u8; 32]>,
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
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WireRead<'a> {
    /// No generation is dispatched when the first item cannot be read.
    pub dispatch: Option<Dispatch>,
    pub segments: Vec<WireSegment<'a>>,
    pub stop: Option<ReadStop>,
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

fn decode_resync<'a>(
    data: &'a [u8],
    offset: usize,
    end: usize,
) -> Result<DecodedResync<'a>, HeaderError> {
    use MalformedReason::{OutOfRange, UnorderedOrRepeated, WrongLength, WrongType};
    let limit = offset.saturating_add(4096);
    let mut cursor = Decoder::new(&data[..end.min(limit)], offset);
    let decode = |cursor: &mut Decoder<'a>| {
        cursor.record(11)?;
        cursor.code(5)?;
        let error = cursor.error(OutOfRange);
        if cursor.uint()? != 2 {
            return Err(error);
        }
        let error = cursor.error(OutOfRange);
        if cursor.byte_string(None)? != RESYNC_MARKER {
            return Err(error);
        }
        let (registry_version, hash_algorithm, profile) = cursor.opener_fields()?;
        let known = hash_algorithm == 1;
        let lineage = cursor.option(|cursor| cursor.lineage(known))?;
        let claimed_offset = cursor.uint()?;
        let digest_size = known.then_some(32);
        let past = cursor.option(|cursor| cursor.byte_string(digest_size))?;
        let marks_at = cursor.offset;
        let count = cursor.head(4)?;
        let mut last: &[u8] = &[];
        for index in 0..count {
            let mark_at = cursor.offset;
            let elements = cursor.head(4)?;
            if elements == 0 {
                return Err(HeaderError {
                    reason: WrongType,
                    offset: mark_at,
                });
            }
            // The first element must be a uint by shape before count and order.
            let Some(tag) = cursor.bytes.get(cursor.offset) else {
                cursor.exhausted = true;
                return Err(cursor.error(WrongLength));
            };
            if tag >> 5 != 0 {
                return Err(HeaderError {
                    reason: WrongType,
                    offset: mark_at,
                });
            }
            if index >= 64 {
                return Err(HeaderError {
                    reason: OutOfRange,
                    offset: mark_at,
                });
            }
            // A complete item cannot be a proper prefix of another complete item.
            let available = &cursor.bytes[mark_at..];
            let prefix = &available[..available.len().min(last.len())];
            if !last.is_empty()
                && prefix <= last
                && (prefix.len() == last.len() || !last.starts_with(prefix))
            {
                return Err(HeaderError {
                    reason: UnorderedOrRepeated,
                    offset: mark_at,
                });
            }
            for _ in 0..elements {
                cursor.closed_item()?;
            }
            last = &cursor.bytes[mark_at..cursor.offset];
        }
        let marks = &cursor.bytes[marks_at..cursor.offset];
        let body = &cursor.bytes[offset + 1..cursor.offset];
        let stored_identifier = cursor.byte_string(digest_size)?;
        let identifier = known.then(|| {
            let mut hash = blake3::Hasher::new_derive_key(HEADER_CONTEXT);
            hash.update(&[0x8a]);
            hash.update(body);
            *hash.finalize().as_bytes()
        });
        Ok(DecodedResync {
            header: Header {
                registry_version,
                hash_algorithm,
                profile,
                canonical: None,
                lineage,
            },
            claimed_offset,
            past,
            marks,
            stored_identifier,
            identifier,
        })
    };
    decode(&mut cursor).map_err(|error| {
        if end > limit && cursor.exhausted {
            HeaderError {
                reason: OutOfRange,
                offset: limit,
            }
        } else {
            error
        }
    })
}

fn opener(data: &[u8], item: WireItem) -> WireSegment<'_> {
    let opener = if item.kind == ItemKind::Header {
        Opener::Header(decode_header(&data[..item.end], item.offset))
    } else {
        Opener::Resync(decode_resync(data, item.offset, item.end))
    };
    let (_, head) = opener.state();
    WireSegment {
        opener,
        items: vec![item],
        frames: Vec::new(),
        head,
    }
}

fn field_offsets(data: &[u8], offset: usize) -> [usize; 11] {
    let mut cursor = offset + 2;
    std::array::from_fn(|index| {
        let start = cursor;
        // The eleven-element envelope was delimited before field inspection.
        if index < 10 {
            cursor = item_end(data, start).expect("delimited frame field");
        }
        start
    })
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
    can_peek: bool,
    budget: &mut PayloadBudget,
) -> WireFrame<'a> {
    let (registry, _) = segment.opener.state();
    let envelope = decode_frame(data, item.offset, registry);
    let mut offsets = None;
    let (frame_type, fields) = if let Ok(frame) = &envelope {
        (
            Some(frame.frame_type),
            LinkFields {
                transparent: frame.transparent,
                identity: Some(frame.identity_fields),
                stored: Some(*frame.identifier),
            },
        )
    } else {
        let offsets = offsets.insert(field_offsets(data, item.offset));
        let frame_type = Decoder::new(data, offsets[0]).uint().ok();
        (frame_type, LinkFields::inspect(data, offsets))
    };
    let computed = fields.computed(predecessor);
    let state = if computed.is_some() && computed == fields.stored {
        LinkState::Linked
    } else if let Some(identifier) =
        computed.filter(|_| can_peek && !fields.transparent && fields.stored.is_some())
    {
        let next = &data[item.end..];
        if next.starts_with(&[0x8b, 0x02])
            && item_end(data, item.end).is_ok()
            && LinkFields::inspect(data, &field_offsets(data, item.end)).matches(&identifier)
        {
            LinkState::Healed
        } else {
            LinkState::Broken
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
        Ok(frame) => budget.check(frame.into(), frame.check_storage()),
        Err(_) => {
            let offsets = offsets.expect("malformed frame fields");
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
    }
}

/// Read the item boundaries, openers and frame observations of a byte input.
///
/// Framing stops at the first incomplete or undelimitable item. This layer
/// supplies the inputs for resumption, content folding, and typed results.
#[must_use]
pub fn read_wire(data: &[u8], capacity: ReadCapacity) -> WireRead<'_> {
    let mut result = WireRead {
        dispatch: None,
        segments: Vec::new(),
        stop: None,
    };
    if data.is_empty() {
        result.dispatch = Some(Dispatch::Empty);
        return result;
    }
    let mut budget = PayloadBudget::new(capacity);
    let mut offset = 0;
    let mut count = 0;
    while offset < data.len() {
        if count == capacity.items_per_read {
            result.stop = Some(ReadStop::ItemLimit { offset });
            break;
        }
        let end = match item_end(data, offset) {
            Ok(end) => end,
            Err(reason) => {
                result.stop = Some(ReadStop::Framing(FramingFailure {
                    reason,
                    offset,
                    length: data.len() - offset,
                }));
                break;
            }
        };
        count += 1;
        if offset == 0 {
            let dispatch = match generation(&data[..end]) {
                Some(Generation::Record(2)) => Dispatch::SecondGeneration,
                Some(Generation::Legacy(1)) => Dispatch::FirstGeneration,
                Some(Generation::Legacy(version) | Generation::Record(version @ 3..)) => {
                    Dispatch::UnsupportedGeneration(version)
                }
                _ => Dispatch::NotThisFormat,
            };
            result.dispatch = Some(dispatch);
            if dispatch != Dispatch::SecondGeneration {
                break;
            }
        }
        let newer = result
            .segments
            .last()
            .is_some_and(|segment| segment.opener.state().0 > REGISTRY_VERSION);
        let kind = classify(&data[..end], offset, newer);
        let mut item = WireItem { kind, offset, end };
        if matches!(kind, ItemKind::Header | ItemKind::Resync) {
            result.segments.push(opener(data, item));
        } else if let Some(segment) = result.segments.last_mut() {
            if let Some(predecessor) = segment.head {
                if kind == ItemKind::Frame {
                    let frame = read_frame(
                        data,
                        item,
                        segment,
                        &predecessor,
                        count < capacity.items_per_read,
                        &mut budget,
                    );
                    if let Some(identifier) = frame.link.effective_identifier {
                        segment.head = Some(identifier);
                    }
                    segment.frames.push(frame);
                }
            } else {
                item.kind = ItemKind::Unchecked;
            }
            segment.items.push(item);
        }
        offset = end;
    }
    result
}
