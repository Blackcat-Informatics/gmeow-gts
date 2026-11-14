# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Tests for the GTS transform catalog (§8), including rsyncable codecs."""

from __future__ import annotations

import gzip

import pytest
import zstandard

from gts.codec import (
    Codec,
    CodecClass,
    PayloadDecodeError,
    check_zstd_frames,
    decode_chain,
    encode_chain,
)


def _rdf_snapshot_payload(rows: int = 140_000) -> bytes:
    out = bytearray()
    for i in range(rows):
        out.extend(
            (
                f"<https://example.org/s/{i % 5000}> "
                f"<https://example.org/p/{i % 40}> "
                f'"value {i % 1000:04d} repeated repeated repeated" '
                f"<https://example.org/g/{i % 10}> .\n"
            ).encode()
        )
    return bytes(out)


@pytest.mark.parametrize("codec_name", ["identity", "gzip", "zstd", "zstd-rsyncable"])
def test_round_trip(codec_name: str) -> None:
    """Every baseline codec round-trips arbitrary bytes."""
    data = b"Hello world " * 1000 + bytes(range(256))
    encoded = encode_chain([codec_name], data)
    cls: CodecClass = "encode" if codec_name == "identity" else "compress"
    decoded = decode_chain([Codec(codec_name, cls)], encoded)
    assert decoded == data


def test_zstd_rsyncable_localizes_changes() -> None:
    """A single-byte mutation only affects the compressed block it lives in.

    Because each 64 KiB chunk is an independent zstd frame, a change at the
    middle of the payload leaves the first half and last half of the
    compressed bytes byte-for-byte identical.
    """
    block_size = 65536
    size = 4 * block_size
    # Use a payload with enough entropy to be mildly compressible but not
    # trivially tiny when compressed.
    base = (b"The quick brown fox jumps over the lazy dog. " * 6000)[:size]
    mid = len(base) // 2
    mutated = base[:mid] + bytes([(base[mid] + 1) % 256]) + base[mid + 1 :]

    rsync_base = encode_chain(["zstd-rsyncable"], base)
    rsync_mut = encode_chain(["zstd-rsyncable"], mutated)

    # With independent blocks, the first two blocks (before mid) and the last
    # block (after mid) are identical between the two compressed streams.
    # Identify block boundaries by walking from both ends until a difference
    # appears.
    prefix_match = 0
    for a, b in zip(rsync_base, rsync_mut, strict=False):
        if a == b:
            prefix_match += 1
        else:
            break

    tail_match = 0
    for a, b in zip(reversed(rsync_base), reversed(rsync_mut), strict=False):
        if a == b:
            tail_match += 1
        else:
            break

    assert prefix_match > 0, "prefix before the change should be identical"
    assert tail_match > 0, "tail after the change should be identical"
    # The changed region is bounded by roughly one compressed block, not the
    # whole remainder of the stream.
    affected = min(len(rsync_base), len(rsync_mut)) - prefix_match - tail_match
    assert affected < len(rsync_base) // 2, (
        f"affected region {affected} bytes should be much smaller than "
        f"stream length {len(rsync_base)}"
    )


def test_zstd_rsyncable_decodes_via_zstd_path() -> None:
    """zstd-rsyncable output is valid zstd and decodes through the shared path."""
    data = b"rsyncable payload " * 5000
    encoded = encode_chain(["zstd-rsyncable"], data)
    # The codec's decode path treats zstd-rsyncable as zstd-compatible.
    decoded = decode_chain([Codec("zstd-rsyncable", "compress")], encoded)
    assert decoded == data


@pytest.mark.parametrize("codec_name", ["zstd", "zstd-rsyncable"])
def test_zstd_level_is_per_chain(codec_name: str) -> None:
    """A caller-selected zstd level applies to both zstd codec names."""
    data = b'<https://ex/s> <https://ex/p> "repeat repeat repeat" .\n' * 4096

    fast = encode_chain([codec_name], data, zstd_level=1)
    high = encode_chain([codec_name], data, zstd_level=19)

    assert len(high) <= len(fast)
    assert decode_chain([Codec(codec_name, "compress")], high) == data


