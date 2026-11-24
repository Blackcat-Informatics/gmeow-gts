// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package ca.blackcatinformatics.gts.cli

import ca.blackcatinformatics.gts.v2.RdfProjectionFailure
import ca.blackcatinformatics.gts.v2.WireEvent
import ca.blackcatinformatics.gts.v2.WireInspection
import ca.blackcatinformatics.gts.v2.inspectWire
import ca.blackcatinformatics.gts.v2.projectRdf
import ca.blackcatinformatics.gts.v2.renderRdfProjection

internal fun foldV2(path: String, data: ByteArray): Boolean? {
    val problem =
        when (val wire = inspectWire(data, retainDecoded = true)) {
            WireInspection.Legacy -> return null
            WireInspection.Empty -> "empty input"
            WireInspection.Unknown -> "input does not identify a GTS generation"
            is WireInspection.Unsupported -> "unsupported GTS generation ${wire.generation}"
            is WireEvent.Stop -> "framing stopped at byte ${wire.offset}: ${wire.reason.name.lowercase()}"
            is WireInspection.VersionTwo -> {
                try {
                    writeUtf8(renderRdfProjection(projectRdf(wire.events)))
                    return true
                } catch (error: RdfProjectionFailure) {
                    error.message
                }
            }
        }
    System.err.println("gts fold: $path: $problem")
    return false
}


private fun writeUtf8(text: String) {
    val writer = System.out.writer(Charsets.UTF_8)
    var offset = 0
    while (offset < text.length) {
        val length = minOf(8192, text.length - offset)
        writer.write(text, offset, length)
        offset += length
    }
    writer.flush()
}
