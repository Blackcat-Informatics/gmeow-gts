# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Term storage records, front coding and formed-byte capacity (P3 §§1.5, 2.1–2.5)."""

from collections.abc import Callable, Iterable, Sequence
from dataclasses import dataclass
from datetime import datetime, timedelta
from itertools import groupby
from typing import Literal

import cbor2

from ._content import UINT_MAX, ContentError, content_code
from ._decode import _Decoder
from .framing import ItemFramingError, _head, item_end

_BLOCK_SIZES = (4, 7, 2, 4, 6, 2, 3)
_OLDEST_SECOND = -62135596800
_LATEST_SECOND = 253402300799
_MAX_NANOSECOND = 999999999
_EPOCH = datetime(1970, 1, 1)
_XSD = "http://www.w3.org/2001/XMLSchema#"
_VALUE_DATATYPES = ("boolean", "nonNegativeInteger", "integer", "dateTime", "hexBinary")
type Direction = Literal["ltr", "rtl"]


@dataclass(frozen=True, slots=True)
class ValueReference:
    digest: bytes
    length: int


@dataclass(frozen=True, slots=True)
class IriTerm:
    text: bytes


@dataclass(frozen=True, slots=True)
class LiteralTerm:
    lexical: bytes | ValueReference
    datatype: int | None = None
    language: str | None = None
    direction: Direction | None = None


@dataclass(frozen=True, slots=True)
class BlankTerm:
    label: bytes


@dataclass(frozen=True, slots=True)
class TripleTerm:
    subject: int
    predicate: int
    object: int


@dataclass(frozen=True, slots=True)
class TimeValue:
    seconds: int
    nanoseconds: int


@dataclass(frozen=True, slots=True)
class TypedValue:
    kind: int
    value: bool | int | TimeValue | bytes

    def literal(self) -> tuple[str, str]:
        """Generate the lexical form and datatype of a P3 typed value."""
        encoded = _value_record(self)
        value = self.value
        if isinstance(value, TimeValue):
            date = _EPOCH + timedelta(seconds=value.seconds)
            fraction = (
                f".{value.nanoseconds:09d}".rstrip("0") if value.nanoseconds else ""
            )
            lexical = date.isoformat(timespec="seconds") + fraction + "Z"
        elif isinstance(value, bytes):
            lexical = value.hex().upper()
        elif isinstance(value, bool):
            lexical = "true" if value else "false"
        else:
            lexical = str(encoded[1])
        return lexical, _XSD + _VALUE_DATATYPES[self.kind - 1]


@dataclass(frozen=True, slots=True)
class TypedValueTerm:
    value: TypedValue | ValueReference


type WireTerm = IriTerm | LiteralTerm | BlankTerm | TripleTerm | TypedValueTerm


@dataclass(frozen=True, slots=True)
class DecodedTerms:
    id_base: int
    restart_group: int
    canonical: bool
    terms: tuple[WireTerm, ...]


class FormedCapacityError(ValueError):
    """The attempted formed-byte total after applying the uint bound."""

    def __init__(self, count: int) -> None:
        self.count = count
        super().__init__(f"total formed length refused at {count} bytes")


class FormedBudget:
    """Track front-coded prefixes copied across all frames of a read."""

    def __init__(self, formed_bytes_per_read: int) -> None:
        _uint(formed_bytes_per_read)
        self._limit = formed_bytes_per_read
        self._used = 0
        self._refused = False

    def _charge(self, length: int) -> None:
        total = self._used + length
        if self._refused or total > self._limit:
            self._refused = True
            raise FormedCapacityError(min(total, UINT_MAX))
        self._used = total


def _uint(value: int) -> None:
    if type(value) is not int or not 0 <= value <= UINT_MAX:
        raise ValueError("term field must be a uint")


