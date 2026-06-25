# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""The ``gts`` command-line tool: inspect, fold, verify, and compose GTS files.

``cat`` and ``verify`` implement the §14.1 composition-tooling contract: raw
byte concatenation is always valid GTS (§3.1), but a publish-class tool
refuses pathological states instead of trusting them to be intentional. The
Rust engine ships a binary with the IDENTICAL command surface; this entry
point keeps the contract while the native wheel lands.

Exit codes: 0 clean; 1 diagnostics found or input refused; 2 usage/IO error.
"""

from __future__ import annotations

import argparse
import os
import sys
from collections.abc import Mapping

from gts.cli_common import _load, _print_ledger, _write_out
from gts.cli_export import (
    _cmd_from_nq,
    _cmd_from_trig,
    _cmd_to_duckdb,
    _cmd_to_parquet,
    _cmd_to_sqlite,
    _cmd_to_trig,
)
from gts.cli_files import (
    _cmd_cat,
    _cmd_compact,
    _cmd_diff,
    _cmd_extract,
    _cmd_ls,
    _cmd_pack,
    _cmd_unpack,
)
from gts.cli_verify import _cmd_extract_key, _cmd_verify, _cmd_verify_proof
from gts.mmr import parse_hex_32
from gts.nquads import to_nquads
from gts.reader import read, read_segments
from gts.replication import (
    heads_json,
    inventory,
    missing,
    missing_json,
    resume_after,
    segments_json,
)

_CLI_COMMANDS = {
    "cat",
    "compact",
    "diff",
    "extract",
    "extract-key",
    "fold",
    "from-nq",
    "from-trig",
    "heads",
    "info",
    "ls",
    "missing",
    "pack",
    "resume",
    "segments",
    "to-duckdb",
    "to-parquet",
    "to-sqlite",
    "to-trig",
    "unpack",
    "verify",
    "verify-proof",
}

_USAGE_EN = """usage: gts <command> [args]

commands:
  info <file>...            per-segment composition ledger
  fold <file>               fold to N-Quads on stdout
  verify <file>...          verify chains; ledger + diagnostics; exit 1 on any
  verify-proof <proof.json> verify detached MMR proof JSON without the GTS file
  heads <file>              JSON segment heads and aggregate comparison digest
  segments <file>           JSON segment byte ranges and layout inventory
  missing --from-head <head> <file>
                            JSON byte ranges needed after a peer head
  resume --after <frame-id> <file>
                            emit bytes after a verified frame boundary
  extract-key <file>        print the embedded transport/verification key
  ls <file>                 list inline blobs: digest, size, declared media type
  extract <file> <digest> [-o out] [--mt TYPE] [--include-suppressed]
                            extract one blob by content digest
  cat -o <out> <file>...    validating composer: refuse degenerate inputs,
                            then byte-concatenate
  compact <file> -o <out> --streamable [--seal-original] [--timestamp ISO]
                            rewrite into the streamable layout state
  pack <dir|file>... -o out.gts
                            pack files/directories into a files-profile archive
  unpack <archive> [-C dir] [--include-suppressed]
                            unpack a files-profile archive
  diff <archive> <dir>      compare archive to directory by digest
  from-nq <in.nq> [-o out]  build a GTS from N-Quads; '-' reads stdin
  to-trig <file>            fold to TriG on stdout
  from-trig <in.trig> [-o out]
                            build a GTS from TriG; '-' reads stdin
  to-sqlite <file> <out>    export a folded graph to SQLite
  to-duckdb <file> <out>    export a folded graph to DuckDB
  to-parquet <file> <out_dir>
                            export a folded graph to Parquet"""

_USAGE_FR_CA = """utilisation: gts <command> [args]

commandes:
  info <file>...            affiche le registre de composition par segment
  fold <file>               plie vers N-Quads sur stdout
  verify <file>...          verifie les chaines, le registre et les diagnostics
  verify-proof <proof.json> verifie une preuve MMR detachee sans fichier GTS
  heads <file>              emet les tetes de segments en JSON
  segments <file>           emet les plages d'octets des segments en JSON
  missing --from-head <head> <file>
                            emet les plages JSON requises apres une tete de pair
  resume --after <frame-id> <file>
                            emet les octets apres une frontiere de trame valide
  extract-key <file>        imprime la cle de transport/verification integree
  ls <file>                 liste les blobs: digest, taille, type media declare
  extract <file> <digest> [-o out] [--mt TYPE] [--include-suppressed]
                            extrait un blob par digest de contenu
  cat -o <out> <file>...    compose en validant et refuse les entrees degenerees
  compact <file> -o <out> --streamable [--seal-original] [--timestamp ISO]
                            reecrit vers l'etat de disposition diffusable
  pack <dir|file>... -o out.gts
                            emballe des fichiers en archive de profil files
  unpack <archive> [-C dir] [--include-suppressed]
                            deballe une archive de profil files
  diff <archive> <dir>      compare une archive a un repertoire par digest
  from-nq <in.nq> [-o out]  construit un GTS depuis N-Quads; '-' lit stdin
  to-trig <file>            plie vers TriG sur stdout
  from-trig <in.trig> [-o out]
                            construit un GTS depuis TriG; '-' lit stdin
  to-sqlite <file> <out>    exporte le graphe plie vers SQLite
  to-duckdb <file> <out>    exporte le graphe plie vers DuckDB
  to-parquet <file> <out_dir>
                            exporte le graphe plie vers Parquet"""

_USAGE_ZH_HANS = """用法: gts <command> [args]

