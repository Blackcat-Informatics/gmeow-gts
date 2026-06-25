// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

package ca.blackcatinformatics.gts

import java.nio.file.Files
import java.nio.file.LinkOption
import java.nio.file.Path
import java.nio.file.StandardCopyOption
import java.nio.file.attribute.FileTime
import java.nio.file.attribute.PosixFilePermissions
import java.time.Instant
import java.time.format.DateTimeFormatter
import kotlin.io.path.absolute
import kotlin.io.path.createDirectories
import kotlin.io.path.exists
import kotlin.io.path.isDirectory
import kotlin.io.path.isRegularFile
import kotlin.io.path.isSymbolicLink
import kotlin.io.path.name
import kotlin.io.path.readBytes
import kotlin.io.path.relativeTo
import kotlin.io.path.writeBytes

private const val FILES_NS = "https://w3id.org/gts/files#"
private const val RDF_TYPE = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type"
private const val XSD_INTEGER = "http://www.w3.org/2001/XMLSchema#integer"
private const val XSD_DATETIME = "http://www.w3.org/2001/XMLSchema#dateTime"

class FilesProfileException(message: String) : IllegalArgumentException(message)

private fun filesProfileError(message: String): Nothing = throw FilesProfileException(message)

private fun iriTerm(value: String) = Term(TermKind.IRI, value)

private fun literalTerm(value: String, datatype: Int? = null) = Term(TermKind.LITERAL, value, datatype = datatype)

private fun bnodeTerm(value: String) = Term(TermKind.BNODE, value)

fun pack(sources: List<Path>): ByteArray {
    val writer = Writer("files")
    val shared =
        listOf(
            iriTerm(FILES_NS + "FileEntry"),
            iriTerm(FILES_NS + "path"),
            iriTerm(FILES_NS + "digest"),
            iriTerm(FILES_NS + "size"),
            iriTerm(FILES_NS + "mode"),
            iriTerm(FILES_NS + "modified"),
            iriTerm(FILES_NS + "mediaType"),
            iriTerm(RDF_TYPE),
            iriTerm(XSD_INTEGER),
            iriTerm(XSD_DATETIME),
        )
    writer.addTerms(shared)
    val fileEntryId = 0
    val pathId = 1
    val digestId = 2
    val sizeId = 3
    val modeId = 4
    val modifiedId = 5
    val mediaTypeId = 6
    val typeId = 7
    val xsdIntegerId = 8
    val xsdDateTimeId = 9

    val entries = resolveSources(sources)
    val fileTerms = mutableListOf<Term>()
    val quads = mutableListOf<Quad>()
    val blobs = linkedMapOf<String, Pair<ByteArray, String>>()
    for ((idx, entry) in entries.withIndex()) {
        val data = entry.source.readBytes()
        val digest = digestStr(data)
        val size = Files.size(entry.source)
        val mode = fileMode(entry.source)
        val modified = formatDateTime(Files.getLastModifiedTime(entry.source).toInstant())
        val mediaType = guessMediaType(entry.source)
        val base = shared.size + fileTerms.size
        fileTerms +=
            listOf(
                bnodeTerm("f$idx"),
                literalTerm(entry.archivePath),
                literalTerm(digest),
                literalTerm(size.toString(), xsdIntegerId),
                literalTerm(mode.toString(), xsdIntegerId),
                literalTerm(modified, xsdDateTimeId),
                literalTerm(mediaType),
            )
        quads +=
            listOf(
                Quad(base, typeId, fileEntryId),
                Quad(base, pathId, base + 1),
                Quad(base, digestId, base + 2),
                Quad(base, sizeId, base + 3),
                Quad(base, modeId, base + 4),
                Quad(base, modifiedId, base + 5),
                Quad(base, mediaTypeId, base + 6),
            )
        blobs.putIfAbsent(digest, data to mediaType)
    }
    if (fileTerms.isNotEmpty()) writer.addTerms(fileTerms)
    if (quads.isNotEmpty()) writer.addQuads(quads)
    blobs.values.forEach { (data, mediaType) -> writer.addBlob(data, mediaType) }
    return writer.toBytes()
}