def _read_runs(
    decoder: _Decoder, base: int, registry_version: int, *, reserve: bool = False
) -> list[tuple[int, int]]:
    start = decoder.offset
    size = decoder.array()
    if size == 0:
        raise ContentError("out_of_range", start)
    runs: list[tuple[int, int]] = []
    for _ in range(size):
        decoder.record(2)
        offset = decoder.offset
        if reserve and registry_version > 1:
            kind = decoder.uint(minimum=1, reason="unregistered_value")
        else:
            kind = content_code(decoder, 7, registry_version)
        if runs and runs[-1][0] == kind:
            raise ContentError("not_deterministic", offset)
        runs.append((kind, decoder.uint(minimum=1)))
    if base + sum(count for _, count in runs) > UINT_MAX + 1:
        raise ContentError("out_of_range", start)
    return runs


def reserved_term_runs(
    content: bytes | memoryview, *, expected_base: int, registry_version: int = 1
) -> tuple[tuple[int, int], ...]:
    """Read the runs for ID reservation even when other frame fields fail."""
    data = bytes(content)
    try:
        major, count, base_at = _head(data, 0, 0)
        if major != 4 or (count is not None and count < 2):
            return ()
        runs_at = item_end(data, base_at)
        decoder = _Decoder(data, runs_at, ContentError)
        return tuple(_read_runs(decoder, expected_base, registry_version, reserve=True))
    except (ContentError, ItemFramingError):
        return ()


def reserved_term_count(
    content: bytes | memoryview, *, expected_base: int, registry_version: int = 1
) -> int:
    """Count the IDs reserved by the runs field."""
    return sum(
        count
        for _, count in reserved_term_runs(
            content,
            expected_base=expected_base,
            registry_version=registry_version,
        )
    )


def _array[T](decoder: _Decoder, read: Callable[[], T]) -> list[T]:
    return [read() for _ in range(decoder.array())]


@dataclass(frozen=True, slots=True)
class _FrontCoding:
    prefixes: list[int]
    lengths: list[int]
    suffixes: memoryview

    def expand(self, restart: int, offset: int) -> list[bytes]:
        if sum(self.lengths) != len(self.suffixes):
            raise ContentError("inconsistent_counts", offset)
        previous = b""
        start = 0
        texts = []
        for index, (prefix, length) in enumerate(
            zip(self.prefixes, self.lengths, strict=True)
        ):
            if index and prefix > len(previous):
                raise ContentError("out_of_range", offset)
            if index % restart == 0:
                if prefix:
                    raise ContentError("forbidden_combination", offset)
            elif (
                prefix < len(previous)
                and length
                and previous[prefix] == self.suffixes[start]
            ):
                raise ContentError("not_deterministic", offset)
            end = start + length
            text = previous[:prefix] + self.suffixes[start:end]
            texts.append(text)
            previous = text
            start = end
        return texts


@dataclass(frozen=True, slots=True)
class _Forms:
    forms: list[int]
    datatypes: list[int]
    indexes: list[int]

    def literals(
        self, lexical: Iterable[bytes | ValueReference], languages: Sequence[str]
    ) -> Iterable[LiteralTerm]:
        datatype_ids = iter(self.datatypes)
        language_indexes = iter(self.indexes)
        for value, form in zip(lexical, self.forms, strict=True):
            if form == 1:
                yield LiteralTerm(value, datatype=next(datatype_ids))
            else:
                direction: Direction | None = None
                if form == 3:
                    direction = "ltr"
                elif form == 4:
                    direction = "rtl"
                yield LiteralTerm(
                    value,
                    language=languages[next(language_indexes)],
                    direction=direction,
                )


@dataclass(frozen=True, slots=True)
class _Block:
    kind: int
    offset: int
    counts: tuple[int, ...]
    make: Callable[[Sequence[bytes], Sequence[str]], Iterable[WireTerm]]
    front: _FrontCoding | None = None
    forms: _Forms | None = None

    def check_counts(self, expected: int) -> None:
        if any(count != expected for count in self.counts):
            raise ContentError("inconsistent_counts", self.offset)
        if self.forms is not None:
            typed = self.forms.forms.count(1)
            if (
                len(self.forms.datatypes) != typed
                or len(self.forms.indexes) != expected - typed
            ):
                raise ContentError("inconsistent_counts", self.offset)