命令:
  info <file>...            显示每个段的组合账本
  fold <file>               将内容折叠为 N-Quads 并写到 stdout
  verify <file>...          验证链、账本和诊断；发现问题时退出 1
  verify-proof <proof.json> 在没有 GTS 文件时验证分离的 MMR 证明
  heads <file>              输出段头和聚合摘要的 JSON
  segments <file>           输出段字节范围和布局清单的 JSON
  missing --from-head <head> <file>
                            输出对等段头之后所需的字节范围 JSON
  resume --after <frame-id> <file>
                            输出已验证帧边界之后的字节
  extract-key <file>        打印内嵌的传输/验证密钥
  ls <file>                 列出内联 blob 的摘要、大小和声明媒体类型
  extract <file> <digest> [-o out] [--mt TYPE] [--include-suppressed]
                            按内容摘要提取一个 blob
  cat -o <out> <file>...    验证后组合，拒绝退化输入
  compact <file> -o <out> --streamable [--seal-original] [--timestamp ISO]
                            重写为可流式布局状态
  pack <dir|file>... -o out.gts
                            将文件或目录打包为 files 配置文件归档
  unpack <archive> [-C dir] [--include-suppressed]
                            解包 files 配置文件归档
  diff <archive> <dir>      按摘要比较归档和目录
  from-nq <in.nq> [-o out]  从 N-Quads 构建 GTS；'-' 读取 stdin
  to-trig <file>            将内容折叠为 TriG 并写到 stdout
  from-trig <in.trig> [-o out]
                            从 TriG 构建 GTS；'-' 读取 stdin
  to-sqlite <file> <out>    将折叠图导出到 SQLite
  to-duckdb <file> <out>    将折叠图导出到 DuckDB
  to-parquet <file> <out_dir>
                            将折叠图导出到 Parquet"""

_USAGE_BY_LOCALE = {
    "en": _USAGE_EN,
    "fr-CA": _USAGE_FR_CA,
    "zh-Hans": _USAGE_ZH_HANS,
}


def _locale_from(raw: str) -> str:
    value = raw.strip().split(".", 1)[0].split("@", 1)[0].replace("_", "-").lower()
    if value in {"fr", "fr-ca"}:
        return "fr-CA"
    if value in {"zh", "zh-cn", "zh-hans", "zh-hans-cn"}:
        return "zh-Hans"
    return "en"


def _cli_locale(env: Mapping[str, str] | None = None) -> str:
    source = os.environ if env is None else env
    for key in ("GTS_LANG", "LC_ALL", "LC_MESSAGES", "LANG"):
        raw = source.get(key, "")
        if raw.strip():
            return _locale_from(raw)
    return "en"


def _usage_text(locale: str | None = None) -> str:
    return _USAGE_BY_LOCALE.get(locale or _cli_locale(), _USAGE_EN)


def _unknown_command(command: str, locale: str) -> str:
    if locale == "fr-CA":
        return f"gts: commande inconnue '{command}'\n{_usage_text(locale)}"
    if locale == "zh-Hans":
        return f"gts: 未知命令 '{command}'\n{_usage_text(locale)}"
    return f"gts: unknown command '{command}'\n{_usage_text(locale)}"


def _cmd_info(paths: list[str]) -> int:
    for path in paths:
        segments, torn, fatal = read_segments(_load(path))
        if fatal is not None:
            print(f"{path}: 0 segment(s)")
            print(f"  FATAL {fatal.code}: {fatal.detail}")
            continue
        _print_ledger(path, segments, torn)
    return 0


def _cmd_fold(path: str) -> int:
    g = read(_load(path))
    for d in g.diagnostics:
        print(f"gts: diagnostic {d.code}: {d.detail}", file=sys.stderr)
    sys.stdout.write(to_nquads(g))
    # The (possibly partial) fold is still emitted, but any diagnostic —
    # or never reaching segmentation at all — is a nonzero exit, so
    # `gts fold … && publish` pipelines fail on damage.
    return 1 if g.diagnostics or not g.segment_heads else 0


def _cmd_heads(path: str) -> int:
    inv = inventory(_load(path))
    print(heads_json(inv), end="")
    return 1 if inv.has_problems() else 0


def _cmd_segments(path: str) -> int:
    inv = inventory(_load(path))
    print(segments_json(inv), end="")
    return 1 if inv.has_problems() else 0


def _cmd_missing(from_head: str, path: str) -> int:
    try:
        peer_head = parse_hex_32(from_head)
    except ValueError as exc:
        print(f"gts missing: invalid peer head: {exc}", file=sys.stderr)
        return 2
    result = missing(inventory(_load(path)), peer_head)
    print(missing_json(result), end="")
    return 1 if result.status == "error" else 0


def _cmd_resume(after: str, path: str) -> int:
    try:
        frame_id = parse_hex_32(after)
    except ValueError as exc:
        print(f"gts resume: invalid frame id: {exc}", file=sys.stderr)
        return 2
    data = _load(path)
    try:
        tail = resume_after(data, frame_id)
    except ValueError as exc:
        print(f"gts resume: {exc}", file=sys.stderr)
        return 1
    return _write_out(None, tail)


def main(argv: list[str] | None = None) -> int:
    """Entry point for the ``gts`` console script."""
    raw_args = list(sys.argv[1:] if argv is None else argv)
    locale = _cli_locale()
    if not raw_args:
        print(_usage_text(locale), file=sys.stderr)
        return 2
    if raw_args[0] in {"-h", "--help", "help"}:
        print(_usage_text(locale))
        return 0
    if raw_args[0] not in _CLI_COMMANDS:
        print(_unknown_command(raw_args[0], locale), file=sys.stderr)
        return 2

    parser = argparse.ArgumentParser(
        prog="gts",
        description="Inspect, fold, verify, and compose GTS files.",
    )
    sub = parser.add_subparsers(dest="command", required=True)

    p_info = sub.add_parser("info", help="per-segment composition ledger (§14.1)")
    p_info.add_argument("files", nargs="+")

    p_fold = sub.add_parser("fold", help="fold to N-Quads on stdout")
    p_fold.add_argument("file")

    p_to_trig = sub.add_parser("to-trig", help="fold to TriG on stdout")
    p_to_trig.add_argument("file")

    p_verify = sub.add_parser(
        "verify", help="verify chains; ledger + diagnostics; exit 1 on any"
    )
    p_verify.add_argument("files", nargs="+")
    p_verify.add_argument(
        "--key",
        action="append",
        metavar="KID:HEXPUB",
        help="verify COSE signatures against a raw Ed25519 public key (repeatable)",
    )
    p_verify.add_argument(
        "--trusted-signer",
        action="append",
        metavar="KID",
        help="profile-policy trust anchor for an already verified signer kid",
    )

    p_verify_proof = sub.add_parser(
        "verify-proof",
        help="verify detached MMR proof JSON without the GTS file",
    )
    p_verify_proof.add_argument("proof")

    p_heads = sub.add_parser(
        "heads", help="JSON segment heads and aggregate comparison digest"
    )
    p_heads.add_argument("file")

    p_segments = sub.add_parser(
        "segments", help="JSON segment byte ranges and layout inventory"
    )
    p_segments.add_argument("file")

    p_missing = sub.add_parser(
        "missing", help="JSON byte ranges needed after a peer head"
    )
    p_missing.add_argument("--from-head", required=True)
    p_missing.add_argument("file")

    p_resume = sub.add_parser(
        "resume", help="emit bytes after a verified frame boundary"
    )
    p_resume.add_argument("--after", required=True)
    p_resume.add_argument("file")

    p_ls = sub.add_parser(
        "ls", help="list inline blobs: digest, size, declared media type"
    )
    p_ls.add_argument("file")

    p_extract_key = sub.add_parser(
        "extract-key",
        help="print the embedded transport/verification key: kid, fingerprint, "
        "emojihash, armored public key (§9.2)",
    )
    p_extract_key.add_argument("file")

    p_from_nq = sub.add_parser(
        "from-nq",
        help="build a GTS from N-Quads — the inverse of fold; '-' reads stdin",
    )
    p_from_nq.add_argument("file")
    p_from_nq.add_argument("-o", "--out", default=None)

    p_from_trig = sub.add_parser(
        "from-trig",
        help="build a GTS from TriG — the inverse of to-trig; '-' reads stdin",
    )
    p_from_trig.add_argument("file")
    p_from_trig.add_argument("-o", "--out", default=None)

    p_to_sqlite = sub.add_parser(
        "to-sqlite", help="export a folded graph to a SQLite database (§14)"
    )
    p_to_sqlite.add_argument("file")
    p_to_sqlite.add_argument("out")

    p_to_duckdb = sub.add_parser(
        "to-duckdb",
        help="export a folded graph to a DuckDB database (needs the [db] extra)",
    )
    p_to_duckdb.add_argument("file")
    p_to_duckdb.add_argument("out")

    p_to_parquet = sub.add_parser(
        "to-parquet",
        help="export a folded graph to Parquet, one file per table "
        "(needs the [db] extra)",
    )
    p_to_parquet.add_argument("file")
    p_to_parquet.add_argument("out_dir")

    p_extract = sub.add_parser(
        "extract",
        help="extract one blob by content digest; --mt asserts the declared "
        "media type (never converts)",
    )
    p_extract.add_argument("file")
    p_extract.add_argument("digest")
    p_extract.add_argument("-o", "--out", default=None)
    p_extract.add_argument("--mt", default=None)
    p_extract.add_argument("--include-suppressed", action="store_true")

    p_cat = sub.add_parser(
        "cat",
        help="validating composer: refuse degenerate inputs, then "
        "byte-concatenate (§3.1, §14.1)",
    )
    p_cat.add_argument("files", nargs="+")
    p_cat.add_argument("-o", "--out", default=None)

    p_compact = sub.add_parser(
        "compact",
        help="rewrite into the streamable layout state: leading streaming "
        "index, blobs most-significant-first, trailing index footer (§10.1)",
    )
    p_compact.add_argument("file")
    p_compact.add_argument("-o", "--out", required=True)
    p_compact.add_argument(
        "--streamable",
        action="store_true",
        help="produce the delivery-ordered streamable layout (§3.3)",
    )
    p_compact.add_argument(
        "--seal-original",
        action="store_true",
        help="carry the verbatim source as a nested GTS blob, role 'source' "
        "(REQUIRED for evidence input)",
    )
    p_compact.add_argument(
        "--timestamp",
        default=None,
        help="rewrite time recorded as stream:timestamp (ISO 8601 UTC); "
        "defaults to now — pass a fixed value for reproducible output",
    )

    p_pack = sub.add_parser(
        "pack", help="pack files/directories into a files-profile GTS archive"
    )
    p_pack.add_argument("sources", nargs="+")
    p_pack.add_argument("-o", "--out", required=True)

    p_unpack = sub.add_parser("unpack", help="unpack a files-profile GTS archive")
    p_unpack.add_argument("file")
    p_unpack.add_argument("-C", dest="dest", default=None)
    p_unpack.add_argument(
        "--include-suppressed",
        action="store_true",
        help="extract digest-suppressed entries anyway",
    )

    p_diff = sub.add_parser(
        "diff",
        help="compare a files-profile GTS archive to a directory by digest",
    )
    p_diff.add_argument("archive")
    p_diff.add_argument("directory")

    args = parser.parse_args(raw_args)
    if args.command == "info":
        return _cmd_info(args.files)
    if args.command == "fold":
        return _cmd_fold(args.file)
    if args.command == "to-trig":
        return _cmd_to_trig(args.file)
    if args.command == "verify":
        return _cmd_verify(args.files, args.key, args.trusted_signer)
    if args.command == "verify-proof":
        return _cmd_verify_proof(args.proof)
    if args.command == "heads":
        return _cmd_heads(args.file)
    if args.command == "segments":
        return _cmd_segments(args.file)
    if args.command == "missing":
        return _cmd_missing(args.from_head, args.file)
    if args.command == "resume":
        return _cmd_resume(args.after, args.file)
    if args.command == "ls":
        return _cmd_ls(args.file)
    if args.command == "extract-key":
        return _cmd_extract_key(args.file)
    if args.command == "from-nq":
        return _cmd_from_nq(args.file, args.out)
    if args.command == "from-trig":
        return _cmd_from_trig(args.file, args.out)
    if args.command == "to-sqlite":
        return _cmd_to_sqlite(args.file, args.out)
    if args.command == "to-duckdb":
        return _cmd_to_duckdb(args.file, args.out)
    if args.command == "to-parquet":
        return _cmd_to_parquet(args.file, args.out_dir)
    if args.command == "extract":
        return _cmd_extract(
            args.file, args.digest, args.out, args.mt, args.include_suppressed
        )
    if args.command == "compact":
        return _cmd_compact(
            args.file,
            args.out,
            streamable=args.streamable,
            seal_original=args.seal_original,
            timestamp=args.timestamp,
        )
    if args.command == "pack":
        return _cmd_pack(args.sources, args.out)
    if args.command == "unpack":
        return _cmd_unpack(args.file, args.dest, args.include_suppressed)
    if args.command == "diff":
        return _cmd_diff(args.archive, args.directory)
    return _cmd_cat(args.files, args.out)


if __name__ == "__main__":  # pragma: no cover
    raise SystemExit(main())
