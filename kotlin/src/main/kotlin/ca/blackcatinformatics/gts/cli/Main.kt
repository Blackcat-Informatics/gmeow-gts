// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

package ca.blackcatinformatics.gts.cli

import ca.blackcatinformatics.gts.CompactRefusedException
import ca.blackcatinformatics.gts.CborArray
import ca.blackcatinformatics.gts.CborMap
import ca.blackcatinformatics.gts.Graph
import ca.blackcatinformatics.gts.Quad
import ca.blackcatinformatics.gts.asInt
import ca.blackcatinformatics.gts.asText
import ca.blackcatinformatics.gts.diff
import ca.blackcatinformatics.gts.digestStr
import ca.blackcatinformatics.gts.emojihash
import ca.blackcatinformatics.gts.foldSummaryJson
import ca.blackcatinformatics.gts.formatFingerprint
import ca.blackcatinformatics.gts.fromNQuads
import ca.blackcatinformatics.gts.getTextKey
import ca.blackcatinformatics.gts.headsJson
import ca.blackcatinformatics.gts.hex
import ca.blackcatinformatics.gts.inventoryFor
import ca.blackcatinformatics.gts.missing
import ca.blackcatinformatics.gts.missingJson
import ca.blackcatinformatics.gts.normalizeDigest
import ca.blackcatinformatics.gts.pack
import ca.blackcatinformatics.gts.parseHex
import ca.blackcatinformatics.gts.parseHex32
import ca.blackcatinformatics.gts.parseTransportKey
import ca.blackcatinformatics.gts.proofFromJson
import ca.blackcatinformatics.gts.read
import ca.blackcatinformatics.gts.readFileSegments
import ca.blackcatinformatics.gts.resumeAfter
import ca.blackcatinformatics.gts.segmentsJson
import ca.blackcatinformatics.gts.streamableCompact
import ca.blackcatinformatics.gts.toNQuads
import ca.blackcatinformatics.gts.unpack
import ca.blackcatinformatics.gts.verifySignatures
import ca.blackcatinformatics.gts.verifyProof
import java.nio.file.Files
import java.nio.file.Path
import java.time.Instant
import java.time.format.DateTimeFormatter
import kotlin.system.exitProcess

