// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The shared P5 range over effective identifiers.

use ciborium::value::Value;

use crate::wire;

pub(super) type Digest = [u8; 32];

pub(super) struct RangeFull;

fn leaf(identifier: &Digest) -> Digest {
    let record = Value::Array(vec![Value::Bytes(identifier.to_vec())]);
    blake3::derive_key("gts2 range leaf", &wire::encode(&record))
}

fn parent(left: &Digest, right: &Digest) -> Digest {
    let record = Value::Array(vec![
        Value::Bytes(left.to_vec()),
        Value::Bytes(right.to_vec()),
    ]);
    blake3::derive_key("gts2 range node", &wire::encode(&record))
}

pub(super) struct Range {
    count: u64,
    peaks: Vec<(u8, Digest)>,
}

impl Range {
    pub(super) fn new(opener: &Digest) -> Self {
        Self {
            count: 1,
            peaks: vec![(0, leaf(opener))],
        }
    }

    pub(super) fn append(&mut self, identifier: &Digest) -> Result<(), RangeFull> {
        let next = self.count.checked_add(1).ok_or(RangeFull)?;
        let mut height = 0;
        let mut digest = leaf(identifier);
        while let Some((left_height, left)) = self.peaks.last().copied() {
            if left_height != height {
                break;
            }
            self.peaks.pop();
            digest = parent(&left, &digest);
            height += 1;
        }
        self.peaks.push((height, digest));
        self.count = next;
        Ok(())
    }

    pub(super) fn ordinary_count(&self) -> u64 {
        self.count - 1
    }

    pub(super) fn root(&self) -> Digest {
        let peaks = self
            .peaks
            .iter()
            .map(|(_, digest)| Value::Bytes(digest.to_vec()))
            .collect();
        let record = Value::Array(vec![Value::from(self.count), Value::Array(peaks)]);
        blake3::derive_key("gts2 range root", &wire::encode(&record))
    }
}
