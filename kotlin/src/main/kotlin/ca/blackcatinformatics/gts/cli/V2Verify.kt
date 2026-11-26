// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package ca.blackcatinformatics.gts.cli

import ca.blackcatinformatics.gts.v2.CommitChecks
import ca.blackcatinformatics.gts.v2.FrameLink
import ca.blackcatinformatics.gts.v2.ItemKind
import ca.blackcatinformatics.gts.v2.PayloadState
import ca.blackcatinformatics.gts.v2.WireEvent
import ca.blackcatinformatics.gts.v2.WireInspection
import ca.blackcatinformatics.gts.v2.inspectWire

/** Null routes a confirmed archived generation to its verifier. */
internal fun verifyV2(path: String, data: ByteArray, hasKeys: Boolean): Boolean? {
    val problem =
        when (val wire = inspectWire(data)) {
            WireInspection.Legacy -> return null
            WireInspection.Empty -> "empty input"
            WireInspection.Unknown -> "input does not identify a GTS generation"
            is WireInspection.Unsupported -> "unsupported GTS generation ${wire.generation}"
            is WireEvent.Stop -> wire.describeStop()
            is WireInspection.VersionTwo -> {
                if (!hasKeys) return verifyV2Records(path, wire.events)
                "--key is unsupported for GTS v2"
            }
        }
    System.err.println("gts verify: $path: $problem")
    return false
}

private fun verifyV2Records(path: String, events: Sequence<WireEvent>): Boolean {
    println("$path: GTS v2 wire and commit integrity")
    var clean = true
    var ordinal = -1
    var checks: CommitChecks? = null
    fun report(message: String) {
        System.err.println("gts verify: $path: $message")
        clean = false
    }

    for (event in events) {
        if (event is WireEvent.Stop) {
            report(event.describeStop())
            break
        }
        val item = event as WireEvent.Item
        val problem = item.primaryWireFinding()
        val offset = item.offset
        if (item.kind == ItemKind.HEADER || item.kind == ItemKind.RESYNC) {
            checks?.printSummary(ordinal)
            ordinal++
            val openerId = item.effectiveId
            val registry = item.opener?.registry
            checks = null
            when {
                problem != null -> report("byte $offset: $problem")
                registry != 1uL || openerId == null -> report("byte $offset: unsupported registry version $registry")
                else -> checks = CommitChecks(openerId)
            }
            continue
        }
        if (item.kind == ItemKind.DAMAGED) {
            report("framing stopped at byte $offset: $problem")
            checks?.observe(item)
            continue
        }
        val segment = checks ?: continue
        item.reportWireFailures { report("byte $offset: $it") }
        val mismatch = segment.observe(item)
        if (mismatch != null) {
            val location = "byte $offset, item ${mismatch.position}"
            if (mismatch.countMismatch) {
                report("$location: commit_count_mismatch (expected ${mismatch.expectedCount})")
            }
            if (mismatch.rootMismatch) report("$location: commit_root_mismatch")
        }
    }
    checks?.printSummary(ordinal)
    return clean
}

private fun WireEvent.Stop.describeStop(): String = "framing stopped at byte $offset: ${reason.name.lowercase()}"

private fun WireEvent.Item.reportWireFailures(report: (String) -> Unit) {
    val finding = primaryWireFinding()
    if (finding != null) report(finding)
    if (kind != ItemKind.FRAME || (frameType == 16uL && finding != null)) return
    val state = link
    if (state != FrameLink.LINKED) report("link ${state?.name?.lowercase() ?: "unchecked"}")
    if (storedMatches == false) report("stored check mismatch")
    if (payload?.state != PayloadState.VERIFIED) {
        val reason = payload?.reason ?: payload?.state?.name?.lowercase() ?: "unchecked"
        report("payload $reason")
    }
}

private fun CommitChecks.printSummary(ordinal: Int) {
    val prefix = if (committedFrames == 0) "no_commit" else "committed_frames $committedFrames"
    val tail = tailPosition?.let { ", uncommitted_tail $tailFrames (item $it)" }.orEmpty()
    val index = if (hasIndex) ", index payload claims unchecked" else ""
    println("  segment $ordinal: $prefix$tail$index")
}
