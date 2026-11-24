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
        assertEquals(WireEvent.Item(4, 4 + next.size, ItemKind.HEADER, opener = OpenerEnvelope(1uL, 1uL)), events[2])
        assertEquals(WireEvent.Item(4 + next.size, 5 + next.size, ItemKind.OPAQUE, "wrong_type"), events[3])
    }

    @Test
    fun retainsBoundariesAndDistinguishesTornAndUndelimitableItems() {
        val torn = assertIs<WireInspection.VersionTwo>(inspectWire(vector("torn-after-frames")))
        val events = torn.events.toList()
        assertEquals(WireEvent.Stop(353, StopReason.TORN_TAIL), events.last())
        val lastFrame = assertIs<WireEvent.Item>(events[events.lastIndex - 1])
        assertEquals(
            WireEvent.Item(
                239,
                353,
                ItemKind.FRAME,
                frameType = 14uL,
                link = FrameLink.LINKED,
                payload = PayloadObservation(PayloadState.VERIFIED),
            ),
            lastFrame,
        )

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

    @Test
    fun validatesOpenersAndKeepsInvalidSegmentsUnchecked() {
        val newer = assertIs<WireInspection.VersionTwo>(inspectWire(vector("header-hash-newer")))
        val events = newer.events.toList()
        assertEquals(OpenerEnvelope(2uL, 9uL), assertIs<WireEvent.Item>(events.first()).opener)
        assertEquals(ItemKind.UNCHECKED, assertIs<WireEvent.Item>(events.last()).kind)

        val wrongMarker =
            "8b050250".hexToByteArray() + ByteArray(16) +
                "010181018101008101805820".hexToByteArray() + ByteArray(32)
        val badResync = assertIs<WireInspection.VersionTwo>(inspectWire(wrongMarker))
        assertEquals("out_of_range", assertIs<WireEvent.Item>(badResync.events.single()).problem)

        val cases =
            listOf(
                Triple("header-hash-unregistered", ItemKind.HEADER, "unregistered_value"),
                Triple("marks-order-at-one-byte.head-before-order", ItemKind.RESYNC, "not_deterministic"),
                Triple("marks-order-at-one-byte.65th-and-below", ItemKind.RESYNC, "out_of_range"),
                Triple("marks-order-at-one-byte.order-before-deeper-head", ItemKind.RESYNC, "unordered_or_repeated"),
                Triple("marks-order-at-one-byte.text-first-below", ItemKind.RESYNC, "wrong_type"),
                Triple("resync-too-long.walked", ItemKind.RESYNC, "out_of_range"),
                Triple("resync-too-long.exactly-4096", ItemKind.RESYNC, null),
            )
        for ((name, kind, problem) in cases) {
            val wire = assertIs<WireInspection.VersionTwo>(inspectWire(vector(name)), name)
            val item = wire.events.filterIsInstance<WireEvent.Item>().last { it.kind == kind }
            assertEquals(problem, item.problem, name)
        }
    }

    @Test
    fun resyncLimitAppliesToTheNestedFieldStart() {
        val prefix = "8b050250fcf3eb32a2f0267dd16a53f09bff879202098101820282".hexToByteArray()
        for ((size, problem) in listOf(4061 to "wrong_type", 4062 to "out_of_range")) {
            val lineageDigest = byteArrayOf(0x59, (size shr 8).toByte(), size.toByte()) + ByteArray(size)
            val suffix = byteArrayOf(1, 0, 0x82.toByte(), 2, 0, 0x80.toByte(), 0x40)
            val wire = assertIs<WireInspection.VersionTwo>(inspectWire(prefix + lineageDigest + suffix))
            // The invalid past digest starts one byte before or at the cap.
            assertEquals(problem, assertIs<WireEvent.Item>(wire.events.single()).problem)
        }
    }

    @Test
    fun validatesFrameFieldsInWireOrder() {
        val cases =
            listOf(
                "frame-flags-order" to "forbidden_combination",
                "envelope-field-before-content-value" to "wrong_type",
                "frame-compressed-empty" to "forbidden_combination",
                "stored-check-short" to "wrong_length",
                "frame-unknown-content-map" to "wrong_type",
                "frame-unknown-content-bad-utf8" to "wrong_type",
                "frame-signatures-unordered" to "unordered_or_repeated",
                "frame-storage-tag-newer" to null,
            )
        for ((name, problem) in cases) {
            val wire = assertIs<WireInspection.VersionTwo>(inspectWire(vector(name)), name)
            val frame = assertIs<WireEvent.Item>(wire.events.last())
            assertEquals(ItemKind.FRAME, frame.kind, name)
            assertEquals(problem, frame.problem, name)
        }
        val path = Path("../vectors/v2/P5/commit-payload-nonempty.bin")
        val wire = assertIs<WireInspection.VersionTwo>(inspectWire(path.readBytes()))
        val commit = wire.events.filterIsInstance<WireEvent.Item>().single { it.frameType == 14uL }
        assertEquals("out_of_range", commit.problem)
    }

    @Test
    fun usesOnlyTheImmediateFrameAsAHealingWitness() {
        val witnessed = frames("link-healed-before-transparent")
        assertEquals(listOf(FrameLink.HEALED, FrameLink.LINKED), witnessed.map { it.link })
        assertEquals(16uL, witnessed[1].frameType)
        val interrupted = frames("link-heal-opaque-between")
        assertEquals(listOf(FrameLink.LINKED, FrameLink.BROKEN, FrameLink.BROKEN, FrameLink.LINKED), interrupted.map { it.link })

        val resync = frames("resync-identifier-field-differs")
        assertEquals(List(4) { FrameLink.LINKED }, resync.map { it.link })
    }

    @Test
    fun observesStoredAndDecodedChecksIndependently() {
        val mismatch = frames("stored-mismatch").single()
        assertEquals(FrameLink.LINKED, mismatch.link)
        assertEquals(false, mismatch.storedMatches)
        assertEquals(PayloadObservation(PayloadState.DAMAGED, "decode_failed"), mismatch.payload)

        val malformed = frames("payload-fields-ill-typed").first()
        assertEquals(PayloadState.DAMAGED, malformed.payload?.state)

        val cases =
            listOf(
                "zstd-ok" to PayloadObservation(PayloadState.VERIFIED),
                "zstd-header-short" to PayloadObservation(PayloadState.DAMAGED, "codec_constraint"),
                "zstd-trailing-byte" to PayloadObservation(PayloadState.DAMAGED, "decode_failed"),
                "zstd-two-frames" to PayloadObservation(PayloadState.DAMAGED, "decode_failed"),
                "zstd-output-short" to PayloadObservation(PayloadState.DAMAGED, "decode_failed"),
            )
        for ((name, expected) in cases) {
            val frame = frames(name).single()
            assertEquals(true, frame.storedMatches, name)
            assertEquals(expected, frame.payload, name)
        }
    }

    @Test
    fun latchesTheTotalDecodedRefusalAcrossLaterFrames() {
        val data = vector("zstd-ok")
        val start = itemEnd(data, 0)
        val frame = readFrame(CborField(data, start, itemEnd(data, start)), 1uL)
        val budget = PayloadBudget(readCapacity = 599uL)
        assertEquals(PayloadObservation(PayloadState.VERIFIED), budget.check(frame.payload))
        val refused = PayloadObservation(PayloadState.UNCHECKED, "total_decoded", 600uL)
        assertEquals(refused, budget.check(frame.payload))
        val empty = frame.payload.copy(length = 0uL)
        assertEquals(refused.copy(count = 300uL), budget.check(empty))
    }

    @Test
    fun checksClosingClaimsAgainstTheObservedItemBoundaries() {
        val cases =
            listOf(
                Triple("offsets-counts-opaque", null, null),
                Triple("offsets-count-short", null, ClosingClaimFailure.INDEX_OFFSETS),
                Triple("offsets-count-long", null, ClosingClaimFailure.INDEX_OFFSETS),
                Triple("offsets-length-differs", null, ClosingClaimFailure.INDEX_OFFSETS),
                Triple("locator-last-index.first", null, ClosingClaimFailure.INDEX_LOCATOR),
                Triple("locator-last-index.second", null, null),
                Triple("locator-index-type-long-form", null, ClosingClaimFailure.INDEX_LOCATOR),
                Triple("locator-short-form", "not_deterministic", null),
            )
        for ((name, problem, failure) in cases) {
            val wire = assertIs<WireInspection.VersionTwo>(inspectWire(vector(name)), name)
            val closing = assertIs<WireEvent.Item>(wire.events.last(), name)
            assertEquals(problem, closing.problem, name)
            assertEquals(failure, closing.closingClaimFailure, name)
        }
    }

    @Test
    fun checksPlacementBeforeClaimsAndResetsAtTheNextOpener() {
        val opener = vector("gen-record-two")
        val offsets = "8204810b".hexToByteArray()
        val locator = "820300".hexToByteArray()
        val torn = "82".hexToByteArray()
        val data = opener + offsets + locator + locator + opener + offsets + torn
        val wire = assertIs<WireInspection.VersionTwo>(inspectWire(data))
        val events = wire.events.toList()
        val closing = events.filterIsInstance<WireEvent.Item>().filter { it.kind in setOf(ItemKind.OFFSETS, ItemKind.LOCATOR) }
        assertEquals(List(3) { "unordered_or_repeated" } + listOf(null), closing.map { it.problem })
        assertEquals(List(4) { null }, closing.map { it.closingClaimFailure })
        assertEquals(WireEvent.Stop(data.size - torn.size, StopReason.TORN_TAIL), events.last())
    }

    private fun frames(name: String): List<WireEvent.Item> =
        assertIs<WireInspection.VersionTwo>(inspectWire(vector(name)), name).events
            .filterIsInstance<WireEvent.Item>()
            .filter { it.kind == ItemKind.FRAME }
            .toList()
}
