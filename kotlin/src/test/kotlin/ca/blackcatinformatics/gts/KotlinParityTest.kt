// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

package ca.blackcatinformatics.gts

import kotlin.io.path.Path
import kotlin.io.path.readBytes
import kotlin.test.Test
import kotlin.test.assertEquals
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
}