private const val USAGE = """usage: gts <command> [args]

commands:
  info <file>...
  fold <file>
  verify <file>... [--key KID:HEXPUB]
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
            "verify-proof" -> cmdVerifyProof(args.drop(1))
            "heads" -> cmdHeads(args.drop(1))
            "segments" -> cmdSegments(args.drop(1))
            "missing" -> cmdMissing(args.drop(1))
            "resume" -> cmdResume(args.drop(1))
            "extract-key" -> cmdExtractKey(args.drop(1))
            "ls" -> cmdLs(args.drop(1))
            "extract" -> cmdExtract(args.drop(1))
            "cat" -> cmdCat(args.drop(1))
            "compact" -> cmdCompact(args.drop(1))
            "pack" -> cmdPack(args.drop(1))
            "unpack" -> cmdUnpack(args.drop(1))
            "diff" -> cmdDiff(args.drop(1))
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
            graph.segmentStreamable.firstOrNull()?.takeIf { it.claimed }?.let { layout ->
                val headHex = layout.head?.let { hex(it) } ?: "<none>"
                val tail = if (layout.tail > 0) ", accretive tail ${layout.tail} frame(s)" else ""
                println("    layout: streamable through frame ${layout.covered} (head $headHex)$tail")
            }
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

private fun cmdVerify(args: List<String>): Int {
    val paths = mutableListOf<String>()
    val keys = mutableMapOf<String, ByteArray>()
    var i = 0
    while (i < args.size) {
        when (args[i]) {
            "--key" -> {
                if (i + 1 >= args.size) return dieUsage()
                val parsed = parseKey(args[i + 1])
                if (parsed == null) {
                    System.err.println("gts verify: bad --key ${args[i + 1]} (want kid:hexpubkey)")
                    return 2
                }
                keys[parsed.first] = parsed.second
                i += 2
            }
            else -> {
                paths += args[i]
                i++
            }
        }
    }
    if (paths.isEmpty()) return dieUsage()
    var problems = false
    for (path in paths) {
        val data = load(path) ?: return 2
        val graph = read(data, true)
        if (keys.isNotEmpty()) {
            verifySignatures(graph.signatures) { kid -> keys[kid] }
        }
        println(foldSummaryJson(graph))
        if (keys.isNotEmpty()) {
            for (sig in graph.signatures) {
                val kid = sig.kid.ifEmpty { "?" }
                println("  signature $kid: ${sig.status}")
                if (sig.status == "invalid") problems = true
            }
        }
        if (graph.diagnostics.isNotEmpty() || graph.segmentHeads.isEmpty()) problems = true
    }
    return if (problems) 1 else 0
}

private fun parseKey(spec: String): Pair<String, ByteArray>? {
    val idx = spec.indexOf(':')
    if (idx <= 0) return null
    val raw =
        try {
            parseHex(spec.substring(idx + 1))
        } catch (_: RuntimeException) {
            return null
        }
    if (raw.size != 32) return null
    return spec.substring(0, idx) to raw
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
                System.`in`.readBytes().decodeToString()
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

private fun cmdVerifyProof(args: List<String>): Int {
    if (args.size != 1) return dieUsage()
    return try {
        val proof = proofFromJson(Files.readAllBytes(Path.of(args[0])))
        verifyProof(proof)
        println("proof ok: root ${hex(proof.root)} frame ${hex(proof.frameId)}")
        0
    } catch (err: java.io.IOException) {
        System.err.println("gts verify-proof: cannot read ${args[0]}: ${err.message}")
        2
    } catch (err: Exception) {
        System.err.println("gts verify-proof: invalid proof: ${err.message}")
        1
    }
}

private fun cmdHeads(args: List<String>): Int {
    if (args.size != 1) return dieUsage()
    val data = load(args[0]) ?: return 2
    val inventory = inventoryFor(data)
    print(headsJson(inventory))
    return if (inventory.hasProblems()) 1 else 0
}

private fun cmdSegments(args: List<String>): Int {
    if (args.size != 1) return dieUsage()
    val data = load(args[0]) ?: return 2
    val inventory = inventoryFor(data)
    print(segmentsJson(inventory))
    return if (inventory.hasProblems()) 1 else 0
}

private fun cmdMissing(args: List<String>): Int {
    if (args.size != 3 || args[0] != "--from-head") return dieUsage()
    val fromHead =
        try {
            parseHex32(args[1])
        } catch (err: Exception) {
            System.err.println("gts missing: invalid peer head: ${err.message}")
            return 2
        }
    val data = load(args[2]) ?: return 2
    val result = missing(inventoryFor(data), fromHead)
    print(missingJson(result))
    return if (result.status == "error") 1 else 0
}

private fun cmdResume(args: List<String>): Int {
    if (args.size != 3 || args[0] != "--after") return dieUsage()
    val frameId =
        try {
            parseHex32(args[1])
        } catch (err: Exception) {
            System.err.println("gts resume: invalid frame id: ${err.message}")
            return 2
        }
    val data = load(args[2]) ?: return 2
    return try {
        System.out.write(resumeAfter(data, frameId))
        0
    } catch (err: Exception) {
        System.err.println("gts resume: ${err.message}")
        1
    }
}

private fun cmdExtractKey(args: List<String>): Int {
    if (args.isEmpty()) return dieUsage()
    val path = args[0]
    val graph = read(load(path) ?: return 2, true)
    val (kid, gpg) =
        transportKey(graph)
            ?: run {
                System.err.println("$path: no embedded transport key")
                return 1
            }
    println("kid:         $kid")
    try {
        val key = parseTransportKey(gpg)
        println("fingerprint: ${formatFingerprint(key.fingerprint)}")
        println("emojihash:   ${emojihash(key.rawPublic, 11)}")
    } catch (_: Exception) {
        println("fingerprint: ${formatFingerprint(kid)}")
    }
    println(gpg)
    return 0
}

private fun cmdPack(args: List<String>): Int {
    var outPath: String? = null
    val sources = mutableListOf<String>()
    var i = 0
    while (i < args.size) {
        when (args[i]) {
            "-o", "--out" -> {
                if (i + 1 >= args.size) {
                    System.err.println("gts pack: -o requires a path")
                    return 2
                }
                outPath = args[i + 1]
                i += 2
            }
            else -> {
                sources += args[i]
                i++
            }
        }
    }
    if (sources.isEmpty() || outPath == null) return dieUsage()
    return try {
        Files.write(Path.of(outPath), pack(sources.map { Path.of(it) }))
        0
    } catch (err: Exception) {
        System.err.println("gts pack: ${err.message}")
        1
    }
}

private fun cmdUnpack(args: List<String>): Int {
    var dest = Path.of(".")
    var includeSuppressed = false
    val positional = mutableListOf<String>()
    var i = 0
    while (i < args.size) {
        when (args[i]) {
            "-C", "--directory" -> {
                if (i + 1 >= args.size) return dieUsage()
                dest = Path.of(args[i + 1])
                i += 2
            }
            "--include-suppressed" -> {
                includeSuppressed = true
                i++
            }
            else -> {
                positional += args[i]
                i++
            }
        }
    }
    if (positional.size != 1) return dieUsage()
    return try {
        val graph = read(load(positional[0]) ?: return 2, true)
        if (graph.diagnostics.isNotEmpty()) {
            System.err.println("gts unpack: refusing archive with diagnostics")
            return 1
        }
        unpack(graph, dest, includeSuppressed)
        0
    } catch (err: Exception) {
        System.err.println("gts unpack: ${err.message}")
        1
    }
}

private fun cmdDiff(args: List<String>): Int {
    if (args.size != 2) return dieUsage()
    return try {
        val graph = read(load(args[0]) ?: return 2, true)
        if (graph.diagnostics.isNotEmpty()) {
            System.err.println("gts diff: refusing archive with diagnostics")
            return 1
        }
        val lines = diff(graph, Path.of(args[1]))
        lines.forEach(::println)
        if (lines.isEmpty()) 0 else 1
    } catch (err: Exception) {
        System.err.println("gts diff: ${err.message}")
        1
    }
}

private fun cmdLs(args: List<String>): Int {
    if (args.size != 1) return dieUsage()
    val graph = read(load(args[0]) ?: return 2, true)
    if (graph.diagnostics.isNotEmpty()) return 1
    val meta = graph.blobMeta.associateBy { it.digest }
    graph.blobs.sortedBy { it.digest }.forEach { blob ->
        val mt = (meta[blob.digest]?.meta as? ca.blackcatinformatics.gts.CborMap)?.getTextKey("mt").asText() ?: "application/octet-stream"
        println("${blob.digest}\t${blob.data.size}\t$mt")
    }
    return 0
}

private fun cmdExtract(args: List<String>): Int {
    var outPath: String? = null
    var includeSuppressed = false
    var assertedMediaType: String? = null
    val positional = mutableListOf<String>()
    var i = 0
    while (i < args.size) {
        when (args[i]) {
            "-o", "--out" -> {
                if (i + 1 >= args.size) return dieUsage()
                outPath = args[i + 1]
                i += 2
            }
            "--include-suppressed" -> {
                includeSuppressed = true
                i++
            }
            "--mt" -> {
                if (i + 1 >= args.size) return dieUsage()
                assertedMediaType = args[i + 1]
                i += 2
            }
            else -> {
                positional += args[i]
                i++
            }
        }
    }
    if (positional.size != 2) return dieUsage()
    val graph = read(load(positional[0]) ?: return 2, true)
    if (graph.diagnostics.isNotEmpty()) return 1
    val digest = normalizeDigest(positional[1])
    val suppressed = if (includeSuppressed) emptySet() else ca.blackcatinformatics.gts.suppressedBlobDigests(graph)
    if (digest in suppressed) {
        System.err.println("gts extract: blob is suppressed: $digest")
        return 1
    }
    val blob = graph.blobs.firstOrNull { it.digest == digest }
    if (blob == null) {
        System.err.println("gts extract: blob not found: $digest")
        return 1
    }
    if (assertedMediaType != null) {
        val declared =
            (graph.blobMeta.firstOrNull { it.digest == digest }?.meta as? CborMap)
                ?.getTextKey("mt")
                .asText()
                .orEmpty()
        if (declared != assertedMediaType) {
            System.err.println("gts extract: declared media type \"$declared\" does not match asserted \"$assertedMediaType\"")
            return 1
        }
    }
    if (digestStr(blob.data) != digest) {
        System.err.println("gts extract: integrity failure: $digest")
        return 1
    }
    return try {
        if (outPath == null || outPath == "-") {
            System.out.write(blob.data)
        } else {
            Files.write(Path.of(outPath), blob.data)
        }
        0
    } catch (err: Exception) {
        System.err.println("gts extract: ${err.message}")
        2
    }
}

private fun cmdCat(args: List<String>): Int {
    var outPath: String? = null
    val inputs = mutableListOf<String>()
    var i = 0
    while (i < args.size) {
        when (args[i]) {
            "-o", "--out" -> {
                if (i + 1 >= args.size) return dieUsage()
                outPath = args[i + 1]
                i += 2
            }
            else -> {
                inputs += args[i]
                i++
            }
        }
    }
    if (inputs.size < 2) {
        System.err.println("gts: cat needs at least two inputs")
        return dieUsage()
    }

    val chunks = mutableListOf<ByteArray>()
    for (path in inputs) {
        val data = load(path) ?: return 2
        val fs = readFileSegments(data)
        if (fs.fatal != null || fs.torn >= 0 || fs.segments.any { it.diagnostics.isNotEmpty() }) {
            System.err.println("gts: refusing $path: not a clean GTS input")
            return 1
        }
        for ((idx, segment) in fs.segments.withIndex()) {
            val contributes =
                segment.quads.isNotEmpty() ||
                    segment.blobs.isNotEmpty() ||
                    segment.reifiers.isNotEmpty() ||
                    segment.annotations.isNotEmpty() ||
                    segment.suppressions.isNotEmpty()
            if (!contributes) {
                System.err.println("gts: refusing $path: segment $idx folds to nothing (no quads/blobs/reifies/annot/suppress)")
                return 1
            }
        }
        chunks += data
    }

    val combined = ByteArray(chunks.sumOf { it.size })
    var offset = 0
    for (chunk in chunks) {
        chunk.copyInto(combined, offset)
        offset += chunk.size
    }
    if (allQuadsSuppressed(read(combined, true))) {
        System.err.println("gts: refusing composition: suppressions hide every quad in the folded output")
        return 1
    }
    return try {
        if (outPath == null) {
            System.out.write(combined)
        } else {
            Files.write(Path.of(outPath), combined)
        }
        0
    } catch (err: Exception) {
        System.err.println("gts: cannot write output: ${err.message}")
        2
    }
}

private fun cmdCompact(args: List<String>): Int {
    var outPath: String? = null
    var timestamp: String? = null
    var streamable = false
    var sealOriginal = false
    val positional = mutableListOf<String>()
    var i = 0
    while (i < args.size) {
        when (args[i]) {
            "-o", "--out" -> {
                if (i + 1 >= args.size) return dieUsage()
                outPath = args[i + 1]
                i += 2
            }
            "--streamable" -> {
                streamable = true
                i++
            }
            "--seal-original" -> {
                sealOriginal = true
                i++
            }
            "--timestamp" -> {
                if (i + 1 >= args.size) return dieUsage()
                timestamp = args[i + 1]
                i += 2
            }
            else -> {
                positional += args[i]
                i++
            }
        }
    }
    if (positional.size != 1 || outPath == null) return dieUsage()
    if (!streamable) {
        System.err.println("gts: compact requires --streamable")
        return 2
    }
    val data = load(positional[0]) ?: return 2
    val ts = timestamp ?: DateTimeFormatter.ISO_INSTANT.format(Instant.now())
    return try {
        Files.write(Path.of(outPath), streamableCompact(data, ts, sealOriginal))
        0
    } catch (err: CompactRefusedException) {
        System.err.println("gts: refusing compact: ${err.message}")
        1
    } catch (err: Exception) {
        System.err.println("gts compact: ${err.message}")
        1
    }
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

private fun transportKey(graph: Graph): Pair<String, String>? {
    for (entry in graph.meta) {
        if (entry.key != "gts:transportKey") continue
        val map = entry.value as? CborMap ?: return null
        val kid = map.getTextKey("kid").asText()
        val gpg = map.getTextKey("gpg").asText()
        if (kid != null && gpg != null) return kid to gpg
        return null
    }
    return null
}

private fun allQuadsSuppressed(graph: Graph): Boolean {
    if (graph.quads.isEmpty() || graph.suppressions.isEmpty()) return false
    val termSuppressed = mutableSetOf<Int>()
    val quadSuppressed = mutableSetOf<String>()
    for (suppression in graph.suppressions) {
        for (target in suppression.targets) {
            val map = target as? CborMap ?: continue
            when (map.getTextKey("kind").asText()) {
                "term", "reifier" -> map.getTextKey("id").asInt()?.let { termSuppressed += it }
                "quad" -> {
                    val ids = (map.getTextKey("q") as? CborArray)?.value ?: continue
                    val parts = mutableListOf<String>()
                    var valid = true
                    for (id in ids) {
                        val n = id.asInt()
                        if (n == null) {
                            valid = false
                            break
                        }
                        parts += n.toString()
                    }
                    if (valid) quadSuppressed += parts.joinToString(",")
                }
            }
        }
    }
    for (quad in graph.quads) {
        if (quadKey(quad) in quadSuppressed) continue
        if (quad.s in termSuppressed || quad.p in termSuppressed || quad.o in termSuppressed) continue
        if (quad.g != null && quad.g in termSuppressed) continue
        return false
    }
    return true
}

private fun quadKey(quad: Quad): String = listOfNotNull(quad.s, quad.p, quad.o, quad.g).joinToString(",")
