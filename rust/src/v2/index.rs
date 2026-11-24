// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Index grammar and the claims the wire reader can check (P5 §§4.1, 4.3).

use super::{
    frame::{INDEX, PARITY},
    payload::PayloadState,
    range::{decode_claim, Digest, Range},
    reader::{LinkState, WireFrame},
    Decoder, HeaderError, MalformedReason, REGISTRY_VERSION,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IndexCheck {
    pub position: usize,
    pub outcome: IndexOutcome,
}

/// Wire-level index results. Fold-dependent claims require a content reader.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IndexOutcome {
    Checked(IndexWireClaims),
    /// The error offset is relative to the decoded payload.
    MalformedPayload(HeaderError),
    /// A later registry value in the payload at the given decoded offset.
    UnknownEnumeration {
        offset: usize,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IndexWireClaims {
    /// Both the frame count and the claimed head match the prefix.
    pub head_matches: bool,
    pub root_matches: bool,
    pub type_runs_match: bool,
}

enum PayloadError {
    Malformed(HeaderError),
    Unknown { offset: usize },
}

impl From<HeaderError> for PayloadError {
    fn from(error: HeaderError) -> Self {
        Self::Malformed(error)
    }
}

struct Registry {
    newer: bool,
    stopped: bool,
}

impl Registry {
    fn code(&self, cursor: &mut Decoder<'_>, last: u64) -> Result<u64, PayloadError> {
        let error = cursor.error(MalformedReason::UnregisteredValue);
        let code = cursor.uint()?;
        if code == 0 || (code > last && !self.newer) {
            return Err(error.into());
        }
        if code > last && !self.stopped {
            return Err(PayloadError::Unknown {
                offset: error.offset,
            });
        }
        Ok(code)
    }

    fn graph_name(&self, cursor: &mut Decoder<'_>) -> Result<(), PayloadError> {
        let error = cursor.error(MalformedReason::WrongLength);
        let fields = cursor.head(4)?;
        if fields == 0 {
            return Err(error.into());
        }
        match self.code(cursor, 2)? {
            tag @ 1..=2 if tag != fields => return Err(error.into()),
            1 => {}
            2 => {
                cursor.uint()?;
            }
            _ => {
                for _ in 1..fields {
                    cursor.closed_item()?;
                }
            }
        }
        Ok(())
    }
}

fn payload_claims<'a>(
    payload: &'a [u8],
    registry: &Registry,
    runs: &[(Option<u64>, u64)],
) -> Result<(bool, &'a Digest), PayloadError> {
    let mut cursor = Decoder::new(payload, 0);
    cursor.record(6)?;
    let count = cursor.head(4)?;
    let mut runs_match = count == runs.len() as u64;
    for index in 0..count {
        cursor.record(2)?;
        let frame_type = registry.code(&mut cursor, PARITY)?;
        let run = cursor.uint()?;
        runs_match &= usize::try_from(index)
            .ok()
            .and_then(|index| runs.get(index))
            == Some(&(Some(frame_type), run));
    }
    for _ in 0..cursor.head(4)? {
        cursor.record(2)?;
        registry.graph_name(&mut cursor)?;
        for _ in 0..cursor.head(4)? {
            cursor.record(4)?;
            registry.code(&mut cursor, PARITY)?;
            for _ in 0..3 {
                cursor.uint()?;
            }
        }
    }
    cursor.record(4)?;
    for _ in 0..3 {
        cursor.uint()?;
    }
    for _ in 0..cursor.head(4)? {
        cursor.record(2)?;
        cursor.uint()?;
        cursor.uint()?;
    }
    let root = cursor.fixed::<32>()?;
    cursor.option(Decoder::fixed::<32>)?;
    for _ in 0..cursor.head(4)? {
        cursor.record(3)?;
        cursor.fixed::<32>()?;
        cursor.uint()?;
        cursor.uint()?;
    }
    if cursor.offset != payload.len() {
        return Err(cursor.error(MalformedReason::WrongLength).into());
    }
    Ok((runs_match, root))
}

pub(super) fn index_bytes<'a, 'b>(frame: &'b WireFrame<'a>) -> Option<(&'a [u8], &'b [u8])> {
    let envelope = frame.envelope.as_ref().ok()?;
    if envelope.frame_type != INDEX || frame.link.state == LinkState::Broken {
        return None;
    }
    let PayloadState::Verified(payload) = &frame.payload.state else {
        return None;
    };
    Some((envelope.content, payload.as_ref()))
}

pub(super) struct IndexPrefix {
    registry: Registry,
    head: Digest,
    runs: Vec<(Option<u64>, u64)>,
}

impl IndexPrefix {
    pub(super) fn new(registry_version: u64, opener: Digest) -> Self {
        Self {
            registry: Registry {
                newer: registry_version > REGISTRY_VERSION,
                stopped: false,
            },
            head: opener,
            runs: Vec::new(),
        }
    }

    pub(super) fn check(
        &self,
        position: usize,
        content: &[u8],
        payload: &[u8],
        range: &Range,
    ) -> Result<IndexCheck, HeaderError> {
        let (count, head) = decode_claim(content)?;
        let outcome = match payload_claims(payload, &self.registry, &self.runs) {
            Ok((type_runs_match, root)) => IndexOutcome::Checked(IndexWireClaims {
                head_matches: count == range.ordinary_count() && *head == self.head,
                root_matches: *root == range.root(),
                type_runs_match,
            }),
            Err(PayloadError::Malformed(error)) => IndexOutcome::MalformedPayload(error),
            Err(PayloadError::Unknown { offset }) => IndexOutcome::UnknownEnumeration { offset },
        };
        Ok(IndexCheck { position, outcome })
    }

    pub(super) fn observe(&mut self, frame: &WireFrame<'_>) {
        self.registry.stopped |= frame.unknown_type_stops_fold;
        if let Some(identifier) = frame.link.effective_identifier {
            self.head = identifier;
            match self.runs.last_mut() {
                Some((kind, length)) if *kind == frame.frame_type => *length += 1,
                _ => self.runs.push((frame.frame_type, 1)),
            }
        }
    }
}