def _read_value(decoder: _Decoder, registry_version: int) -> TypedValue:
    offset = decoder.offset
    size = decoder.array()
    if not size:
        raise ContentError("wrong_length", offset)
    kind = content_code(decoder, 5, registry_version)
    if size != (3 if kind == 4 else 2):
        raise ContentError("wrong_length", offset)
    value: bool | int | TimeValue | bytes
    match kind:
        case 1:
            value = decoder.boolean()
        case 2:
            value = decoder.uint()
        case 3:
            value = decoder.integer()
        case 4:
            seconds_at = decoder.offset
            seconds = decoder.integer()
            if not _OLDEST_SECOND <= seconds <= _LATEST_SECOND:
                raise ContentError("out_of_range", seconds_at)
            value = TimeValue(seconds, decoder.uint(maximum=_MAX_NANOSECOND))
        case _:
            value = bytes(decoder.bytes())
    return TypedValue(kind, value)


def _read_forms(decoder: _Decoder, registry_version: int) -> _Forms:
    return _Forms(
        _array(decoder, lambda: content_code(decoder, 4, registry_version)),
        _array(decoder, decoder.uint),
        _array(decoder, decoder.uint),
    )


def _read_block(decoder: _Decoder, registry_version: int) -> _Block:
    offset = decoder.offset
    size = decoder.array()
    if not size:
        raise ContentError("wrong_length", offset)
    kind = content_code(decoder, 7, registry_version)
    if size != _BLOCK_SIZES[kind - 1]:
        raise ContentError("wrong_length", offset)
    if kind in (1, 2):
        front = _FrontCoding(
            _array(decoder, decoder.uint),
            _array(decoder, decoder.uint),
            decoder.bytes(),
        )
        counts = (len(front.prefixes), len(front.lengths))
        if kind == 1:
            return _Block(
                kind, offset, counts, lambda texts, _: map(IriTerm, texts), front
            )
        forms = _read_forms(decoder, registry_version)
        return _Block(
            kind, offset, (*counts, len(forms.forms)), forms.literals, front, forms
        )
    if kind == 3:
        labels = _array(decoder, lambda: bytes(decoder.bytes()))
        return _Block(
            kind, offset, (len(labels),), lambda _texts, _: map(BlankTerm, labels)
        )
    if kind == 4:
        subjects = _array(decoder, decoder.uint)
        predicates = _array(decoder, decoder.uint)
        objects = _array(decoder, decoder.uint)
        return _Block(
            kind,
            offset,
            (len(subjects), len(predicates), len(objects)),
            lambda _texts, _: map(TripleTerm, subjects, predicates, objects),
        )
    if kind == 6:
        values = _array(decoder, lambda: _read_value(decoder, registry_version))
        return _Block(
            kind, offset, (len(values),), lambda _texts, _: map(TypedValueTerm, values)
        )

    digests = _array(decoder, lambda: bytes(decoder.bytes(32)))
    lengths = _array(decoder, decoder.uint)
    counts = (len(digests), len(lengths))
    if kind == 7:
        return _Block(
            kind,
            offset,
            counts,
            lambda _texts, _: map(
                TypedValueTerm, map(ValueReference, digests, lengths)
            ),
        )
    forms = _read_forms(decoder, registry_version)
    return _Block(
        kind,
        offset,
        (*counts, len(forms.forms)),
        lambda _texts, languages: forms.literals(
            map(ValueReference, digests, lengths), languages
        ),
        forms=forms,
    )


