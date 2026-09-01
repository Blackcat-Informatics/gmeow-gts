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
import kotlin.test.assertContentEquals
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
    fun zstdDecodesPayloadAboveFormerSafetyBound() {
        val payload = ByteArray(16 * 1024 * 1024 + 1)
        val encoded = encodeChain(listOf(Codec("zstd", "compress")), payload)
        val decoded = decodeChain(listOf(Codec("zstd", "compress")), encoded)

        assertEquals(payload.size, decoded.size)
        assertContentEquals(payload, decoded)
    }

    @Test
    fun fromNQuadsPreservesDirectionalLanguageLiterals() {
        val nq = "<https://ex/s> <https://ex/label> \"RTL\"@ar--rtl .\n"
        val graph = read(fromNQuads(nq), false)
        val literal = graph.terms.single { it.kind == TermKind.LITERAL }
        assertEquals("ar", literal.lang)
        assertEquals("rtl", literal.direction)
        assertEquals(RDF_DIR_LANG_STRING, graph.datatypeIri(literal))
        assertEquals(nq.trim().lines().sorted(), toNQuads(graph).trim().lines().sorted())
    }

    @Test
    fun writerAllowsMultipleReifiersForSameStatement() {
        val writer = Writer("dist")
        writer.addTerms(
            listOf(
                Term(TermKind.IRI, "https://ex/r1"),
                Term(TermKind.IRI, "https://ex/r2"),
                Term(TermKind.IRI, "https://ex/s"),
                Term(TermKind.IRI, "https://ex/p"),
                Term(TermKind.IRI, "https://ex/o"),
            ),
        )
        writer.addQuads(listOf(Quad(2, 3, 4)))
        writer.addReifies(
            listOf(
                ReifierEntry(0, Triple(2, 3, 4)),
                ReifierEntry(1, Triple(2, 3, 4)),
            ),
        )
        assertEquals(2, read(writer.toBytes(), false).reifiers.size)
    }

    @Test
    fun fromNQuadsPreservesMultipleReifiersForSameStatement() {
        val rdfReifies = "http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies"
        val nq =
            "<https://ex/r1> <$rdfReifies> <<( <https://ex/s> <https://ex/p> <https://ex/o> )>> .\n" +
                "<https://ex/r2> <$rdfReifies> <<( <https://ex/s> <https://ex/p> <https://ex/o> )>> .\n"
        val graph = read(fromNQuads(nq), false)
        assertEquals(2, graph.reifiers.size)
        assertEquals(nq.trim().lines().sorted(), toNQuads(graph).trim().lines().sorted())
    }

    /** Terms 0..4 shared by the §7.3 statement-layer tests: r, s, p, "Cat"@en, "Chat"@fr. */
    private fun statementLayerTerms(): List<Term> =
        listOf(
            Term(TermKind.IRI, "https://ex/r"),
            Term(TermKind.IRI, "https://ex/s"),
            Term(TermKind.IRI, "https://ex/p"),
            Term(TermKind.LITERAL, "Cat", lang = "en"),
            Term(TermKind.LITERAL, "Chat", lang = "fr"),
        )

    @Test
    fun multiValuedReifiesKeepsEveryRowWithoutDiagnostic() {
        val writer = Writer("generic")
        writer.addTerms(statementLayerTerms())
        writer.addReifies(listOf(ReifierEntry(0, Triple(1, 2, 3)), ReifierEntry(0, Triple(1, 2, 4))))

        val graph = read(writer.toBytes(), true)
        assertEquals(emptyList(), graph.diagnostics.map { it.code })
        assertEquals(2, graph.reifiers.size)
        assertEquals(listOf(Triple(1, 2, 3), Triple(1, 2, 4)), graph.reifierTriples(0))
        assertEquals(2, sortedNQuads(graph).size)
    }

    @Test
    fun byteIdenticalReifiesRowsStillCollapse() {
        val writer = Writer("generic")
        writer.addTerms(statementLayerTerms())
        writer.addReifies(listOf(ReifierEntry(0, Triple(1, 2, 3)), ReifierEntry(0, Triple(1, 2, 3))))

        val graph = read(writer.toBytes(), true)
        assertEquals(emptyList(), graph.diagnostics.map { it.code })
        assertEquals(1, graph.reifiers.size)
        assertEquals(listOf(Triple(1, 2, 3)), graph.reifierTriples(0))
    }

    @Test
    fun conflictingReifierFlagsOnlyTheLegacyTripleTermAndDropsNoRow() {
        val writer = Writer("generic")
        writer.addTerms(statementLayerTerms() + Term(TermKind.TRIPLE, "", reifier = 0))
        writer.addReifies(listOf(ReifierEntry(0, Triple(1, 2, 3)), ReifierEntry(0, Triple(1, 2, 4))))
        writer.addQuads(listOf(Quad(5, 2, 1)))

        val graph = read(writer.toBytes(), true)
        assertEquals(listOf("ConflictingReifier"), graph.diagnostics.map { it.code })
        assertEquals(null, graph.diagnostics.single().frameIndex)
        assertEquals(2, graph.reifiers.size)
        assertEquals(Triple(1, 2, 3), graph.tripleOf(5))
    }

    @Test
    fun selfDescribingTripleTermsNeverRaiseConflictingReifier() {
        val writer = Writer("generic")
        writer.addTerms(statementLayerTerms() + Term(TermKind.TRIPLE, "", reifier = 0, triple = Triple(1, 2, 3)))
        writer.addReifies(listOf(ReifierEntry(0, Triple(1, 2, 3)), ReifierEntry(0, Triple(1, 2, 4))))
        writer.addQuads(listOf(Quad(5, 2, 1)))

        val graph = read(writer.toBytes(), true)
        assertEquals(emptyList(), graph.diagnostics.map { it.code })
        assertEquals(2, graph.reifiers.size)
        assertEquals(Triple(1, 2, 3), graph.tripleOf(5))
    }

    @Test
    fun distinctTripleTermsSharingAReifierIdStayDistinct() {
        val writer = Writer("generic")
        writer.addTerms(
            statementLayerTerms() +
                listOf(
                    Term(TermKind.TRIPLE, "", reifier = 0, triple = Triple(1, 2, 3)),
                    Term(TermKind.TRIPLE, "", reifier = 0, triple = Triple(1, 2, 4)),
                ),
        )
        writer.addReifies(listOf(ReifierEntry(0, Triple(1, 2, 3)), ReifierEntry(0, Triple(1, 2, 4))))
        writer.addQuads(listOf(Quad(5, 2, 1), Quad(6, 2, 1)))

        val single = read(writer.toBytes(), true)
        assertEquals(emptyList(), single.diagnostics.map { it.code })
        assertEquals(Triple(1, 2, 3), single.tripleOf(5))
        assertEquals(Triple(1, 2, 4), single.tripleOf(6))

        val union = read(writer.toBytes() + writer.toBytes(), true)
        assertEquals(emptyList(), union.diagnostics.map { it.code })
        assertEquals(2, union.segmentHeads.size)
        val quoted = union.terms.indices.filter { union.terms[it].kind == TermKind.TRIPLE }
        assertEquals(2, quoted.size)
        assertEquals(2, quoted.mapNotNull { union.tripleOf(it) }.distinct().size)
    }

    /**
     * One segment: `<reifierIri>` reifies `(s p objectIri)`, quoted by a LEGACY `"tt"`-less triple
     * term (id 4) that the segment's only quad uses as its subject.
     */
    private fun legacyReifierSegment(reifierIri: String, objectIri: String): ByteArray {
        val writer = Writer("generic")
        writer.addTerms(
            listOf(
                Term(TermKind.IRI, reifierIri),
                Term(TermKind.IRI, "https://ex/s"),
                Term(TermKind.IRI, "https://ex/p"),
                Term(TermKind.IRI, objectIri),
                Term(TermKind.TRIPLE, "", reifier = 0),
            ),
        )
        writer.addReifies(listOf(ReifierEntry(0, Triple(1, 2, 3))))
        writer.addQuads(listOf(Quad(4, 2, 3)))
        return writer.toBytes()
    }

    @Test
    fun unionMergesTripleTermsWithTheSameResolvedTriple() {
        // §7.8: triple-term equality IS equality of the RESOLVED (s, p, o), so two legacy terms
        // that hang off DIFFERENT reifier ids but resolve to the SAME triple are one term.
        val union =
            read(
                legacyReifierSegment("https://ex/r1", "https://ex/o") +
                    legacyReifierSegment("https://ex/r2", "https://ex/o"),
                true,
            )

        assertEquals(emptyList(), union.diagnostics.map { it.code })
        assertEquals(2, union.segmentHeads.size)
        assertEquals(1, union.terms.count { it.kind == TermKind.TRIPLE })
        assertEquals(1, union.quads.size)
    }

    @Test
    fun unionSeparatesLegacyTripleTermsWithDifferentResolvedTriples() {
        val union =
            read(
                legacyReifierSegment("https://ex/r", "https://ex/o1") +
                    legacyReifierSegment("https://ex/r", "https://ex/o2"),
                true,
            )

        // Both segments bind one reifier to different triples: legal RDF 1.2, so both rows and
        // both terms survive — and the over-bound LEGACY terms are what §7.3 still flags, once
        // each, on the union (the extra binding arrives from the other segment).
        assertEquals(listOf("ConflictingReifier", "ConflictingReifier"), union.diagnostics.map { it.code })
        assertEquals(2, union.reifiers.size)
        assertEquals(2, union.terms.count { it.kind == TermKind.TRIPLE })
        assertEquals(2, union.quads.size)
    }

    @Test
    fun unreifiedTripleTermRoundTripsAsItself() {
        val nq = "<https://ex/s> <https://ex/p> <<( <https://ex/a> <https://ex/b> <https://ex/c> )>> .\n"
        val graph = read(fromNQuads(nq), false)

        assertEquals(emptyList(), graph.diagnostics.map { it.code })
        assertTrue(graph.reifiers.isEmpty(), "an unreified triple term mints no reifies row")
        val quoted = graph.terms.single { it.kind == TermKind.TRIPLE }
        assertEquals(null, quoted.reifier)
        assertNotNull(quoted.triple)
        assertEquals(nq, toNQuads(graph))
    }

    @Test
    fun forwardReferencingTripleTermComponentsAreDroppedWithDiagnostic() {
        val writer = Writer("generic")
        writer.addFrame(
            "terms",
            cborArray(
                cborMap(text("k") to uint(TermKind.IRI.wire), text("v") to text("https://ex/s")),
                cborMap(
                    text("k") to uint(TermKind.TRIPLE.wire),
                    text("tt") to cborArray(uint(0), uint(7), uint(0)),
                ),
            ),
        )

        val graph = read(writer.toBytes(), true)
        assertEquals(listOf("ForwardReference"), graph.diagnostics.map { it.code })
        assertEquals(null, graph.terms[1].triple)
        assertEquals(null, graph.tripleOf(1))
    }

    @Test
    fun tripleTermPositionViolationsDropTtAndFallBackToTheReifier() {
        val writer = Writer("generic")
        writer.addTerms(
            statementLayerTerms() +
                // "tt" names a LITERAL predicate (term 3), which §7.4 forbids.
                Term(TermKind.TRIPLE, "", reifier = 0, triple = Triple(1, 3, 4)),
        )
        writer.addReifies(listOf(ReifierEntry(0, Triple(1, 2, 3))))

        val graph = read(writer.toBytes(), true)
        assertEquals(listOf("PositionConstraint"), graph.diagnostics.map { it.code })
        assertEquals(null, graph.terms[5].triple)
        assertEquals(Triple(1, 2, 3), graph.tripleOf(5))
    }

    @Test
    fun cborMapsUseRfc8949LexicographicKeyOrder() {
        val got = encode(cborMap(text("") to uint(1), uint(1000) to uint(2)))
        assertEquals("a21903e8026001", hex(got))
    }

    @Test
    fun fromNQuadsParsesSupplementaryUnicodeEscapes() {
        val source = "<https://example.org/s> <https://example.org/p> \"\\U0001F63A\" .\n"
        val expected = String(Character.toChars(0x1f63a))
        val graph = read(fromNQuads(source), false)
        assertEquals(
            "<https://example.org/s> <https://example.org/p> \"$expected\" .\n",
            toNQuads(graph),
        )
    }

    @Test
    fun nQuadsEscapesC0LiteralControls() {
        val writer = Writer("generic")
        writer.addTerms(
            listOf(
                Term(TermKind.IRI, "https://example.org/s"),
                Term(TermKind.IRI, "https://example.org/p"),
                Term(TermKind.LITERAL, "\b\u000c\u0001"),
            ),
        )
        writer.addQuads(listOf(Quad(0, 1, 2)))

        assertEquals(
            "<https://example.org/s> <https://example.org/p> \"\\u0008\\u000C\\u0001\" .\n",
            toNQuads(read(writer.toBytes(), true)),
        )
    }

    @Test
    fun negativeTermReferencesProduceDiagnostics() {
        val writer = Writer("generic")
        writer.addFrame(
            "terms",
            cborArray(
                cborMap(text("k") to uint(TermKind.LITERAL.wire), text("v") to text("bad"), text("dt") to CborNInt(-1)),
            ),
        )

        val graph = read(writer.toBytes(), true)
        assertTrue(graph.diagnostics.any { it.code == "ForwardReference" })
    }

    @Test
    fun corruptGzipPayloadBecomesDamagedFrame() {
        val headerUnsigned =
            cborMap(
                text("cat") to cborMap(uint(1) to cborMap(text("cls") to text("compress"), text("name") to text("gzip"))),
                text("gts") to text(MAGIC),
                text("prof") to text("generic"),
                text("v") to uint(VERSION),
            )
        val headerId = headerId(headerUnsigned)
        val header = CborTag(SELF_DESCRIBE_TAG, CborMap(headerUnsigned.value + (text("id") to bytes(headerId))))
        val frameUnsigned =
            cborMap(
                text("d") to bytes(byteArrayOf(0x00, 0x01, 0x02)),
                text("prev") to bytes(headerId),
                text("t") to text("blob"),
                text("x") to cborArray(uint(1)),
            )
        val frame = CborMap(frameUnsigned.value + (text("id") to bytes(contentId(frameUnsigned))))

        val graph = read(encode(header) + encode(frame), true)
        assertEquals(listOf("DamagedFrame"), graph.diagnostics.map { it.code })
        assertEquals(listOf("damaged"), graph.opaque.map { it.reason })
    }

    @Test
    fun publicReaderReportsMalformedInputDiagnosticsWithoutThrowing() {
        val writer = Writer("generic")
        val torn = writer.toBytes() + byteArrayOf(0xa3.toByte())
        val cases =
            listOf(
                byteArrayOf() to listOf("EmptyFile"),
                byteArrayOf(0x01) to listOf("DamagedFrame"),
                torn to listOf("TornAppendError"),
            )
        for ((data, expected) in cases) {
            assertEquals(expected, read(data, true).diagnostics.map { it.code })
        }
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

    /**
     * A `reifies` row that names the very TERM resolving through it.
     *
     * Built through the real writer, not hand-forged: the row `(0, (2, 1, 1))` sits alongside
     * term `2 = k:3 rf=0`. Resolution MUST terminate (§7.3) — the union interns a triple term on
     * its RESOLVED components and the N-Quads projection walks them, so both would recurse
     * forever without a guard.
     */
    private fun selfReachingSegment(): ByteArray {
        val writer = Writer("generic")
        writer.addTerms(
            listOf(
                Term(TermKind.IRI, "https://example.org/r1"),
                Term(TermKind.IRI, "https://example.org/p"),
                Term(TermKind.TRIPLE, "", reifier = 0),
            ),
        )
        writer.addReifies(listOf(ReifierEntry(0, Triple(2, 1, 1))))
        writer.addQuads(listOf(Quad(2, 1, 1)))
        return writer.toBytes()
    }

    @Test
    fun selfReachingTripleTermFoldsAndProjects() {
        val graph = read(selfReachingSegment(), true)
        assertEquals(emptyList(), graph.diagnostics.map { it.code })
        assertEquals(
            listOf(
                "<<( _:unbound_triple_2 <https://example.org/p> <https://example.org/p> )>> " +
                    "<https://example.org/p> <https://example.org/p> .",
                "<https://example.org/r1> <http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies> " +
                    "<<( <<( _:unbound_triple_2 <https://example.org/p> <https://example.org/p> )>> " +
                    "<https://example.org/p> <https://example.org/p> )>> .",
            ),
            sortedNQuads(graph),
        )
    }

    @Test
    fun selfReachingTripleTermUnionTerminates() {
        val one = selfReachingSegment()
        val graph = read(one + one, true)
        // Each segment keeps its own quad: a self-reaching term has no resolved value to
        // compare, so it is never merged with anything it cannot be proven equal to.
        assertEquals(2, graph.quads.size)
        assertTrue(graph.quads.all { graph.terms[it.s].kind == TermKind.TRIPLE })
        assertEquals(setOf("ConflictingReifier"), graph.diagnostics.map { it.code }.toSet())
        assertEquals(4, sortedNQuads(graph).size)
    }

    private fun sortedNQuads(graph: Graph): List<String> {
        val text = toNQuads(graph).trimEnd('\n')
        return if (text.isEmpty()) emptyList() else text.lines().sorted()
    }
}
