// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use std::{collections::HashMap, str, sync::Arc};

use super::{values::append_hex, ValueIssue};
use crate::v2::{
    attachments::{
        decode_attachment, decode_pack, unit_digest, Attachment, Reference, BLOB, PACK,
        UNIT_CONTEXT, VALUE,
    },
    HeaderError,
};

const REBUILT_BYTES_PER_READ: u64 = 1 << 37;

pub(super) struct Rebuild<'a> {
    units: HashMap<[u8; 32], &'a [u8]>,
    values: HashMap<Reference, Candidate<'a>>,
    charged: u64,
    refused: bool,
    materialized_remaining: u64,
}

struct Candidate<'a> {
    id: usize,
    value: Result<RebuiltValue<'a>, ValueIssue>,
}

struct RebuiltValue<'a> {
    parts: Vec<&'a [u8]>,
    text: Option<Arc<str>>,
    hex: Option<Arc<str>>,
}

impl<'a> Rebuild<'a> {
    pub(super) fn new(limit: u64) -> Self {
        Self {
            units: HashMap::new(),
            values: HashMap::new(),
            charged: 0,
            refused: false,
            materialized_remaining: limit,
        }
    }

    pub(super) fn start_segment(&mut self) {
        self.units.clear();
    }

    pub(super) fn frame(
        &mut self,
        kind: u64,
        content: &[u8],
        payload: &'a [u8],
    ) -> Result<(), HeaderError> {
        if kind == PACK {
            let members = decode_pack(content, payload)?;
            // The entire pack occupies one frame position.
            for member in members.iter().filter(|member| member.kind <= BLOB) {
                self.unit(member);
            }
            for member in members.into_iter().filter(|member| member.kind > BLOB) {
                self.declare(member);
            }
        } else {
            let record = decode_attachment(kind, content, payload)?;
            if record.kind <= BLOB {
                self.unit(&record);
            } else {
                self.declare(record);
            }
        }
        Ok(())
    }

    fn unit(&mut self, record: &Attachment<'a>) {
        let digest = record.reference.digest;
        if unit_digest(record.bytes) == digest {
            self.units.entry(digest).or_insert(record.bytes);
        }
    }

    fn admit(&mut self, length: u64) -> bool {
        if self.refused || length > REBUILT_BYTES_PER_READ - self.charged {
            self.refused = true;
            false
        } else {
            self.charged += length;
            true
        }
    }

    fn declare(&mut self, declaration: Attachment<'a>) {
        let admitted = self.admit(declaration.reference.length);
        // Recipes participate in the budget. RDF projection exposes no blobs.
        if declaration.kind != VALUE {
            return;
        }
        let result = if admitted {
            self.verify_value(&declaration)
        } else {
            Err(ValueIssue::RebuiltCapacity)
        };
        let id = self.values.len();
        let candidate = self
            .values
            .entry(declaration.reference)
            .or_insert_with(|| Candidate {
                id,
                value: Err(ValueIssue::MissingValue),
            });
        if matches!(candidate.value, Err(ValueIssue::MissingValue))
            || candidate.value.is_err() && result.is_ok()
        {
            candidate.value = result;
        }
    }

    fn verify_value(&self, declaration: &Attachment<'_>) -> Result<RebuiltValue<'a>, ValueIssue> {
        let mut hash = blake3::Hasher::new_derive_key(UNIT_CONTEXT);
        let mut parts = Vec::new();
        for entry in &declaration.entries {
            let &bytes = self
                .units
                .get(&entry.digest)
                .ok_or(ValueIssue::MissingUnit)?;
            if bytes.len() as u64 != entry.length {
                return Err(ValueIssue::UnitLength);
            }
            hash.update(bytes);
            parts.push(bytes);
        }
        if hash.finalize().as_bytes() != &declaration.reference.digest {
            return Err(ValueIssue::ValueDigest);
        }
        Ok(RebuiltValue {
            parts,
            text: None,
            hex: None,
        })
    }

    pub(super) fn check_text(&self, reference: Reference) -> Result<(), ValueIssue> {
        let value = self
            .values
            .get(&reference)
            .ok_or(ValueIssue::MissingValue)?
            .value
            .as_ref()
            .map_err(|&issue| issue)?;
        if value.text.is_none() && !valid_utf8(&value.parts) {
            return Err(ValueIssue::InvalidUtf8);
        }
        Ok(())
    }

    pub(super) fn lexical(
        &mut self,
        reference: Reference,
        octets: bool,
    ) -> Result<(usize, Arc<str>), ValueIssue> {
        let candidate = self
            .values
            .get_mut(&reference)
            .ok_or(ValueIssue::MissingValue)?;
        let value = candidate.value.as_mut().map_err(|&mut issue| issue)?;
        let cache = if octets {
            &mut value.hex
        } else {
            &mut value.text
        };
        if let Some(text) = cache {
            return Ok((candidate.id, Arc::clone(text)));
        }
        let size = reference
            .length
            .checked_mul(if octets { 2 } else { 1 })
            .filter(|size| *size <= self.materialized_remaining)
            .and_then(|size| isize::try_from(size).ok())
            .ok_or(ValueIssue::MaterializedCapacity)? as usize;
        let lexical: Arc<str> = if octets {
            let mut text = String::with_capacity(size);
            for part in &value.parts {
                append_hex(&mut text, part);
            }
            Arc::from(text)
        } else {
            let mut bytes = Vec::with_capacity(size);
            for part in &value.parts {
                bytes.extend_from_slice(part);
            }
            Arc::from(String::from_utf8(bytes).map_err(|_| ValueIssue::InvalidUtf8)?)
        };
        self.materialized_remaining -= size as u64;
        *cache = Some(Arc::clone(&lexical));
        Ok((candidate.id, lexical))
    }
}

fn valid_utf8(parts: &[&[u8]]) -> bool {
    let mut tail = [0_u8; 4];
    let mut held = 0;
    for &part in parts {
        let mut rest = part;
        while held != 0 {
            let Some((&byte, remaining)) = rest.split_first() else {
                break;
            };
            tail[held] = byte;
            held += 1;
            rest = remaining;
            match str::from_utf8(&tail[..held]) {
                Ok(_) => held = 0,
                Err(error) if error.error_len().is_some() => return false,
                Err(_) => {}
            }
        }
        if held != 0 {
            continue;
        }
        if let Err(error) = str::from_utf8(rest) {
            if error.error_len().is_some() {
                return false;
            }
            let incomplete = &rest[error.valid_up_to()..];
            held = incomplete.len();
            tail[..held].copy_from_slice(incomplete);
        }
    }
    held == 0
}
