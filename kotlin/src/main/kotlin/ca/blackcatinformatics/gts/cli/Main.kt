// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

package ca.blackcatinformatics.gts.cli

import ca.blackcatinformatics.gts.foldSummaryJson
import ca.blackcatinformatics.gts.fromNQuads
import ca.blackcatinformatics.gts.hex
import ca.blackcatinformatics.gts.read
import ca.blackcatinformatics.gts.readFileSegments
import ca.blackcatinformatics.gts.toNQuads
import java.nio.file.Files
import java.nio.file.Path
import kotlin.system.exitProcess

private const val USAGE = """usage: gts <command> [args]

commands:
  info <file>...
  fold <file>
  verify <file>...
  verify-proof <proof.json>
  heads <file>
  segments <file>
  missing --from-head <head> <file>
  resume --after <frame-id> <file>
  extract-key <file>
  ls <file>
  extract <file> <digest> [-o out] [--mt TYPE] [--include-suppressed]
  cat -o <out> <file>...
  compact <file> -o <out> --streamable [--seal-original] [--timestamp ISO]
  pack <dir|file>... -o out.gts
  unpack <archive> [-C dir] [--include-suppressed]
  diff <archive> <dir>
  from-nq <in.nq> [-o out]"""

fun main(args: Array<String>) {
    if (args.isEmpty()) dieUsage()
    val code =
        when (args[0]) {
            "info" -> cmdInfo(args.drop(1))
            "fold" -> cmdFold(args.drop(1))
            "verify" -> cmdVerify(args.drop(1))
            "verify-proof",
            "heads",
            "segments",
            "missing",
            "resume",
            "extract-key",
            "ls",
            "extract",
            "cat",
            "compact",
            "pack",
            "unpack",
            "diff",
            -> notImplemented(args[0])
            "from-nq" -> cmdFromNq(args.drop(1))
            "-h", "--help", "help" -> {
                println(USAGE)
                0
            }
            else -> {
                System.err.println("gts: unknown command '${args[0]}'\n$USAGE")
                2
            }
        }
    exitProcess(code)
}

private fun cmdInfo(paths: List<String>): Int {
    if (paths.isEmpty()) return dieUsage()
    var problems = false
    for (path in paths) {
        val data = load(path) ?: return 2
        val fs = readFileSegments(data)
        val tornSuffix = if (fs.torn >= 0) ", TORN at byte ${fs.torn}" else ""
        println("$path: ${fs.segments.size} segment(s)$tornSuffix")
        fs.fatal?.let {
            println("  FATAL ${it.code}: ${it.detail}")
            problems = true
            return@let
        }
        for ((idx, graph) in fs.segments.withIndex()) {
            val head = graph.segmentHeads.firstOrNull()?.let { hex(it) } ?: "<none>"
            val profile = graph.segmentProfiles.firstOrNull() ?: "<none>"
            println(
                "  segment $idx: head $head profile $profile terms ${graph.terms.size} quads ${graph.quads.size} " +
                    "reifies ${graph.reifiers.size} annot ${graph.annotations.size} blobs ${graph.blobs.size} " +
                    "suppress ${graph.suppressions.size} opaque ${graph.opaque.size} sigs ${graph.signatures.size}",
            )
            graph.diagnostics.forEach { println("    diagnostic ${it.code}: ${it.detail}") }
            if (graph.diagnostics.isNotEmpty()) problems = true
        }
    }
    return if (problems) 1 else 0
}

private fun cmdFold(paths: List<String>): Int {
    if (paths.size != 1) return dieUsage()
    val data = load(paths[0]) ?: return 2
    val graph = read(data, true)
    graph.diagnostics.forEach { System.err.println("gts: diagnostic ${it.code}: ${it.detail}") }
    print(toNQuads(graph))
    return if (graph.diagnostics.isEmpty() && graph.segmentHeads.isNotEmpty()) 0 else 1
}

private fun cmdVerify(paths: List<String>): Int {
    if (paths.isEmpty()) return dieUsage()
    var problems = false
    for (path in paths) {
        val data = load(path) ?: return 2
        val graph = read(data, true)
        println(foldSummaryJson(graph))
        if (graph.diagnostics.isNotEmpty() || graph.segmentHeads.isEmpty()) problems = true
    }
    return if (problems) 1 else 0
}

private fun cmdFromNq(args: List<String>): Int {
    var outPath: String? = null
    val positional = mutableListOf<String>()
    var i = 0
    while (i < args.size) {
        when (args[i]) {
            "-o", "--out" -> {
                if (i + 1 >= args.size) {
                    System.err.println("gts from-nq: -o requires a path\n$USAGE")
                    return 2
                }
                outPath = args[i + 1]
                i += 2
            }
            else -> {
                positional += args[i]
                i++
            }
        }
    }
    if (positional.size != 1) return dieUsage()
    val text =
        try {
            if (positional[0] == "-") {
                generateSequence(::readLine).joinToString("\n")
            } else {
                Files.readString(Path.of(positional[0]))
            }
        } catch (err: Exception) {
            System.err.println("gts from-nq: cannot read ${positional[0]}: ${err.message}")
            return 2
        }
    val bytes =
        try {
            fromNQuads(text)
        } catch (err: Exception) {
            System.err.println("gts from-nq: ${err.message}")
            return 1
        }
    return try {
        if (outPath == null) {
            System.out.write(bytes)
        } else {
            Files.write(Path.of(outPath), bytes)
        }
        0
    } catch (err: Exception) {
        System.err.println("gts from-nq: cannot write output: ${err.message}")
        2
    }
}

private fun notImplemented(command: String): Int {
    System.err.println("gts: command '$command' is not implemented yet in this stage-1 branch")
    return 2
}

private fun load(path: String): ByteArray? =
    try {
        Files.readAllBytes(Path.of(path))
    } catch (err: Exception) {
        System.err.println("gts: cannot read $path: ${err.message}")
        null
    }

private fun dieUsage(): Int {
    System.err.println(USAGE)
    return 2
}
