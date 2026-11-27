# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Every recorded P6 read result is rebuilt byte for byte.

The rebuild is `scripts/gen_p6_read_results.py`. The test rebuilds each read result from
its input file with the Python reader and compares it with the recorded bytes. It writes
nothing: a mismatch fails the case that differs.
"""

from __future__ import annotations

import importlib.util
from pathlib import Path

import pytest

_SCRIPT = Path(__file__).resolve().parents[1] / "scripts" / "gen_p6_read_results.py"
_spec = importlib.util.spec_from_file_location("gen_p6_read_results", _SCRIPT)
assert _spec is not None and _spec.loader is not None
gen = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(gen)

_CASES = [
    (f"{path.name}:{index}", case)
    for path, _document, index, case in gen._cases(gen.P6_DIR)
]


@pytest.mark.parametrize(
    "case", [case for _, case in _CASES], ids=[name for name, _ in _CASES]
)
def test_recorded_p6_read_result_is_rebuilt_byte_for_byte(case: dict) -> None:
    assert gen.rebuild(gen.P6_DIR, case) == case["read_result"]
