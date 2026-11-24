# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Select the first GTS segment or legacy reader by P1 §§2.3 and 6.7."""

from dataclasses import dataclass, replace
from typing import Literal

from ._item_walk import ItemCountReached, ItemWalk
from .framing import FramingReason, ItemFramingError, _generation_field, _head, item_end
from .header import DecodedHeader, HeaderError, decode_header
from .payload import PayloadBudget
from .resync import (
    DecodedResync,
    ResyncError,
    ResyncRefusal,
    ResyncScanner,
    decode_resync,
)

type DispatchReason = (
    Literal["empty_input", "not_this_format", "unsupported_generation", "item_count"]
    | FramingReason
)
type Opener = DecodedHeader | DecodedResync | HeaderError | ResyncError


@dataclass(frozen=True, slots=True)
class DispatchFinding:
    reason: DispatchReason
    offset: int | None = None
    length: int | None = None
    count: int | None = None


@dataclass(frozen=True, slots=True)
class FileDispatch:
    """The reader selection and any opener or finding at the start of a file."""

    generation: Literal[1, 2] | None
    next_offset: int
    opener: Opener | None = None
    opener_offset: int | None = None
    scanned: bool = False
    finding: DispatchFinding | None = None
    scan_refusal: ResyncRefusal | None = None


def dispatch_file(
    data: bytes,
    budget: PayloadBudget,
    *,
    items_per_read: int = 1 << 26,
    first_and_second: bool = False,
) -> FileDispatch:
    """Dispatch from the first item, allowing resync only where P1 permits it.

    The caller owns the budget so recovery charges carry into later checks.
    On generation 1, pass the original input to the legacy reader.
    """
    return _dispatch(
        data,
        ItemWalk(data, items_per_read),
        ResyncScanner(data, budget),
        first_and_second=first_and_second,
    )


def _dispatch(
    data: bytes, walk: ItemWalk, scanner: ResyncScanner, *, first_and_second: bool
) -> FileDispatch:
    if not data:
        return FileDispatch(None, 0, finding=DispatchFinding("empty_input"))

    try:
        first_end = walk.take(0)
    except ItemCountReached:
        return FileDispatch(
            None, 0, finding=DispatchFinding("item_count", 0, len(data), 1)
        )
    except ItemFramingError as error:
        failed = FileDispatch(
            None, 0, finding=DispatchFinding(error.reason, 0, len(data))
        )
        return _recover(
            data,
            walk,
            scanner,
            failed,
            len(data),
        )

    generation = _generation_field(data, first_end)
    if generation is not None and generation.grammar == "first_generation":
        if generation.value == 1 and first_and_second:
            return FileDispatch(1, 0)
        return _unsupported(first_end)
    if generation is not None and generation.value >= 3:
        return _unsupported(first_end)
    if generation is not None and generation.value == 2:
        _, _, kind_offset = _head(data, 0, 0)
        _, kind, _ = _head(data, kind_offset, 0)
        opener = _decode_opener(data, 0, resync=kind == 5)
        failed = FileDispatch(2, first_end, opener=opener, opener_offset=0)
        if not isinstance(opener, HeaderError | ResyncError):
            return failed
    else:
        failed = FileDispatch(
            None, 0, finding=DispatchFinding("not_this_format", 0, first_end)
        )
    return _recover(data, walk, scanner, failed, first_end)


def _decode_opener(data: bytes, offset: int, *, resync: bool) -> Opener:
    decoder = decode_resync if resync else decode_header
    try:
        return decoder(data, offset)
    except (HeaderError, ResyncError) as error:
        return error


def _unsupported(first_end: int) -> FileDispatch:
    return FileDispatch(
        None, 0, finding=DispatchFinding("unsupported_generation", 0, first_end)
    )


def _recover(
    data: bytes,
    walk: ItemWalk,
    scanner: ResyncScanner,
    failed: FileDispatch,
    first_end: int,
) -> FileDispatch:
    if not walk.scan_admitted(first_end):
        return failed

    result = scanner.scan(0, first_end)
    if result is None:
        return failed
    if isinstance(result, ResyncRefusal):
        return replace(failed, scan_refusal=result)

    walk.accept_resync()
    return FileDispatch(
        2,
        result.next_offset,
        result,
        result.offset,
        scanned=True,
        finding=_gap_finding(data, 0, result.offset),
    )


def _gap_finding(data: bytes, start: int, end: int) -> DispatchFinding | None:
    if end > start:
        try:
            item_end(memoryview(data)[:end], start)
        except ItemFramingError as error:
            return DispatchFinding(error.reason, start, end - start)
    return None