fun unpack(graph: Graph, dest: Path, includeSuppressed: Boolean = false) {
    val entries = readFileEntries(graph)
    val blobByDigest = graph.blobs.associateBy({ it.digest }, { it.data })
    val suppressed = if (includeSuppressed) emptySet() else suppressedBlobDigests(graph)
    dest.createDirectories()
    for ((archivePath, entry) in entries) {
        val digest = entry["digest"] ?: filesProfileError("missing digest for $archivePath")
        if (digest in suppressed) continue
        val data = blobByDigest[digest] ?: filesProfileError("missing inline blob for $archivePath: $digest")
        if (digestStr(data) != digest) filesProfileError("integrity failure for $archivePath: $digest")
        val target = destPath(dest, archivePath)
        target.parent?.createDirectories()
        val tmp = target.parent.resolve(".${target.fileName}.gts-tmp-${ProcessHandle.current().pid()}")
        tmp.writeBytes(data)
        if (target.exists(LinkOption.NOFOLLOW_LINKS) && target.isSymbolicLink()) {
            Files.deleteIfExists(tmp)
            filesProfileError("refusing to write through symlink: $archivePath")
        }
        Files.move(tmp, target, StandardCopyOption.REPLACE_EXISTING, StandardCopyOption.ATOMIC_MOVE)
        entry["mode"]?.toIntOrNull()?.let { setFileMode(target, it) }
        entry["modified"]?.let { parseDateTime(it) }?.let { Files.setLastModifiedTime(target, FileTime.from(it)) }
    }
}

fun diff(graph: Graph, directory: Path): List<String> {
    val archive = readFileEntries(graph).mapValues { it.value["digest"].orEmpty() }
    val disk =
        walkDirSorted(directory).associate { file ->
            directory.relativize(file).joinToStringPath() to digestStr(file.readBytes())
        }
    val lines = mutableListOf<String>()
    for (path in archive.keys) if (path !in disk) lines += "removed: $path"
    for (path in disk.keys) if (path !in archive) lines += "added: $path"
    for ((path, digest) in archive) if (disk[path] != null && disk[path] != digest) lines += "modified: $path"
    return lines.sorted()
}

private data class SourceEntry(val source: Path, val archivePath: String)

private fun resolveSources(sources: List<Path>): List<SourceEntry> {
    val out = mutableListOf<SourceEntry>()
    val seen = mutableSetOf<String>()
    for (source in sources) {
        if (source.isSymbolicLink()) filesProfileError("symlink not supported: $source")
        when {
            source.isDirectory(LinkOption.NOFOLLOW_LINKS) -> {
                for (file in walkDirSorted(source)) {
                    val rel = source.relativize(file).joinToStringPath()
                    safeArchivePath(rel)
                    if (!seen.add(rel)) filesProfileError("duplicate archive path: $rel")
                    out += SourceEntry(file, rel)
                }
            }
            source.isRegularFile(LinkOption.NOFOLLOW_LINKS) -> {
                val name = source.name
                safeArchivePath(name)
                if (!seen.add(name)) filesProfileError("duplicate archive path: $name")
                out += SourceEntry(source, name)
            }
            else -> filesProfileError("unsupported source type: $source")
        }
    }
    return out.sortedBy { it.archivePath }
}

private fun walkDirSorted(dir: Path): List<Path> {
    val out = mutableListOf<Path>()
    Files.walk(dir).use { stream ->
        stream.sorted().forEach { path ->
            if (path == dir) return@forEach
            if (path.isSymbolicLink()) filesProfileError("symlink not supported: $path")
            if (path.isRegularFile(LinkOption.NOFOLLOW_LINKS)) out.add(path)
        }
    }
    return out.sortedBy { it.toString() }
}

private fun Path.joinToStringPath(): String = joinToString("/") { it.toString() }

private fun safeArchivePath(path: String) {
    require(path.isNotEmpty()) { "empty archive path" }
    val normalized = path.replace('\\', '/')
    require(!Regex("^[a-zA-Z]:").containsMatchIn(path) && !normalized.startsWith("/")) {
        "absolute or drive-relative path not allowed in archive: $path"
    }
    require('\\' !in path) { "backslash path separator not allowed in archive: $path" }
    val parts = normalized.split("/")
    require(parts.none { it == ".." }) { "path traversal not allowed in archive: $path" }
    require(parts.none { it.isEmpty() || it == "." }) { "empty or current-directory path component not allowed in archive: $path" }
}

