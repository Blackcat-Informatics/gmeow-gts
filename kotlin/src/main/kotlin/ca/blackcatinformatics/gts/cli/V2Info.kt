// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package ca.blackcatinformatics.gts.cli

import ca.blackcatinformatics.gts.v2.ItemKind
import ca.blackcatinformatics.gts.v2.WireEvent
import ca.blackcatinformatics.gts.v2.WireInspection
import ca.blackcatinformatics.gts.v2.inspectWire

internal fun printV2Info(path: String, data: ByteArray): Boolean? =
    when (val wire = inspectWire(data)) {
        WireInspection.Legacy -> null
        WireInspection.Empty -> {
            println("$path: empty input, 0 segments")
            false
        }
        WireInspection.Unknown -> {
            System.err.println("gts info: $path: input does not identify a GTS generation")
            false
        }
        is WireInspection.Unsupported -> {
            System.err.println("gts info: $path: unsupported GTS generation ${wire.generation}")
            false
        }
        is WireEvent.Stop -> {
            println("$path: framing stopped at byte ${wire.offset}: ${wire.reason.name.lowercase()}")
            false
        }
        is WireInspection.VersionTwo -> printV2Ledger(path, wire)
    }

private fun printV2Ledger(path: String, wire: WireInspection.VersionTwo): Boolean {
    println("$path: GTS v2 framing ledger (item boundaries and envelopes)")
    var ordinal = 0
    var clean = true
    for (event in wire.events) {
        when (event) {
            is WireEvent.Stop -> {
                println("  framing stopped at byte ${event.offset}: ${event.reason.name.lowercase()}")
                return false
            }
            is WireEvent.Item -> {
                val prefix =
                    if (event.kind == ItemKind.HEADER || event.kind == ItemKind.RESYNC) {
                        "  segment ${ordinal++}: "
                    } else {
                        "    "
                    }
                val type = event.frameType?.let { " type $it" }.orEmpty()
                val parameters = event.opener?.let { ", registry ${it.registry}, hash algorithm ${it.hashAlgorithm}" }.orEmpty()
                val problem =
                    event.problem?.let { if (event.frameType == 16uL) ": parity_damaged ($it)" else ": $it" }.orEmpty()
                println("$prefix${event.kind.name.lowercase()}$type bytes [${event.offset}, ${event.end})$parameters$problem")
                if (event.problem != null) clean = false
                if (event.opener?.registry?.let { it > 1uL } == true) {
                    println("    unsupported registry version ${event.opener.registry}")
                    clean = false
                }
            }
        }
    }
    return clean
}
