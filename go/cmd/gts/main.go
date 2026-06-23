// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

// Command gts inspects, folds, verifies, and composes GTS files.
//
// Exit codes: 0 clean; 1 diagnostics found or input refused; 2 usage/IO error.
package main

import (
	"fmt"
	"os"
)

const usage = `usage: gts <command> [args]

commands:
  info <file>...            per-segment composition ledger
  fold <file>               fold to N-Quads on stdout
  verify <file>...          verify chains; ledger + diagnostics; exit 1 on any
  verify-proof <proof.json>  verify detached MMR proof JSON without the GTS file
  heads <file>              JSON segment heads and aggregate comparison digest
  segments <file>           JSON segment byte ranges and layout inventory
  missing --from-head <head> <file>
                            JSON byte ranges needed after a peer head
  resume --after <frame-id> <file>
                            emit bytes after a verified frame boundary
  extract-key <file>        print the embedded transport key: kid, OpenPGP
                            fingerprint, emojihash, and armored public key
  ls <file>                 list inline blobs: digest, size, declared media type
  extract <file> <digest> [-o out] [--mt TYPE] [--include-suppressed]
                            extract one blob by content digest
  cat -o <out> <file>...    validating composer: refuse degenerate inputs,
                            then byte-concatenate
  compact <file> -o <out> --streamable [--seal-original] [--timestamp ISO]
                            rewrite into the streamable layout state: leading
                            streaming index, blobs most-significant-first,
                            trailing index footer
  pack <dir|file>... -o out.gts
                            pack files/directories into a files-profile archive
  unpack <archive> [-C dir] [--include-suppressed]
                            unpack a files-profile archive
  diff <archive> <dir>      compare archive to directory by digest
  from-nq <in.nq> [-o out]  build a GTS from N-Quads; '-' reads stdin`

func main() {
	args := os.Args[1:]
	if len(args) == 0 {
		fmt.Fprintln(os.Stderr, usage)
		os.Exit(2)
	}
	cmd := args[0]
	switch cmd {
	case "info":
		os.Exit(cmdInfo(args[1:]))
	case "fold":
		os.Exit(cmdFold(args[1:]))
	case "verify":
		os.Exit(cmdVerify(args[1:]))
	case "verify-proof":
		os.Exit(cmdVerifyProof(args[1:]))
	case "heads":
		os.Exit(cmdHeads(args[1:]))
	case "segments":
		os.Exit(cmdSegments(args[1:]))
	case "missing":
		os.Exit(cmdMissing(args[1:]))
	case "resume":
		os.Exit(cmdResume(args[1:]))
	case "extract-key":
		os.Exit(cmdExtractKey(args[1:]))
	case "ls":
		os.Exit(cmdLs(args[1:]))
	case "extract":
		os.Exit(cmdExtract(args[1:]))
	case "cat":
		os.Exit(cmdCat(args[1:]))
	case "compact":
		os.Exit(cmdCompact(args[1:]))
	case "pack":
		os.Exit(cmdPack(args[1:]))
	case "unpack":
		os.Exit(cmdUnpack(args[1:]))
	case "diff":
		os.Exit(cmdDiff(args[1:]))
	case "from-nq":
		os.Exit(cmdFromNQ(args[1:]))
	case "-h", "--help", "help":
		fmt.Println(usage)
		os.Exit(0)
	default:
		fmt.Fprintf(os.Stderr, "gts: unknown command '%s'\n%s\n", cmd, usage)
		os.Exit(2)
	}
}
