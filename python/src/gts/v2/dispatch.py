# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Select the first GTS segment or legacy reader by P1 §§2.3 and 6.7."""

from dataclasses import dataclass, replace
from typing import Literal

from .framing import FramingReason, ItemFramingError, _generation_field, _head, item_end
from .header import DecodedHeader, HeaderError, decode_header
from .payload import PayloadBudget
from .resync import (
    RESYNC_MAX,
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
    if type(items_per_read) is not int or not 0 <= items_per_read < 1 << 64:
        raise ValueError("items_per_read must be a uint")
    if not data:
        return FileDispatch(None, 0, finding=DispatchFinding("empty_input"))
    if items_per_read == 0:
        return FileDispatch(
            None, 0, finding=DispatchFinding("item_count", 0, len(data), 1)
        )

    try:
        first_end = item_end(data)
    except ItemFramingError as error:
        failed = FileDispatch(
            None, 0, finding=DispatchFinding(error.reason, 0, len(data))
        )
        return _recover(
            data,
            budget,
            failed,
            len(data),
            int(error.reason == "undelimitable_input"),
            items_per_read,
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
        decoder = decode_header if kind == 1 else decode_resync
        try:
            return FileDispatch(2, first_end, opener=decoder(data), opener_offset=0)
        except (HeaderError, ResyncError) as error:
            failed = FileDispatch(2, first_end, opener=error, opener_offset=0)
    else:
        failed = FileDispatch(
            None, 0, finding=DispatchFinding("not_this_format", 0, first_end)
        )
    return _recover(data, budget, failed, first_end, 1, items_per_read)


def _unsupported(first_end: int) -> FileDispatch:
    return FileDispatch(
        None, 0, finding=DispatchFinding("unsupported_generation", 0, first_end)
    )


def _recover(
    data: bytes,
    budget: PayloadBudget,
    failed: FileDispatch,
    first_end: int,
    delimited: int,
    limit: int,
) -> FileDispatch:
    if not _scan_admitted(data, first_end, delimited, limit):
        return failed

    result = ResyncScanner(data, budget).scan(0, first_end)
    if result is None:
        return failed
    if isinstance(result, ResyncRefusal):
        return replace(failed, scan_refusal=result)

    gap = None
    if result.offset > 0:
        prefix = memoryview(data)[: result.offset]
        try:
            item_end(prefix)
        except ItemFramingError as error:
            gap = DispatchFinding(error.reason, 0, result.offset)
    return FileDispatch(
        2, result.next_offset, result, result.offset, scanned=True, finding=gap
    )


def _scan_admitted(data: bytes, after: int, count: int, limit: int) -> bool:
    """Count items delimited by the initial run's framing-only lookahead."""
    stop = min(len(data), after + RESYNC_MAX)
    view = memoryview(data)
    position = after
    while position < stop and count < limit:
        # The item immediately after a complete failure is held whole.
        bound = len(data) if position == after else stop
        try:
            position = item_end(view[:bound], position)
        except ItemFramingError as error:
            count += int(error.reason == "undelimitable_input")
            break
        count += 1
    return count < limit
