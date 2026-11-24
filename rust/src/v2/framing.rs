// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! CBOR item boundaries and generation dispatch (P1 §§2.2–2.3, 6).

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FramingReason {
    TornTail,
    UndelimitableInput,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FramingFailure {
    pub reason: FramingReason,
    pub offset: usize,
    pub length: usize,
}

#[derive(Clone, Copy)]
pub(super) struct Head {
    pub major: u8,
    pub argument: Option<u64>,
    pub end: usize,
    pub shortest: bool,
}

pub(super) fn read_head(data: &[u8], offset: usize) -> Result<Head, FramingReason> {
    use FramingReason::{TornTail, UndelimitableInput};
    let first = *data.get(offset).ok_or(TornTail)?;
    let major = first >> 5;
    let mut end = offset + 1;
    let (argument, shortest) = match first & 31 {
        value @ 0..=23 => (Some(u64::from(value)), true),
        width @ 24..=27 => {
            let bytes = 1 << (width - 24);
            let body = data.get(end..end.saturating_add(bytes)).ok_or(TornTail)?;
            end += bytes;
            let value = body.iter().fold(0, |n, byte| (n << 8) | u64::from(*byte));
            if major == 7 && width == 24 && value < 32 {
                return Err(UndelimitableInput);
            }
            (
                Some(value),
                value >= [24, 256, 65_536, 1 << 32][usize::from(width - 24)],
            )
        }
        31 if matches!(major, 2..=5 | 7) => (None, false),
        _ => return Err(UndelimitableInput),
    };
    Ok(Head {
        major,
        argument,
        end,
        shortest,
    })
}

enum Open {
    Definite(u128),
    Array,
    Map { needs_value: bool },
    String(u8),
}

/// Establish the exclusive end of a well-formed CBOR item without decoding its values.
pub(super) fn item_end(data: &[u8], mut offset: usize) -> Result<usize, FramingReason> {
    use FramingReason::{TornTail, UndelimitableInput};
    let mut open = vec![Open::Definite(1)];
    while let Some(parent) = open.last_mut() {
        if let Open::String(major) = *parent {
            let first = *data.get(offset).ok_or(TornTail)?;
            if first != 0xff && (first >> 5 != major || first & 31 == 31) {
                return Err(UndelimitableInput);
            }
        }
        let head = read_head(data, offset)?;
        offset = head.end;
        if head.major == 7 && head.argument.is_none() {
            if matches!(parent, Open::Definite(_) | Open::Map { needs_value: true }) {
                return Err(UndelimitableInput);
            }
            open.pop();
        } else {
            match parent {
                Open::Definite(count) => *count -= 1,
                Open::Map { needs_value } => *needs_value = !*needs_value,
                _ => {}
            }
            match (head.major, head.argument) {
                (2 | 3, Some(length)) => {
                    let length = usize::try_from(length).map_err(|_| TornTail)?;
                    offset = offset.checked_add(length).ok_or(TornTail)?;
                    if offset > data.len() {
                        return Err(TornTail);
                    }
                }
                (4, Some(length)) if length != 0 => open.push(Open::Definite(u128::from(length))),
                (5, Some(length)) if length != 0 => {
                    open.push(Open::Definite(2 * u128::from(length)));
                }
                (6, _) => open.push(Open::Definite(1)),
                (2 | 3, None) => open.push(Open::String(head.major)),
                (4, None) => open.push(Open::Array),
                (5, None) => open.push(Open::Map { needs_value: false }),
                _ => {}
            }
        }
        while matches!(open.last(), Some(Open::Definite(0))) {
            open.pop();
        }
    }
    Ok(offset)
}

/// Borrow the octets of a delimited byte string regardless of its head encoding.
pub(super) fn byte_chunks(data: &[u8], offset: usize) -> Option<impl Iterator<Item = &[u8]>> {
    let head = read_head(data, offset).ok()?;
    if head.major != 2 {
        return None;
    }
    let mut next = Some(head.end);
    let mut first_length = head.argument;
    let chunked = first_length.is_none();
    Some(std::iter::from_fn(move || {
        let mut start = next.take()?;
        let length = if let Some(length) = first_length.take() {
            length
        } else {
            if data.get(start) == Some(&0xff) {
                return None;
            }
            let chunk = read_head(data, start).ok()?;
            start = chunk.end;
            chunk.argument?
        };
        let end = start.checked_add(usize::try_from(length).ok()?)?;
        next = chunked.then_some(end);
        data.get(start..end)
    }))
}

#[derive(Clone, Copy)]
pub(super) enum Generation {
    Record(u64),
    Legacy(u64),
}

fn map_key_is_version(data: &[u8], offset: usize) -> Option<bool> {
    let key = read_head(data, offset).ok()?;
    if key.major != 3 {
        return Some(false);
    }
    if let Some(length) = key.argument {
        return Some(length == 1 && data.get(key.end) == Some(&b'v'));
    }
    let mut chunk = key.end;
    let mut found = false;
    while data.get(chunk) != Some(&0xff) {
        let head = read_head(data, chunk).ok()?;
        match head.argument? {
            0 => chunk = head.end,
            1 if !found && data.get(head.end) == Some(&b'v') => {
                found = true;
                chunk = head.end + 1;
            }
            _ => return Some(false),
        }
    }
    Some(found)
}

pub(super) fn generation(first: &[u8]) -> Option<Generation> {
    let mut head = read_head(first, 0).ok()?;
    if head.major == 4 && head.argument.is_none_or(|length| length >= 2) {
        let kind = read_head(first, head.end).ok()?;
        if kind.major != 0 || !matches!(kind.argument, Some(1 | 5)) {
            return None;
        }
        let version = read_head(first, kind.end).ok()?;
        return (version.major == 0).then_some(Generation::Record(version.argument?));
    }
    if head.major == 6 && head.argument == Some(55_799) {
        head = read_head(first, head.end).ok()?;
    }
    if head.major != 5 {
        return None;
    }
    let end = first.len() - usize::from(head.argument.is_none());
    let mut key = head.end;
    while key < end {
        let value = item_end(first, key).ok()?;
        if map_key_is_version(first, key)? {
            let version = read_head(first, value).ok()?;
            return (version.major == 0).then_some(Generation::Legacy(version.argument?));
        }
        key = item_end(first, value).ok()?;
    }
    None
}
