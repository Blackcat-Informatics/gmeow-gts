// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package ca.blackcatinformatics.gts.v2

import kotlin.io.path.Path
import kotlin.io.path.readBytes
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertIs

class WireInspectionTest {
    private fun vector(name: String): ByteArray = Path("../vectors/v2/P1/$name.bin").readBytes()

    @Test
    fun dispatchesOnTheCompleteFirstItemAndKeepsUnsignedGeneration() {
        assertEquals(WireInspection.Legacy, inspectWire(vector("gen-first-generation-tagged")))
        assertEquals(WireInspection.Unknown, inspectWire(vector("gen-first-generation-v-twice")))
        assertEquals(WireInspection.Unknown, inspectWire(vector("gen-record-one")))
        assertEquals(WireInspection.Unsupported(2uL), inspectWire(vector("gen-first-generation-two")))
        val unsigned = byteArrayOf(0x82.toByte(), 1, 0x1b) + ByteArray(8) { 0xff.toByte() }
        assertEquals(WireInspection.Unsupported(ULong.MAX_VALUE), inspectWire(unsigned))
        val indefiniteLegacy = byteArrayOf(0xbf.toByte(), 0x7f, 0x60, 0x61, 0x76, 0xff.toByte(), 1, 0xff.toByte())
        assertEquals(WireInspection.Legacy, inspectWire(indefiniteLegacy))

        val nonShortest = assertIs<WireInspection.VersionTwo>(inspectWire(vector("header-long-form-first-item")))
        assertEquals("not_deterministic", assertIs<WireEvent.Item>(nonShortest.events.first()).problem)
    }

    @Test
    fun leavesItemsUncheckedBetweenAnUnreadableOpenerAndTheNextHeader() {
        val damaged = byteArrayOf(0x82.toByte(), 1, 2, 0)
        val next = vector("gen-record-two")
        val result = assertIs<WireInspection.VersionTwo>(inspectWire(damaged + next + byteArrayOf(0)))
        val events = result.events.toList()
        assertEquals(WireEvent.Item(0, 3, ItemKind.HEADER, "wrong_length"), events[0])
        assertEquals(WireEvent.Item(3, 4, ItemKind.UNCHECKED), events[1])
        assertEquals(WireEvent.Item(4, 4 + next.size, ItemKind.HEADER), events[2])
        assertEquals(WireEvent.Item(4 + next.size, 5 + next.size, ItemKind.OPAQUE, "wrong_type"), events[3])
    }

    @Test
    fun retainsBoundariesAndDistinguishesTornAndUndelimitableItems() {
        val torn = assertIs<WireInspection.VersionTwo>(inspectWire(vector("torn-after-frames")))
        val events = torn.events.toList()
        assertEquals(WireEvent.Stop(353, StopReason.TORN_TAIL), events.last())
        assertEquals(WireEvent.Item(239, 353, ItemKind.FRAME), events[events.lastIndex - 1])

        val header = vector("gen-record-two")
        val cases =
            listOf(
                byteArrayOf(0xbf.toByte(), 1, 0xff.toByte()) to StopReason.UNDELIMITABLE_INPUT,
                byteArrayOf(0x5f, 0x7a) to StopReason.UNDELIMITABLE_INPUT,
                byteArrayOf(0xf8.toByte(), 0) to StopReason.UNDELIMITABLE_INPUT,
                byteArrayOf(0xf8.toByte()) to StopReason.TORN_TAIL,
                byteArrayOf(0x5b) + ByteArray(8) { 0xff.toByte() } to StopReason.TORN_TAIL,
            )
        for ((suffix, reason) in cases) {
            val wire = assertIs<WireInspection.VersionTwo>(inspectWire(header + suffix))
            assertEquals(WireEvent.Stop(header.size, reason), wire.events.last())
        }
        val chunked = byteArrayOf(0x5f, 0x41, 1, 0xff.toByte())
        val complete = assertIs<WireInspection.VersionTwo>(inspectWire(header + chunked + header))
        val last = assertIs<WireEvent.Item>(complete.events.last())
        assertEquals(ItemKind.HEADER, last.kind)
        assertEquals(header.size + chunked.size, last.offset)
    }
}
