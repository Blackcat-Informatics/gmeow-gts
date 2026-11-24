// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package ca.blackcatinformatics.gts.v2

import ca.blackcatinformatics.gts.NQuadsParseException
import ca.blackcatinformatics.gts.fromNQuadsV2
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertIs

class RdfImportTest {
    @Test
    fun writesDistinctQuadsWithNestedObjectsAndAnRdfReifiesPredicate() {
        val data = fromNQuadsV2(
            """
            VERSION "1.2"
            <https://ex/s> <https://ex/p> "one" .
            <https://ex/s> <https://ex/p> "one"^^<http://www.w3.org/2001/XMLSchema#string> . # same quad
            _:é <http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies> <<( <https://ex/\u0073> <https://ex/p> <<( _:é <https://ex/p> "été"@FR--ltr )>> )>> <https://ex/g> .
            """.trimIndent(),
        )
        val wire = assertIs<WireInspection.VersionTwo>(inspectWire(data))
        val items = wire.events.map { assertIs<WireEvent.Item>(it) }.toList()
        val frames = items.drop(1)
        assertEquals(listOf(1uL, 2uL, 14uL), frames.map { it.frameType })
        assertEquals(listOf(FrameLink.LINKED, FrameLink.LINKED, FrameLink.LINKED), frames.map { it.link })
        val rowItem = frames[1]
        val rows = readFrame(CborField(data, rowItem.offset, rowItem.end), 1uL).content.fields(2)
        assertEquals(2uL, rows.next().uint())
        assertEquals(2uL, rows.next().uint())
        val commit = CommitChecks(checkNotNull(items.first().effectiveId))
        for (frame in frames) assertEquals(null, commit.observe(frame))
        assertEquals(3, commit.committedFrames)
    }

    @Test
    fun rejectsInvalidRdfBeforeWriting() {
        val bad = listOf(
            """<<( <urn:s> <urn:p> "o" )>> <urn:p> <urn:o> .""",
            """<urn:s> <urn:p> <<( "s" <urn:p> <urn:o> )>> .""",
            """<relative> <urn:p> <urn:o> .""",
            """<urn:s> <urn:p> "bad\uD800" .""",
            """<urn:s> <urn:p> "bad"@en- .""",
            """<urn:s> <urn:p> "bad"^^<http://www.w3.org/1999/02/22-rdf-syntax-ns#langString> .""",
        )
        for (line in bad) assertFailsWith<NQuadsParseException>(line) { fromNQuadsV2(line) }
    }
}
