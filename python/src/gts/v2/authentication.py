# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""P6 coverage and authentication over checked v2 frame links and commits."""

from collections.abc import Iterable
from dataclasses import dataclass
from typing import Literal

from .frame import Signature
from .link import FrameLink
from .signatures import SignatureOutcome, SignatureVerifier

type Authenticity = Literal["authenticated", "partly_authenticated", "unauthenticated"]


@dataclass(frozen=True, slots=True)
class AuthenticationFrame:
    position: int
    frame_type: int | None
    link: FrameLink
    signatures: tuple[Signature, ...] = ()
    transparent: bool = False
    detached: bool = False
    valid_commit: bool = False


@dataclass(frozen=True, slots=True)
class SignatureResult:
    outcome: SignatureOutcome
    algorithm: int
    key_id: bytes
    coverage_first: int
    coverage_end: int
    authenticated: bool


@dataclass(frozen=True, slots=True)
class SegmentAuthentication:
    signatures: tuple[tuple[int, tuple[SignatureResult, ...]], ...]
    authenticated_positions: frozenset[int]
    authenticated_frames: int
    authenticity: Authenticity


def authenticate_segment(
    opener: bytes,
    frames: Iterable[AuthenticationFrame],
    verifier: SignatureVerifier,
) -> SegmentAuthentication:
    """Judge frame signatures and compute authentication in one segment.

    Supply frames in item order, including broken ordinary links. Commit
    validity is the P5 count, root, grammar, and link verdict. Registry versions
    held here retire no hash or signature algorithm.
    """
    signature_results = []
    ordinary: list[tuple[AuthenticationFrame, int]] = []
    run_ends: dict[int, int] = {}
    run_first = 0
    last_commit = -1
    for frame in frames:
        if frame.transparent:
            continue
        position = len(ordinary)
        identifier = frame.link.effective_identifier
        if identifier is None:
            run_first = position
            continue
        holds = frame.link.state != "broken"
        if frame.detached:
            run_first = position
        ordinary.append((frame, run_first))
        results: list[SignatureResult] = []
        for signature in frame.signatures:
            outcome = verifier.verify(signature, opener, identifier)
            covered = holds and outcome.status == "valid"
            trusted = covered and verifier.trusts(signature.key_id)
            authenticated = trusted and (frame.frame_type != 14 or frame.valid_commit)
            results.append(
                SignatureResult(
                    outcome,
                    signature.algorithm,
                    signature.key_id,
                    run_first if covered else position,
                    position + 1 if covered else position,
                    authenticated,
                )
            )
            if authenticated:
                if frame.frame_type == 14:
                    last_commit = position
                else:
                    run_ends[run_first] = position + 1
        if results:
            signature_results.append((frame.position, tuple(results)))
        if not holds:
            run_first = position + 1

    authenticated_positions: set[int] = set()
    counted = authenticated_frames = 0
    for position, (frame, start) in enumerate(ordinary):
        authenticated = frame.link.state != "broken" and (
            position <= last_commit or position < run_ends.get(start, 0)
        )
        if authenticated:
            authenticated_positions.add(frame.position)
        if frame.frame_type != 15:
            counted += 1
            authenticated_frames += authenticated
    authenticity: Authenticity = "unauthenticated"
    if authenticated_frames:
        authenticity = (
            "authenticated"
            if counted == authenticated_frames
            else "partly_authenticated"
        )
    return SegmentAuthentication(
        tuple(signature_results),
        frozenset(authenticated_positions),
        authenticated_frames,
        authenticity,
    )
