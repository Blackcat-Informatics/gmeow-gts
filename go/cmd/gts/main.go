// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

// Command gts inspects, folds, verifies, and composes GTS files.
//
// Exit codes: 0 clean; 1 diagnostics found or input refused; 2 usage/IO error.
package main

import (
	"fmt"
	"os"
	"strings"
)

type cliLocale int

const (
	localeEnglish cliLocale = iota
	localeFrenchCanada
	localeChineseHans
)

var usage = usageText(resolveLocale())

const usageEN = `usage: gts <command> [args]

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
  from-nq <in.nq> [-o out]  build a GTS from N-Quads; '-' reads stdin
  to-nt <file>              fold the default graph to N-Triples on stdout
  from-nt <in.nt> [-o out]  build a GTS from N-Triples; '-' reads stdin
  to-trig <file>            fold to TriG on stdout
  from-trig <in.trig> [-o out] build a GTS from TriG; '-' reads stdin
  to-turtle <file>          fold the default graph to Turtle on stdout
  from-turtle <in.ttl> [-o out] build a GTS from Turtle; '-' reads stdin
  to-rdfxml <file>          fold the default graph to RDF/XML on stdout
  from-rdfxml <in.rdf> [-o out] build a GTS from RDF/XML; '-' reads stdin`

const usageFRCA = `utilisation: gts <command> [args]

commandes:
  info <file>...            affiche le registre de composition par segment
  fold <file>               plie vers N-Quads sur stdout
  verify <file>...          verifie les chaines, le registre et les diagnostics
  verify-proof <proof.json>  verifie une preuve MMR detachee sans fichier GTS
  heads <file>              emet les tetes de segments et le digest agrege en JSON
  segments <file>           emet les plages d'octets et l'inventaire en JSON
  missing --from-head <head> <file>
emet les plages JSON requises apres une tete de pair
  resume --after <frame-id> <file>
emet les octets apres une frontiere de trame valide
  extract-key <file>        imprime la cle de transport: kid, OpenPGP,
fingerprint, emojihash et cle publique blindee
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
  to-nt <file>              plie le graphe par defaut vers N-Triples
  from-nt <in.nt> [-o out]  construit un GTS depuis N-Triples; '-' lit stdin
  to-trig <file>            plie vers TriG sur stdout
  from-trig <in.trig> [-o out] construit un GTS depuis TriG; '-' lit stdin
  to-turtle <file>          plie le graphe par defaut vers Turtle
  from-turtle <in.ttl> [-o out] construit un GTS depuis Turtle; '-' lit stdin
  to-rdfxml <file>          plie le graphe par defaut vers RDF/XML
  from-rdfxml <in.rdf> [-o out] construit un GTS depuis RDF/XML; '-' lit stdin`

const usageZHHans = `用法: gts <command> [args]

命令:
  info <file>...            显示每个段的组合账本
  fold <file>               将内容折叠为 N-Quads 并写到 stdout
  verify <file>...          验证链、账本和诊断；发现问题时退出 1
  verify-proof <proof.json>  在没有 GTS 文件时验证分离的 MMR 证明
  heads <file>              输出段头和聚合比较摘要的 JSON
  segments <file>           输出段字节范围和布局清单的 JSON
  missing --from-head <head> <file>
输出对等段头之后所需的字节范围 JSON
  resume --after <frame-id> <file>
输出已验证帧边界之后的字节
  extract-key <file>        打印内嵌传输密钥、fingerprint、emojihash 和公钥
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
  to-nt <file>              将默认图折叠为 N-Triples
  from-nt <in.nt> [-o out]  从 N-Triples 构建 GTS；'-' 读取 stdin
  to-trig <file>            将内容折叠为 TriG 并写到 stdout
  from-trig <in.trig> [-o out] 从 TriG 构建 GTS；'-' 读取 stdin
  to-turtle <file>          将默认图折叠为 Turtle
  from-turtle <in.ttl> [-o out] 从 Turtle 构建 GTS；'-' 读取 stdin
  to-rdfxml <file>          将默认图折叠为 RDF/XML
  from-rdfxml <in.rdf> [-o out] 从 RDF/XML 构建 GTS；'-' 读取 stdin`

func resolveLocale() cliLocale {
	for _, key := range []string{"GTS_LANG", "LC_ALL", "LC_MESSAGES", "LANG"} {
		raw := strings.TrimSpace(os.Getenv(key))
		if raw != "" {
			return localeFrom(raw)
		}
	}
	return localeEnglish
}

func localeFrom(raw string) cliLocale {
	value := strings.ToLower(strings.ReplaceAll(raw, "_", "-"))
	if idx := strings.IndexAny(value, ".@"); idx >= 0 {
		value = value[:idx]
	}
	switch value {
	case "fr", "fr-ca":
		return localeFrenchCanada
	case "zh", "zh-cn", "zh-hans", "zh-hans-cn":
		return localeChineseHans
	default:
		return localeEnglish
	}
}

func usageText(locale cliLocale) string {
	switch locale {
	case localeFrenchCanada:
		return usageFRCA
	case localeChineseHans:
		return usageZHHans
	default:
		return usageEN
	}
}

func unknownCommandMessage(command string) string {
	switch resolveLocale() {
	case localeFrenchCanada:
		return fmt.Sprintf("gts: commande inconnue '%s'\n%s", command, usage)
	case localeChineseHans:
		return fmt.Sprintf("gts: 未知命令 '%s'\n%s", command, usage)
	default:
		return fmt.Sprintf("gts: unknown command '%s'\n%s", command, usage)
	}
}

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
	case "to-nt":
		os.Exit(cmdToRDFText(cmd, args[1:]))
	case "to-trig":
		os.Exit(cmdToRDFText(cmd, args[1:]))
	case "to-turtle":
		os.Exit(cmdToRDFText(cmd, args[1:]))
	case "to-rdfxml":
		os.Exit(cmdToRDFText(cmd, args[1:]))
	case "from-nt":
		os.Exit(cmdFromRDFText(cmd, args[1:]))
	case "from-trig":
		os.Exit(cmdFromRDFText(cmd, args[1:]))
	case "from-turtle":
		os.Exit(cmdFromRDFText(cmd, args[1:]))
	case "from-rdfxml":
		os.Exit(cmdFromRDFText(cmd, args[1:]))
	case "-h", "--help", "help":
		fmt.Println(usage)
		os.Exit(0)
	default:
		fmt.Fprintln(os.Stderr, unknownCommandMessage(cmd))
		os.Exit(2)
	}
}
