// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use std::{fs, path::Path};

use gmeow_gts::v2::{
    decode_header,
    reader::{read_wire, LinkState, PayloadState, ReadCapacity},
    writer::{AttachmentWriteError, Blob, SegmentWriter},
    Header,
};

fn vector(name: &str) -> Vec<u8> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../vectors/v2/P4")
        .join(name);
    fs::read(&path).unwrap_or_else(|error| panic!("read {}: {error}", path.display()))
}

fn xof(seed: &[u8], length: usize) -> Vec<u8> {
    let mut octets = vec![0; length];
    blake3::Hasher::new()
        .update(seed)
        .finalize_xof()
        .fill(&mut octets);
    octets
}

fn present<'a>(bytes: &'a [u8], media_types: &'a [&'a str]) -> Blob<'a> {
    Blob {
        digest: blake3::derive_key("gts2 unit", bytes),
        length: bytes.len() as u64,
        bytes: Some(bytes),
        media_types,
    }
}

fn match_attachment_prefix<'a>(
    vector_name: &str,
    values: impl IntoIterator<Item = &'a [u8]>,
    blobs: impl IntoIterator<Item = Blob<'a>>,
    count: usize,
) {
    let published = vector(vector_name);
    let header = decode_header(&published, 0)
        .expect("published header")
        .header;
    let mut writer = SegmentWriter::new(header).expect("write header");
    writer
        .append_attachment_regions(values, blobs)
        .expect("write attachment regions");

    let expected = read_wire(&published, ReadCapacity::default());
    let actual = read_wire(writer.as_bytes(), ReadCapacity::default());
    let frames = &actual.segments[0].frames;
    assert_eq!(frames.len(), count, "{vector_name}");
    assert!(actual.stop.is_none(), "{vector_name}");
    for (written, reference) in frames.iter().zip(&expected.segments[0].frames[..count]) {
        let frame = written.envelope.as_ref().expect("written envelope");
        let want = reference.envelope.as_ref().expect("published envelope");
        assert!(matches!(want.frame_type, 12 | 13));
        assert_eq!(frame.frame_type, want.frame_type);
        assert_eq!(frame.content, want.content, "{vector_name}");
        assert_eq!(frame.identifier, want.identifier, "{vector_name}");
        assert_eq!(written.link.state, LinkState::Linked, "{vector_name}");
        let (PayloadState::Verified(decoded), PayloadState::Verified(expected)) =
            (&written.payload.state, &reference.payload.state)
        else {
            panic!("unverified payload in {vector_name}");
        };
        assert!(decoded == expected, "payload differs in {vector_name}");
    }
    assert_eq!(
        writer.head(),
        frames.last().unwrap().envelope.as_ref().unwrap().identifier
    );
}

#[test]
fn blob_members_promote_shared_units_and_preserve_empty_blobs() {
    let data = xof(b"P4 first emitted", 150_000);
    match_attachment_prefix(
        "canon-one-unit-blob-first-emitted.bin",
        [],
        [present(&data[..105_745], &["m/t"]), present(&data, &[])],
        1,
    );
    match_attachment_prefix("canon-empty-blob.bin", [], [present(b"", &[])], 1);
}

#[test]
fn value_and_blob_regions_store_shared_units_separately() {
    let data = xof(b"P4 canon shared", 400_000);
    let first = 48_824;
    let second = first + 73_555;
    let third = second + 80_312;
    let mut octets = data[first..second].to_vec();
    octets.extend(xof(b"P4 canon shared value", 524_288));
    let external = Blob {
        digest: [0x7e; 32],
        length: 9,
        bytes: None,
        media_types: &[],
    };
    match_attachment_prefix(
        "canon-regions-shared.bin",
        [octets.as_slice(), octets.as_slice()],
        [
            present(&data[..second], &["x/1"]),
            present(&data[second..third], &["b/b", "a/a", "a/a"]),
            external,
            present(&data[first..third], &[]),
        ],
        4,
    );
    match_attachment_prefix(
        "canon-media-types.bin",
        [],
        [present(
            b"three media types",
            &["zz/top-level", "text/plain", "a/b"],
        )],
        3,
    );
}

#[test]
fn packs_cut_on_body_length_at_the_published_boundary() {
    let table = vector("chunker-table.bin");
    let blocks: Vec<_> = table
        .as_chunks::<8>()
        .0
        .iter()
        .zip(0_u8..=255)
        .filter(|(word, _)| u64::from_be_bytes(**word).wrapping_neg() >= 1 << 50)
        .take(64)
        .map(|(_, octet)| vec![octet; 262_144])
        .collect();
    let blobs: Vec<_> = blocks.iter().map(|bytes| present(bytes, &[])).collect();
    match_attachment_prefix("canon-pack-cut.64-blobs.bin", [], blobs.iter().copied(), 1);
    match_attachment_prefix(
        "canon-pack-cut.65-blobs.bin",
        [],
        blobs.into_iter().chain([present(&[1], &[])]),
        2,
    );
}

#[test]
fn invalid_attachment_inputs_leave_the_committed_prefix_unchanged() {
    let header = Header::default();
    let mut writer = SegmentWriter::new(header).expect("header");
    let mut expected = SegmentWriter::new(header).expect("header");
    let mut blob = present(b"one unit", &[]);
    blob.digest[0] ^= 1;
    assert_eq!(
        writer.append_attachment_regions([b"nonempty".as_slice()], [blob]),
        Err(AttachmentWriteError::BlobDigest),
    );
    assert_eq!(writer.commit().unwrap(), expected.commit().unwrap());
    assert_eq!(writer.as_bytes(), expected.as_bytes());
}
