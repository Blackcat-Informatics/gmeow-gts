// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package ca.blackcatinformatics.gts.v2

import kotlin.io.path.Path
import kotlin.io.path.readBytes
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertIs
import kotlin.test.assertTrue

class RdfProjectionTest {
    @Test
    fun foldsQuadsReifiersAnnotationsAndEmptyGraphNames() {
        val projected = renderRdfProjection(fixture("project__reifier-annotation"))
        assertEquals(2, projected.lineSequence().count { it.isNotEmpty() })
        assertTrue(projected.contains("<http://e/r> <http://e/q> <http://e/v> <http://e/g> ."))
        assertTrue(projected.contains("rdf-syntax-ns#reifies> <<( <http://e/s> <http://e/p> <http://e/o> )>> <http://e/g> ."))
        assertEquals("<http://e/r> <http://e/q> <http://e/v> <http://e/g> .\n", renderRdfProjection(fixture("project__annotation-equals-quad")))
        val empty = fixture("project__empty-graph")
        assertEquals(1, empty.emptyGraphs.size)
        assertEquals("", renderRdfProjection(empty))
    }

    @Test
    fun resolvesOnlyVerifiedEarlierValueCandidates() {
        val expected = "<http://e/s> <http://e/p> \"abc\"^^<http://e/dt> .\n"
        assertEquals(expected, renderRdfProjection(fixture("terms-literal-ref-text")))
        assertEquals(expected, renderRdfProjection(fixture("terms-literal-ref-first-candidate.failing-first")))
        val later = assertFailsWith<RdfProjectionFailure> { fixture("terms-literal-ref-value-after.same-segment-after") }
        assertTrue(later.message.orEmpty().contains("missing_value"))
        val invalid = assertFailsWith<RdfProjectionFailure> { fixture("terms-literal-ref-not-utf8") }
        assertTrue(invalid.message.orEmpty().contains("ill_formed_term"))
    }

    @Test
    fun validatesRdfSyntaxOnlyWhenRenderingTheTypedProjection() {
        val projected = fixture("project__malformed-iri")
        assertEquals(1, projected.graph.quads.size)
        assertFailsWith<RdfProjectionFailure> { renderRdfProjection(projected) }
    }

    @Test
    fun distinguishesMalformedPayloadCborFromOuterFraming() {
        val writer = V2SegmentWriter()
        writer.append(1, "840081820101021840".hexToByteArray(), "8281".hexToByteArray())
        writer.commit()
        val wire = assertIs<WireInspection.VersionTwo>(inspectWire(writer.toBytes(), retainDecoded = true))
        val invalid = assertFailsWith<RdfProjectionFailure> { projectRdf(wire.events) }
        assertEquals("byte 11: wrong_length", invalid.message)
    }

    private fun fixture(name: String): RdfProjection {
        val bytes = Path("../vectors/v2/P3/$name.bin").readBytes()
        val wire = assertIs<WireInspection.VersionTwo>(inspectWire(bytes, retainDecoded = true))
        return projectRdf(wire.events)
    }
}
