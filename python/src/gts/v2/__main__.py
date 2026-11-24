# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Command-line conversion to canonical GTS v2."""

import argparse
import sys
from os import link
from pathlib import Path
from tempfile import TemporaryDirectory

from . import ConversionStop, Converted, convert


def _publish(result: Converted, output: Path, proof: Path) -> None:
    with (
        TemporaryDirectory(dir=output.parent) as output_dir,
        TemporaryDirectory(dir=proof.parent) as proof_dir,
    ):
        staged_output = Path(output_dir) / "result"
        staged_proof = Path(proof_dir) / "proof"
        staged_output.write_bytes(result.file)
        staged_proof.write_bytes(result.proof.encode())
        link(staged_proof, proof)
        try:
            link(staged_output, output)
        except BaseException:
            if proof.exists() and proof.samefile(staged_proof):
                proof.unlink()
            raise


def main(argv: list[str] | None = None) -> int:
    """Run a v2 command."""
    parser = argparse.ArgumentParser(prog="python -m gts.v2")
    commands = parser.add_subparsers(dest="command", required=True)
    command = commands.add_parser("convert", help="convert to canonical GTS v2")
    command.add_argument("source", type=Path)
    command.add_argument(
        "-o", "--output", type=Path, required=True, help="new GTS file"
    )
    command.add_argument("--proof", type=Path, required=True, help="new proof file")
    args = parser.parse_args(argv)
    try:
        result = convert(args.source.read_bytes)
        if isinstance(result, ConversionStop):
            print(
                f"conversion_stopped: {result.reason} at {result.offset}",
                file=sys.stderr,
            )
            return 1
        _publish(result, args.output, args.proof)
    except OSError as error:
        print(f"gts v2 convert: {error}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
