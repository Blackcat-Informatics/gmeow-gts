// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

package ca.blackcatinformatics.gts

import java.nio.file.Files
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import kotlin.io.path.Path
import kotlin.io.path.createDirectories
import kotlin.io.path.readBytes
import kotlin.io.path.readText
import kotlin.io.path.writeText
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class KotlinParityTest {
    private val vectors = Path("../vectors")

    @Test
    fun foldsMinimalVectorToExpectedNQuads() {
        val graph = read(vectors.resolve("01-minimal.gts").readBytes(), true)
        assertEquals(emptyList(), graph.diagnostics.map { it.code })
        assertEquals(
            "<https://example.org/Cat> <http://www.w3.org/2000/01/rdf-schema#label> \"Cat\"@en .\n",
            toNQuads(graph),
        )
        assertEquals("ec5a15cbe3b79c333712d64ed83a70e69a2d1be8c1316835727e5d5219823cd9", hex(graph.segmentHeads.single()))
    }

    @Test
    fun fromNQuadsRoundTripsFoldedText() {
        val src = read(vectors.resolve("11-datatype-defaulting.gts").readBytes(), false)
        val nq = toNQuads(src)
        val roundTrip = toNQuads(read(fromNQuads(nq), false))
        assertEquals(nq.trim().lines().sorted(), roundTrip.trim().lines().sorted())
    }

    @Test
    fun zstdFrameFolds() {
        val graph = read(vectors.resolve("02-zstd-frame.gts").readBytes(), true)
        assertEquals(emptyList(), graph.diagnostics.map { it.code })
        assertTrue(toNQuads(graph).contains("\"Cat\"@en"))
    }

    @Test
    fun verifiesFrozenMmrProofAndRejectsBadRoot() {
        val proof = proofFromJson(vectors.resolve("proofs/mmr-basic-proof.json").readBytes())
        verifyProof(proof)

        val badProof = proofFromJson(vectors.resolve("proofs/mmr-basic-proof-bad-root.json").readBytes())
        assertFailsWith<IllegalArgumentException> { verifyProof(badProof) }
    }

    @Test
    fun extractKeyVectorMatchesPinnedMaterial() {
        val case = Json.parseToJsonElement(vectors.resolve("openpgp/extract-key.json").readText()).jsonObject
        val graph = read(parseHex(case["gts"]!!.jsonPrimitive.content), true)
        val meta = graph.meta.single { it.key == "gts:transportKey" }.value as CborMap
        val kid = meta.getTextKey("kid").asText()!!
        val gpg = meta.getTextKey("gpg").asText()!!
        val key = parseTransportKey(gpg)
        val stdout =
            "kid:         $kid\n" +
                "fingerprint: ${formatFingerprint(key.fingerprint)}\n" +
                "emojihash:   ${emojihash(key.rawPublic, 11)}\n" +
                "$gpg\n"
        assertEquals(case["stdout"]!!.jsonPrimitive.content, stdout)
    }

    @Test
    fun filesProfilePacksUnpacksAndDiffs() {
        val tmp = Files.createTempDirectory("gts-kotlin-files")
        val src = tmp.resolve("src")
        src.resolve("subdir").createDirectories()
        src.resolve("a.txt").writeText("hello")
        src.resolve("subdir/b.txt").writeText("world")

        val graph = read(pack(listOf(src)), true)
        val dst = tmp.resolve("dst")
        unpack(graph, dst)
        assertEquals("hello", dst.resolve("a.txt").readText())
        assertEquals("world", dst.resolve("subdir/b.txt").readText())
        assertEquals(emptyList(), diff(graph, dst))

        dst.resolve("a.txt").writeText("changed")
        dst.resolve("new.txt").writeText("new")
        Files.delete(dst.resolve("subdir/b.txt"))
        assertEquals(listOf("added: new.txt", "modified: a.txt", "removed: subdir/b.txt"), diff(graph, dst))
    }

    @Test
    fun replicationInventoryAndResumeUseCleanByteBoundaries() {
        val first = Writer("generic")
        val firstHead = first.addBlob("a".encodeToByteArray(), "text/plain")
        val firstBytes = first.toBytes()
        val second = Writer("generic")
        val secondHead = second.addBlob("b".encodeToByteArray(), "text/plain")
        val secondBytes = second.toBytes()
        val combined = firstBytes + secondBytes

        val inventory = inventoryFor(combined)
        assertFalse(inventory.hasProblems())
        assertTrue(headsJson(inventory).contains("\"${hex(firstHead)}\""))
        assertTrue(headsJson(inventory).contains("\"${hex(secondHead)}\""))
        assertTrue(segmentsJson(inventory).contains("\"item_count\":4"))

        val result = missing(inventory, firstHead)
        assertEquals("ranges", result.status)
        assertEquals(firstBytes.size, result.ranges.single().start)
        assertEquals(combined.size, result.ranges.single().end)
        assertEquals(secondBytes.toList(), resumeAfter(combined, firstHead).toList())
    }

    @Test
    fun compactReproducesFrozenStreamableVector() {
        val source = vectors.resolve("25-streamable-source.gts").readBytes()
        val expected = vectors.resolve("25b-streamable-compacted.gts").readBytes()
        val got = streamableCompact(source, "2026-01-01T00:00:00Z", false)
        assertEquals(expected.toList(), got.toList())
        val graph = read(got, true)
        assertEquals(emptyList(), graph.diagnostics.map { it.code })
        assertTrue(graph.segmentStreamable.single().claimed)
        assertEquals(0, graph.segmentStreamable.single().tail)
    }
}
