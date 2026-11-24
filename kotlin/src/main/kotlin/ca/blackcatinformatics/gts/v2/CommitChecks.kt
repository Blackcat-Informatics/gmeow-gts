// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package ca.blackcatinformatics.gts.v2

import ca.blackcatinformatics.gts.ByteString
import ca.blackcatinformatics.gts.CborArray
import ca.blackcatinformatics.gts.CborBytes
import ca.blackcatinformatics.gts.cborArray
import ca.blackcatinformatics.gts.encode
import ca.blackcatinformatics.gts.uint

internal data class CommitClaim(val count: ULong, val root: CborField)

internal fun CborField.readCommitClaim(): CommitClaim {
    val fields = fields(2)
    return CommitClaim(fields.next().uint(), fields.next())
}

internal data class CommitMismatch(val position: Int, val expectedCount: Int, val countMismatch: Boolean, val rootMismatch: Boolean)

internal class CommitChecks(openerId: ByteString) {
    private val range = PrefixRange()
    private var position = 0
    var committedFrames = 0
        private set
    var tailFrames = 0
        private set
    var tailPosition: Int? = null
        private set
    var hasIndex = false
        private set

    init {
        range.push(openerId)
    }

    fun observe(item: WireEvent.Item): CommitMismatch? {
        position++
        val id = item.effectiveId
        if (item.frameType == 15uL) {
            hasIndex = true
        } else if (id != null) {
            if (tailFrames == 0) tailPosition = position
            tailFrames++
        }
        val mismatch = item.commitClaim?.let { check(it, item.link) }
        if (id != null) range.push(id)
        return mismatch
    }

    private fun check(claim: CommitClaim, link: FrameLink?): CommitMismatch? {
        val expected = range.leafCount - 1
        val countMismatch = claim.count != expected.toULong()
        val rootMismatch = !claim.root.matchesBytes(range.root())
        if (countMismatch || rootMismatch) return CommitMismatch(position, expected, countMismatch, rootMismatch)
        if (link == FrameLink.LINKED || link == FrameLink.HEALED) {
            committedFrames = range.leafCount
            tailFrames = 0
            tailPosition = null
        }
        return null
    }
}

// The frontier holds at most one peak per height, in descending height order.
internal class PrefixRange {
    private data class Peak(val height: Int, val digest: CborBytes)

    private val peaks = mutableListOf<Peak>()
    var leafCount = 0
        private set

    fun push(id: ByteString) {
        var height = 0
        var digest = hash("gts2 range leaf", cborArray(CborBytes(id)))
        while (peaks.lastOrNull()?.height == height) {
            val left = peaks.removeAt(peaks.lastIndex)
            digest = hash("gts2 range node", cborArray(left.digest, digest))
            height++
        }
        peaks.add(Peak(height, digest))
        leafCount++
    }

    fun root(): ByteArray =
        hash("gts2 range root", cborArray(uint(leafCount), CborArray(peaks.map { it.digest }))).value.bytes

    private fun hash(context: String, value: CborArray): CborBytes {
        val preimage = encode(value)
        return CborBytes(ByteString(contextHash(context) { update(preimage, 0, preimage.size) }))
    }
}