def test_high_level_zstd_rsyncable_beats_gzip9_on_rdf_snapshot() -> None:
    """Level 19 zstd-rsyncable stays at or below gzip -9 for a large RDF frame."""
    data = _rdf_snapshot_payload()
    assert 16 * 1024 * 1024 <= len(data) <= 18 * 1024 * 1024

    gzip9 = gzip.compress(data, compresslevel=9, mtime=0)
    zstd_rsyncable_19 = encode_chain(["zstd-rsyncable"], data, zstd_level=19)

    assert len(zstd_rsyncable_19) <= len(gzip9)
    decoded = decode_chain([Codec("zstd-rsyncable", "compress")], zstd_rsyncable_19)
    assert decoded == data


@pytest.mark.parametrize("codec_name", ["zstd", "zstd-rsyncable"])
def test_zstd_decode_accepts_outputs_over_former_safety_bound(
    codec_name: str,
) -> None:
    """The shared zstd decode path accepts payloads above the old 16 MiB cap."""
    data = b"\0" * (16 * 1024 * 1024 + 1)
    encoded = encode_chain([codec_name], data)

    assert decode_chain([Codec(codec_name, "compress")], encoded) == data


def test_gzip_encoding_uses_zero_mtime() -> None:
    """Committed gzip-coded frames must not depend on wall-clock time."""
    encoded = encode_chain(["gzip"], b"stable gzip payload" * 100)

    assert encoded[4:8] == b"\x00\x00\x00\x00"
    assert encode_chain(["gzip"], b"stable gzip payload" * 100) == encoded


def _window_frame(window_log: int, content: bytes) -> bytes:
    """A zstd frame declaring a ``2**window_log`` window over one raw block."""
    block = (1 | (len(content) << 3)).to_bytes(3, "little")
    return b"\x28\xb5\x2f\xfd\x00" + bytes([(window_log - 10) << 3]) + block + content


def test_zstd_frame_walk_accepts_every_frame_shape_the_library_writes() -> None:
    """Frames with or without content size or checksum, empty, skippable: all pass."""
    plain = b"abc" * 70000
    frames = [
        zstandard.ZstdCompressor().compress(plain),
        zstandard.ZstdCompressor(write_content_size=False).compress(plain),
        zstandard.ZstdCompressor(write_checksum=True, level=19).compress(plain),
        zstandard.ZstdCompressor().compress(b""),
        b"\x50\x2a\x4d\x18\x03\x00\x00\x00abc",
    ]
    for frame in frames:
        check_zstd_frames(frame)
    payload = b"".join(frames)
    check_zstd_frames(payload)
    assert decode_chain([Codec("zstd", "compress")], payload) == plain * 3


@pytest.mark.parametrize("cut", [1, 3, 9, 20, -1])
def test_zstd_frame_walk_refuses_a_payload_that_ends_inside_a_frame(cut: int) -> None:
    """A payload cut short is an error; the streaming reader gave short bytes."""
    frame = zstandard.ZstdCompressor().compress(b"y" * 5000 + bytes(range(256)) * 40)
    cut_frame = frame[:cut]
    with pytest.raises(PayloadDecodeError):
        check_zstd_frames(cut_frame)
    with pytest.raises(PayloadDecodeError):
        decode_chain([Codec("zstd", "compress")], cut_frame)


def test_zstd_frame_walk_refuses_bytes_that_are_not_zstd() -> None:
    """A payload that does not start with a zstd or skippable magic is refused."""
    with pytest.raises(PayloadDecodeError, match="not a zstd frame"):
        check_zstd_frames(b"not zstd frame data")


@pytest.mark.parametrize(
    ("window_log", "accepted"), [(27, True), (28, False), (31, False)]
)
def test_zstd_window_above_two_to_the_27_is_refused_before_decoding(
    window_log: int, accepted: bool
) -> None:
    """A header declaring over 2**27 is refused whatever the content; 2**27 decodes."""
    frame = _window_frame(window_log, b"declared window")
    if accepted:
        assert decode_chain([Codec("zstd", "compress")], frame) == b"declared window"
        return
    with pytest.raises(PayloadDecodeError, match="window"):
        check_zstd_frames(frame)
