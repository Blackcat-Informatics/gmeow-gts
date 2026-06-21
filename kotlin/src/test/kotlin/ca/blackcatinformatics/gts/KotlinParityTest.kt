// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

package ca.blackcatinformatics.gts

import java.nio.file.Files
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonNull
import kotlinx.serialization.json.boolean
import kotlinx.serialization.json.contentOrNull
import kotlinx.serialization.json.int
import kotlinx.serialization.json.jsonArray
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
import kotlin.test.assertNotNull
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
    fun fullCommittedCorpusMatchesExpectedJson() {
        Files.list(vectors).use { paths ->
            val names =
                paths
                    .filter { it.fileName.toString().endsWith(".gts") }
                    .map { it.fileName.toString().removeSuffix(".gts") }
                    .sorted()
                    .toList()
            assertTrue(names.size >= 16, "expected full top-level vector corpus")
            for (name in names) {
                val expected = Json.parseToJsonElement(vectors.resolve("$name.expected.json").readText()).jsonObject
                val mode = expected["mode"]!!.jsonPrimitive.content
                val graph = read(vectors.resolve("$name.gts").readBytes(), mode != "pre-segment")

                assertEquals(expectedStrings(expected, "diagnostics"), graph.diagnostics.map { it.code }, name)
                assertEquals(expected["terms"]!!.jsonPrimitive.int, graph.terms.size, name)
                assertEquals(expected["quads"]!!.jsonPrimitive.int, graph.quads.size, name)
                assertEquals(expected["segments"]!!.jsonPrimitive.int, graph.segmentHeads.size, name)
                assertEquals(expectedStrings(expected, "segment_heads"), graph.segmentHeads.map { hex(it) }, name)
                assertEquals(expectedStrings(expected, "profiles"), graph.segmentProfiles, name)
                assertEquals(expectedStreamable(expected), actualStreamable(graph), name)
                assertEquals(expectedStrings(expected, "opaque_reasons"), graph.opaque.map { it.reason }.sorted(), name)
                assertEquals(expected["suppressions"]!!.jsonPrimitive.int, graph.suppressions.size, name)
                assertEquals(expectedBlobs(expected), actualBlobs(graph), name)
                assertEquals(expectedStrings(expected, "nquads"), sortedNQuads(graph), name)
            }
        }
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

    @Test
    fun coseSign1VectorsRoundTrip() {
        Files.list(vectors.resolve("cose")).use { paths ->
            paths.filter { it.toString().endsWith(".json") }.forEach { path ->
                val case = Json.parseToJsonElement(path.readText()).jsonObject
                val frameId = parseHex(case["frame_id"]!!.jsonPrimitive.content)
                val seed = parseHex(case["seed"]!!.jsonPrimitive.content)
                val kid = case["kid"]!!.jsonPrimitive.content
                val expected = parseHex(case["cose"]!!.jsonPrimitive.content)
                val publicKey = parseHex(case["pub"]!!.jsonPrimitive.content)

                val got = signId(frameId, CoseSigner(kid, seed))
                assertEquals(expected.toList(), got.toList(), path.toString())
                assertEquals(publicKey.toList(), publicKeyFromSeed(seed).toList(), path.toString())
                assertEquals(kid, signatureKid(expected), path.toString())
                assertEquals(SignatureStatus.VALID, verifySig(expected, frameId, publicKey), path.toString())
                assertEquals(SignatureStatus.INVALID, verifySig(expected, frameId + byteArrayOf(0), publicKey), path.toString())
            }
        }
    }

    @Test
    fun coseEncrypt0VectorRoundTrips() {
        val case = Json.parseToJsonElement(vectors.resolve("encrypt0/basic.json").readText()).jsonObject
        val plaintext = parseHex(case["plaintext"]!!.jsonPrimitive.content)
        val key = parseHex(case["key"]!!.jsonPrimitive.content)
        val iv = parseHex(case["iv"]!!.jsonPrimitive.content)
        val kid = case["kid"]!!.jsonPrimitive.content
        val expected = parseHex(case["cose"]!!.jsonPrimitive.content)

        val got = encrypt0WithIv(plaintext, kid, key, iv)
        assertEquals(expected.toList(), got.toList())
        assertEquals(kid, recipientKid(expected))
        assertEquals(plaintext.toList(), decrypt0(expected) { probe -> if (probe == kid) key else null }.toList())
        val missing = assertFailsWith<Encrypt0Exception> { decrypt0(expected) { null } }
        assertEquals("missing-key", missing.reason)
    }

    @Test
    fun signedWriterFramesVerifyAgainstResolvedKeys() {
        val seed = parseHex("000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f")
        val writer = Writer("generic", signer = CoseSigner("test-kid", seed))
        writer.addBlob("signed".encodeToByteArray(), "text/plain")
        val graph = read(writer.toBytes(), true)
        assertEquals(1, graph.signatures.size)
        val cose = assertNotNull(graph.signatures.single().cose)
        assertEquals("test-kid", signatureKid(cose))

        verifySignatures(graph.signatures) { kid -> if (kid == "test-kid") publicKeyFromSeed(seed) else null }
        assertEquals("test-kid", graph.signatures.single().kid)
        assertEquals("valid", graph.signatures.single().status)
    }

    private fun expectedStrings(expected: kotlinx.serialization.json.JsonObject, key: String): List<String> =
        expected[key]!!.jsonArray.map { it.jsonPrimitive.content }

    private fun expectedStreamable(expected: kotlinx.serialization.json.JsonObject): List<Map<String, Any>> =
        expected["streamable"]!!.jsonArray.map { item ->
            val obj = item.jsonObject
            mapOf(
                "claimed" to obj["claimed"]!!.jsonPrimitive.boolean,
                "covered" to obj["covered"]!!.jsonPrimitive.int,
                "tail" to obj["tail"]!!.jsonPrimitive.int,
            )
        }

    private fun actualStreamable(graph: Graph): List<Map<String, Any>> =
        graph.segmentStreamable.map { item ->
            mapOf(
                "claimed" to item.claimed,
                "covered" to item.covered,
                "tail" to item.tail,
            )
        }

    private fun expectedBlobs(expected: kotlinx.serialization.json.JsonObject): Map<String, Map<String, Any?>> =
        expected["blobs"]!!.jsonObject.mapValues { (_, value) ->
            val obj = value.jsonObject
            mapOf(
                "size" to obj["size"]!!.jsonPrimitive.int,
                "mt" to obj["mt"]!!.takeUnless { it == JsonNull }?.jsonPrimitive?.contentOrNull,
            )
        }

    private fun actualBlobs(graph: Graph): Map<String, Map<String, Any?>> {
        val metaByDigest = graph.blobMeta.associate { it.digest to (it.meta as? CborMap) }
        return graph.blobs.associate { blob ->
            blob.digest to
                mapOf(
                    "size" to blob.data.size,
                    "mt" to metaByDigest[blob.digest]?.getTextKey("mt").asText(),
                )
        }
    }

    private fun sortedNQuads(graph: Graph): List<String> {
        val text = toNQuads(graph).trimEnd('\n')
        return if (text.isEmpty()) emptyList() else text.lines().sorted()
    }
}