def decode_terms(
    content: bytes | memoryview,
    payload: bytes | memoryview,
    *,
    expected_base: int,
    budget: FormedBudget,
    registry_version: int = 1,
) -> DecodedTerms:
    """Decode the storage forms of a term frame after its wire checks.

    Retain reconstructed text octets for the dictionary's per-term checks.
    Pass a single formed-byte budget across all term frames of the read.
    """
    record = _Decoder(bytes(content), 0, ContentError)
    record.record(4)
    offset = record.offset
    base = record.uint()
    if base != expected_base:
        raise ContentError("inconsistent_counts", offset)
    runs = _read_runs(record, expected_base, registry_version)
    canonical = content_code(record, 2, registry_version) == 1
    restart = record.uint(minimum=1)
    if record.offset != len(content):
        raise ContentError("wrong_length", record.offset)

    decoder = _Decoder(bytes(payload), 0, ContentError)
    decoder.record(2)
    blocks_at = decoder.offset
    blocks = _array(decoder, lambda: _read_block(decoder, registry_version))
    languages_at = decoder.offset
    languages = _array(decoder, decoder.text_value)
    if decoder.offset != len(payload):
        raise ContentError("wrong_length", decoder.offset)
    if len(blocks) != len(runs):
        raise ContentError("inconsistent_counts", blocks_at)
    for block, (_, count) in zip(blocks, runs, strict=True):
        block.check_counts(count)
    for block, (kind, _) in zip(blocks, runs, strict=True):
        if block.kind != kind:
            raise ContentError("forbidden_combination", block.offset)

    budget._charge(sum(sum(b.front.prefixes) for b in blocks if b.front is not None))
    texts = [
        b.front.expand(restart, b.offset) if b.front is not None else [] for b in blocks
    ]
    used: dict[int, None] = {}
    for block in blocks:
        if block.forms is not None:
            for index in block.forms.indexes:
                if index >= len(languages):
                    raise ContentError("out_of_range", block.offset)
                used[index] = None
    if list(used) != list(range(len(languages))) or len(set(languages)) != len(
        languages
    ):
        raise ContentError("not_deterministic", languages_at)
    terms = tuple(
        term
        for block, reconstructed in zip(blocks, texts, strict=True)
        for term in block.make(reconstructed, languages)
    )
    return DecodedTerms(base, restart, canonical, terms)


def _value_record(typed: TypedValue) -> list[object]:
    kind, value = typed.kind, typed.value
    if type(kind) is not int:
        raise ValueError("typed value kind must be a registry code")
    if kind == 1 and type(value) is bool:
        return [kind, value]
    if kind in (2, 3) and type(value) is int:
        if (0 if kind == 2 else -1 - UINT_MAX) <= value <= UINT_MAX:
            return [kind, value]
    if kind == 4 and isinstance(value, TimeValue):
        if (
            type(value.seconds) is int
            and _OLDEST_SECOND <= value.seconds <= _LATEST_SECOND
            and type(value.nanoseconds) is int
            and 0 <= value.nanoseconds <= _MAX_NANOSECOND
        ):
            return [kind, value.seconds, value.nanoseconds]
    if kind == 5 and isinstance(value, bytes):
        return [kind, value]
    raise ValueError("typed value does not match its kind and range")


def _kind(term: WireTerm) -> int:
    match term:
        case IriTerm():
            return 1
        case LiteralTerm():
            if isinstance(term.lexical, bytes):
                return 2
            if isinstance(term.lexical, ValueReference):
                return 5
            raise ValueError("literal requires inline text bytes or a value reference")
        case BlankTerm():
            return 3
        case TripleTerm():
            return 4
        case TypedValueTerm():
            if isinstance(term.value, TypedValue):
                return 6
            if isinstance(term.value, ValueReference):
                return 7
            raise ValueError("literal requires a typed value or a value reference")
    raise TypeError("expected a v2 term")


def _encode_front(texts: Sequence[bytes], restart: int) -> list[object]:
    prefixes = []
    suffixes = []
    previous = b""
    for index, text in enumerate(texts):
        if not isinstance(text, bytes):
            raise ValueError("front-coded text must be bytes")
        text.decode("utf-8")
        prefix = 0
        if index % restart:
            for before, after in zip(previous, text, strict=False):
                if before != after:
                    break
                prefix += 1
        prefixes.append(prefix)
        suffixes.append(text[prefix:])
        previous = text
    return [prefixes, list(map(len, suffixes)), b"".join(suffixes)]


