# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""A zstd payload this reader cannot decode makes its one frame opaque (§6.1, §7.6)."""

from __future__ import annotations

from pathlib import Path

import pytest
import zstandard

from gts import read
from gts.cli import main
from gts.model import Term, TermKind
from gts.vectors import append_frame, blob_around, corpus, zstd_window_frame
from gts.wire import digest_str
from gts.writer import Writer

VECTORS = {case.name: case.data for case in corpus()}


def test_concatenated_frames_decode_to_the_end_of_the_payload() -> None:
    """A zstd-rsyncable payload of several frames yields every unit."""
    g = read(VECTORS["22b-zstd-rsyncable-multi-frame"])
    assert g.diagnostics == []
    (digest,) = g.blobs
    decoded = g.blobs[digest]
    assert decoded.startswith(b"unit 0: ")
    assert decoded.count(b"unit ") == 3
    assert digest == digest_str(decoded)


@pytest.mark.parametrize(
    "name", ["28q-blob-payload-not-zstd", "28r-blob-payload-truncated"]
)
def test_undecodable_blob_is_one_opaque_node_and_folding_continues(name: str) -> None:
    """The frame degrades alone: terms and quads before and after it are folded."""
    g = read(VECTORS[name])
    assert [o.reason for o in g.opaque] == ["damaged"]
    assert g.opaque[0].frame_type == "blob"
    assert [d.code for d in g.diagnostics] == ["DamagedFrame"]
    assert len(g.blobs) == 0
    assert len(g.terms) == 4
    assert len(g.quads) == 2


def test_blob_over_the_window_limit_is_opaque_not_a_failure_on_access() -> None:
    """A frame declaring a 2**28 window is refused whatever its content size is.

    The library decoded this exact hand-built frame, since its content is small and
    known; the reader now decides from the declaration alone (a deliberate change).
    """
    content = b"too much window"
    g = read(blob_around("zstd", zstd_window_frame(28, content), content))
    assert [o.reason for o in g.opaque] == ["damaged"]
    assert len(g.blobs) == 0
    assert len(g.quads) == 2


def test_blob_naming_a_dictionary_is_opaque() -> None:
    """No dictionary is implemented, so a frame that names one is refused at fold."""
    # flags: dictionary id of one byte; window descriptor; id 5; one raw block.
    frame = (
        bytes([0x28, 0xB5, 0x2F, 0xFD, 0x01, 0x00, 0x05, 0x29, 0x00, 0x00]) + b"hello"
    )
    g = read(blob_around("zstd", frame, b"hello"))
    assert [o.reason for o in g.opaque] == ["damaged"]
    assert len(g.blobs) == 0


def test_blob_at_the_window_limit_still_decodes() -> None:
    """A frame declaring exactly 2**27 is still read."""
    g = read(VECTORS["22c-zstd-declared-window-2-27"])
    assert g.opaque == []
    assert list(g.blobs.values()) == [b"a frame that declares a window of 2**27 bytes"]


def test_damage_inside_a_block_fails_cleanly_on_access(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    """Damage the frame walk cannot see ends as a message and status."""
    content = bytes(range(256)) * 300
    payload = bytearray(zstandard.ZstdCompressor(write_checksum=True).compress(content))
    payload[-1] ^= 0xFF  # the content checksum no longer matches
    w = Writer()
    w.add_terms([Term(TermKind.IRI, "https://example.org/a")])
    append_frame(
        w,
        {
            "t": "blob",
            "x": [2],
            "d": bytes(payload),
            "pub": {"digest": digest_str(content), "mt": "text/plain"},
        },
    )
    path = tmp_path / "damaged-inside.gts"
    path.write_bytes(w.to_bytes())
    assert read(w.to_bytes()).opaque == []  # the frame structure is intact
    assert main(["ls", str(path)]) == 1
    err = capsys.readouterr().err
    assert "cannot decode blob" in err
    assert "Traceback" not in err
    assert main(["extract", str(path), digest_str(content)]) == 1
    assert "cannot decode blob" in capsys.readouterr().err


def test_undecodable_blob_makes_the_cli_report_not_crash(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    """ls on a file holding a refused payload reports it and does not crash."""
    path = tmp_path / "not-zstd.gts"
    path.write_bytes(VECTORS["28q-blob-payload-not-zstd"])
    assert main(["ls", str(path)]) == 0
    captured = capsys.readouterr()
    assert "DamagedFrame" in captured.err
    assert "Traceback" not in captured.err