private fun readFileEntries(graph: Graph): Map<String, Map<String, String>> {
    var typeId: Int? = null
    var fileEntryId: Int? = null
    val fieldIds = mutableMapOf<String, Int>()
    for ((idx, term) in graph.terms.withIndex()) {
        if (term.kind != TermKind.IRI) continue
        when (term.value) {
            RDF_TYPE -> typeId = idx
            FILES_NS + "FileEntry" -> fileEntryId = idx
            else -> if (term.value.startsWith(FILES_NS)) fieldIds[term.value.removePrefix(FILES_NS)] = idx
        }
    }
    val t = typeId ?: filesProfileError("not a files-profile archive: missing rdf:type")
    val fileEntry = fileEntryId ?: filesProfileError("not a files-profile archive: missing FileEntry")
    val entries = mutableMapOf<Int, MutableMap<String, String>>()
    val fileSubjects = mutableSetOf<Int>()
    for (quad in graph.quads) {
        if (quad.p == t && quad.o == fileEntry) {
            fileSubjects += quad.s
            entries.getOrPut(quad.s) { mutableMapOf() }
        } else {
            for ((name, id) in fieldIds) {
                if (quad.p == id) {
                    entries.getOrPut(quad.s) { mutableMapOf() }[name] = graph.terms.getOrNull(quad.o)?.value ?: ""
                }
            }
        }
    }
    val byPath = mutableMapOf<String, Map<String, String>>()
    for ((subject, entry) in entries) {
        if (subject !in fileSubjects) continue
        val path = entry["path"] ?: continue
        if (path in byPath) filesProfileError("duplicate files:path in archive: $path")
        byPath[path] = entry
    }
    return byPath
}

fun suppressedBlobDigests(graph: Graph): Set<String> =
    graph.suppressions.flatMap { suppression ->
        suppression.targets.mapNotNull { target ->
            val map = target as? CborMap ?: return@mapNotNull null
            val kind = map.getTextKey("kind").asText()
            val digest = digestFromValue(map.getTextKey("digest"))
            if (kind == "blob" && digest.isNotEmpty()) digest else null
        }
    }.toSet()

fun digestFromValue(value: CborValue?): String =
    when (value) {
        is CborText -> normalizeDigest(value.value)
        is CborBytes -> "blake3:${hex(value.value.bytes)}"
        else -> ""
    }

private fun destPath(dest: Path, archivePath: String): Path {
    safeArchivePath(archivePath)
    val destCanon = dest.absolute().toRealPath()
    val target = destCanon.resolve(archivePath).normalize()
    var ancestor = target.parent
    while (ancestor != null && !ancestor.exists(LinkOption.NOFOLLOW_LINKS)) ancestor = ancestor.parent
    val ancestorCanon = (ancestor ?: destCanon).toRealPath()
    require(ancestorCanon.startsWith(destCanon)) { "path escapes destination: $archivePath" }
    return target
}

private fun guessMediaType(path: Path): String =
    when (path.fileName.toString().substringAfterLast('.', "").lowercase()) {
        "txt" -> "text/plain"
        "html", "htm" -> "text/html"
        "json" -> "application/json"
        "xml" -> "application/xml"
        "png" -> "image/png"
        "jpg", "jpeg" -> "image/jpeg"
        "gif" -> "image/gif"
        "webp" -> "image/webp"
        "pdf" -> "application/pdf"
        "zip" -> "application/zip"
        "gz" -> "application/gzip"
        "tar" -> "application/x-tar"
        else -> "application/octet-stream"
    }

private fun formatDateTime(instant: Instant): String = DateTimeFormatter.ISO_INSTANT.format(instant.truncatedTo(java.time.temporal.ChronoUnit.SECONDS))

private fun parseDateTime(value: String): Instant? =
    try {
        Instant.parse(value)
    } catch (_: RuntimeException) {
        try {
            Instant.parse("${value}Z")
        } catch (_: RuntimeException) {
            null
        }
    }

private fun fileMode(path: Path): Int =
    try {
        PosixFilePermissions.toString(Files.getPosixFilePermissions(path, LinkOption.NOFOLLOW_LINKS)).toModeBits()
    } catch (_: UnsupportedOperationException) {
        420
    }

private fun setFileMode(path: Path, mode: Int) {
    try {
        Files.setPosixFilePermissions(path, mode.toPosixPermissions())
    } catch (_: UnsupportedOperationException) {
    }
}

private fun String.toModeBits(): Int {
    var mode = 0
    val bits = listOf(256, 128, 64, 32, 16, 8, 4, 2, 1)
    for ((idx, ch) in withIndex()) {
        if (ch != '-') mode = mode or bits[idx]
    }
    return mode
}

private fun Int.toPosixPermissions() =
    PosixFilePermissions.fromString(
        buildString {
            append(if (this@toPosixPermissions and 256 != 0) 'r' else '-')
            append(if (this@toPosixPermissions and 128 != 0) 'w' else '-')
            append(if (this@toPosixPermissions and 64 != 0) 'x' else '-')
            append(if (this@toPosixPermissions and 32 != 0) 'r' else '-')
            append(if (this@toPosixPermissions and 16 != 0) 'w' else '-')
            append(if (this@toPosixPermissions and 8 != 0) 'x' else '-')
            append(if (this@toPosixPermissions and 4 != 0) 'r' else '-')
            append(if (this@toPosixPermissions and 2 != 0) 'w' else '-')
            append(if (this@toPosixPermissions and 1 != 0) 'x' else '-')
        },
    )