def _reference_fields(refs: Sequence[ValueReference]) -> list[object]:
    for ref in refs:
        if not isinstance(ref.digest, bytes) or len(ref.digest) != 32:
            raise ValueError("value reference requires a 32-byte digest")
        _uint(ref.length)
    return [[ref.digest for ref in refs], [ref.length for ref in refs]]


def _encode_forms(
    terms: Sequence[LiteralTerm], base: int, languages: dict[str, int]
) -> list[object]:
    forms = []
    datatypes = []
    indexes = []
    for term_id, term in enumerate(terms, base):
        if term.datatype is not None:
            _uint(term.datatype)
            if (
                term.language is not None
                or term.direction is not None
                or term.datatype >= term_id
            ):
                raise ValueError("a typed literal requires an earlier datatype ID")
            forms.append(1)
            datatypes.append(term.datatype)
        else:
            if not isinstance(term.language, str) or term.direction not in (
                None,
                "ltr",
                "rtl",
            ):
                raise ValueError(
                    "a language literal requires a language and an optional direction"
                )
            forms.append((None, "ltr", "rtl").index(term.direction) + 2)
            indexes.append(languages.setdefault(term.language, len(languages)))
    return [forms, datatypes, indexes]


def _encode_block(
    kind: int,
    terms: Sequence[WireTerm],
    base: int,
    restart: int,
    languages: dict[str, int],
) -> list[object]:
    if kind == 1:
        return [
            kind,
            *_encode_front([t.text for t in terms if isinstance(t, IriTerm)], restart),
        ]
    if kind in (2, 5):
        literals = [t for t in terms if isinstance(t, LiteralTerm)]
        lexicals = [t.lexical for t in literals]
        if kind == 2:
            encoded = _encode_front(
                [value for value in lexicals if isinstance(value, bytes)], restart
            )
        else:
            encoded = _reference_fields(
                [value for value in lexicals if isinstance(value, ValueReference)]
            )
        return [kind, *encoded, *_encode_forms(literals, base, languages)]
    if kind == 3:
        labels = [t.label for t in terms if isinstance(t, BlankTerm)]
        if any(not isinstance(label, bytes) for label in labels):
            raise ValueError("blank labels must be bytes")
        return [kind, labels]
    if kind == 4:
        triples = [t for t in terms if isinstance(t, TripleTerm)]
        for term_id, triple in enumerate(triples, base):
            for component in (triple.subject, triple.predicate, triple.object):
                _uint(component)
                if component >= term_id:
                    raise ValueError("triple components must refer to earlier IDs")
        return [
            kind,
            [t.subject for t in triples],
            [t.predicate for t in triples],
            [t.object for t in triples],
        ]
    values = [t.value for t in terms if isinstance(t, TypedValueTerm)]
    if kind == 6:
        return [
            kind,
            [_value_record(value) for value in values if isinstance(value, TypedValue)],
        ]
    return [
        kind,
        *_reference_fields(
            [value for value in values if isinstance(value, ValueReference)]
        ),
    ]


def encode_terms(
    terms: Sequence[WireTerm], *, id_base: int = 0, restart_group: int = 64
) -> tuple[list[object], bytes]:
    """Return a terms content record and payload in arrival order.

    Supply the earlier datatype and triple IDs from the same segment.
    """
    _uint(id_base)
    _uint(restart_group)
    if not terms or not restart_group or id_base + len(terms) > UINT_MAX + 1:
        raise ValueError("terms require a positive restart group and uint IDs")
    runs = []
    blocks = []
    languages: dict[str, int] = {}
    base = id_base
    for kind, group in groupby(terms, _kind):
        run = list(group)
        runs.append([kind, len(run)])
        blocks.append(_encode_block(kind, run, base, restart_group, languages))
        base += len(run)
    return [id_base, runs, 2, restart_group], cbor2.dumps([blocks, list(languages)])
