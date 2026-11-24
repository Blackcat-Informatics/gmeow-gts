// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package ca.blackcatinformatics.gts.v2

import ca.blackcatinformatics.gts.ByteString
import ca.blackcatinformatics.gts.encode
import ca.blackcatinformatics.gts.uint
import kotlin.io.path.Path
import kotlin.io.path.readBytes
import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertIs

class SegmentWriterTest {
    @Test
    fun reproducesPublishedPlainFramesAndCommits() {
        val fixtures =
            listOf(
                "P5/commit-valid",
                "P5/range-transparent.no-parity",
                "P5/range-transparent.with-parity",
                "P3/state-prefix-fold",
            )
        for (name in fixtures) {
            val data = Path("../vectors/v2/$name.bin").readBytes()
            val wire = assertIs<WireInspection.VersionTwo>(inspectWire(data), name)
            var writer: V2SegmentWriter? = null
            var start = 0
            for (event in wire.events) {
                val item = assertIs<WireEvent.Item>(event, name)
                val field = CborField(data, item.offset, item.end)
                if (item.kind == ItemKind.HEADER) {
                    writer?.let { assertContentEquals(data.copyOfRange(start, item.offset), it.toBytes(), name) }
                    writer = writerFor(field)
                    start = item.offset
                } else {
                    val segment = checkNotNull(writer)
                    assertEquals(ItemKind.FRAME, item.kind, name)
                    assertEquals(PayloadState.VERIFIED, item.payload?.state, name)
                    val frame = readFrame(field, 1uL)
                    val before = segment.head
                    val id =
                        if (frame.type == 14uL) {
                            segment.commit()
                        } else {
                            val stored = frame.payload.stored
                            segment.append(
                                checkNotNull(frame.type).toInt(),
                                frame.content.encoded(),
                                data.copyOfRange(stored.bytes(), stored.end),
                            )
                        }
                    assertContentEquals(frame.storedId, id, "$name byte ${item.offset}")
                    if (frame.transparent) assertContentEquals(before, segment.head, name)
                }
            }
            assertContentEquals(data.copyOfRange(start, data.size), checkNotNull(writer).toBytes(), name)
        }
    }

    @Test
    fun encodesFullWidthLineageAndPreservesTheSignedCborGuard() {
        val writer = V2SegmentWriter(6, true, V2Lineage(ByteString(ByteArray(32)), ULong.MAX_VALUE))
        val data = writer.toBytes()
        val item = assertIs<WireInspection.VersionTwo>(inspectWire(data)).events.single()
        assertIs<WireEvent.Item>(item)
        assertEquals(null, item.problem)
        val lineage = CborField(data, 0, data.size).fields(7)
        repeat(6) { lineage.next() }
        lineage.next().option {
            val record = it.fields(2)
            record.next().bytes(32)
            assertEquals(ULong.MAX_VALUE, record.next().uint())
        }
        assertFailsWith<IllegalArgumentException> { encode(uint(-1)) }
    }

    @Test
    fun leavesThePrefixIntactAfterRejectedAppendsAndReturnedValueMutations() {
        val writer = V2SegmentWriter()
        val before = writer.toBytes()
        val head = writer.head
        val empty = ByteArray(0)
        assertFailsWith<IllegalArgumentException> { writer.append(14, "80".hexToByteArray(), empty) }
        assertFailsWith<IllegalArgumentException> { writer.append(2, "8102".hexToByteArray(), empty) }
        assertFailsWith<IllegalArgumentException> { writer.append(2, "8202ff".hexToByteArray(), empty) }
        assertFailsWith<IllegalArgumentException> { writer.append(2, "82020100".hexToByteArray(), empty) }
        assertFailsWith<IllegalArgumentException> { writer.append(9, ("825820" + "00".repeat(32) + "820261ff").hexToByteArray(), empty) }
        assertFailsWith<IllegalArgumentException> { writer.append(8, "815820".hexToByteArray() + ByteArray(32), empty) }
        writer.head.fill(0)
        writer.toBytes().fill(0)
        assertContentEquals(before, writer.toBytes())
        assertContentEquals(head, writer.head)
        val id = writer.commit()
        val expected = V2SegmentWriter()
        assertContentEquals(expected.commit(), id)
        id.fill(0)
        assertContentEquals(expected.head, writer.head)
        assertContentEquals(expected.toBytes(), writer.toBytes())
    }

    private fun writerFor(header: CborField): V2SegmentWriter {
        val fields = header.fields(7)
        repeat(4) { fields.next() }
        var profile: Int? = null
        fields.next().option { profile = it.uint().toInt() }
        var canonical = false
        fields.next().option { canonical = it.uint() == 1uL }
        var lineage: V2Lineage? = null
        fields.next().option {
            val pair = it.fields(2)
            val digest = pair.next()
            val previousHead = ByteString(digest.data.copyOfRange(digest.bytes(32), digest.end))
            lineage = V2Lineage(previousHead, pair.next().uint())
        }
        return V2SegmentWriter(profile, canonical, lineage)
    }

    private fun CborField.encoded(): ByteArray = data.copyOfRange(start, end)
}
