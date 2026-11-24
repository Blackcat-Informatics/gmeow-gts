// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package ca.blackcatinformatics.gts.cli

import java.io.ByteArrayOutputStream
import java.io.PrintStream
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class KotlinCliI18nTest {
    @Test
    fun localizesHelpAndUnknownCommand() {
        val cases =
            listOf(
                Triple("nonsense", "usage: gts", "unknown command"),
                Triple("fr_CA", "utilisation: gts", "commande inconnue"),
                Triple("zh_CN", "用法: gts", "未知命令"),
            )

        for ((locale, usageMarker, errorMarker) in cases) {
            val help = captureCli(locale, "help")
            assertEquals(0, help.code)
            assertTrue(help.stdout.contains(usageMarker), help.stdout)
            assertTrue(help.stdout.contains("from-nq"), help.stdout)

            val bad = captureCli(locale, "not-a-gts-command")
            assertEquals(2, bad.code)
            assertTrue(bad.stderr.contains(errorMarker), bad.stderr)
            assertTrue(bad.stderr.contains("not-a-gts-command"), bad.stderr)
        }
    }

    @Test
    fun reportsV2FramingAndStillInspectsLegacyInputs() {
        val result =
            captureCli(
                "en",
                "info",
                "../vectors/v2/P1/gen-record-three.bin",
                "../vectors/v2/P1/torn-after-frames.bin",
                "../vectors/01-minimal.gts",
                "../vectors/v2/P1/gen-record-two.bin",
            )
        assertEquals(1, result.code)
        assertTrue(result.stderr.contains("unsupported GTS generation 3"), result.stderr)
        assertTrue(result.stdout.contains("framing stopped at byte 353: torn_tail"), result.stdout)
        assertTrue(result.stdout.contains("01-minimal.gts: 1 segment(s)"), result.stdout)
        assertTrue(result.stdout.contains("gen-record-two.bin: GTS v2 framing ledger"), result.stdout)
        assertTrue(result.stdout.contains("segment 0: header bytes [0, 11)"), result.stdout)
    }

    @Test
    fun reportsMalformedParityWithoutLosingTheFrameKind() {
        val result = captureCli("en", "info", "../vectors/v2/P1/parity-signature.bin")
        assertEquals(1, result.code)
        assertTrue(result.stdout.contains("frame type 16"), result.stdout)
        assertTrue(result.stdout.contains("parity_damaged (forbidden_combination)"), result.stdout)
        val brokenLink = captureCli("en", "info", "../vectors/v2/P1/parity-damaged-link.bin")
        assertEquals(1, brokenLink.code)
        assertTrue(brokenLink.stdout.contains("parity_damaged"), brokenLink.stdout)
        assertTrue(brokenLink.stdout.contains("link broken"), brokenLink.stdout)
    }

    @Test
    fun exitsOnAnIntegrityFindingAndPrintsTheIndependentChecks() {
        val clean = captureCli("en", "info", "../vectors/v2/P1/zstd-ok.bin")
        assertEquals(0, clean.code)
        assertTrue(clean.stdout.contains("link linked, stored check matches, payload verified"), clean.stdout)
        val mismatch = captureCli("en", "info", "../vectors/v2/P1/stored-mismatch.bin")
        assertEquals(1, mismatch.code)
        assertTrue(mismatch.stdout.contains("link linked, stored check mismatch, payload damaged (decode_failed)"), mismatch.stdout)
    }

    @Test
    fun failsInfoWhenAnOffsetsClaimDisagreesWithTheObservedLengths() {
        assertEquals(0, captureCli("en", "info", "../vectors/v2/P1/offsets-holds.bin").code)
        val failed = captureCli("en", "info", "../vectors/v2/P1/offsets-count-short.bin")
        assertEquals(1, failed.code)
        assertTrue(failed.stdout.contains("index_claim_failed (index_offsets)"), failed.stdout)
    }

    @Test
    fun verifiesCommitRangesAndPreservesLegacyDispatch() {
        val result =
            captureCli(
                "en",
                "verify",
                "../vectors/01-minimal.gts",
                "../vectors/v2/P5/commit-valid.bin",
                "../vectors/v2/P5/range-transparent.with-parity.bin",
                "../vectors/v2/P1/resync-range-leaf0.stored-identifier-differs.bin",
            )
        assertEquals(0, result.code, result.stderr)
        assertTrue(result.stdout.contains("committed_frames 3, uncommitted_tail 1 (item "), result.stdout)
        assertTrue(result.stdout.contains("index payload claims unchecked"), result.stdout)
        assertTrue(result.stdout.contains("segment 1: committed_frames "), result.stdout)
        assertTrue(result.stdout.trimStart().startsWith("{"), result.stdout)
    }

    @Test
    fun reportsCommitClaimsSeparatelyAndKeepsTheObservedPrefixAtAStop() {
        val cases =
            listOf(
                "P5/commit-count-mismatch.bin" to "commit_count_mismatch (expected 2)",
                "P5/commit-root-mismatch.bin" to "commit_root_mismatch",
                "P5/commit-payload-nonempty.bin" to "out_of_range",
                "P5/range-healed-leaf.bin" to "link healed",
                "P1/torn-after-frames.bin" to "framing stopped at byte 353: torn_tail",
            )
        for ((name, finding) in cases) {
            val result = captureCli("en", "verify", "../vectors/v2/$name")
            assertEquals(1, result.code, name)
            assertTrue(result.stderr.contains(finding), result.stderr)
            if (name.contains("range-healed")) {
                assertTrue(result.stdout.contains("committed_frames 3"), result.stdout)
            }
            if (name.contains("torn-after")) {
                assertTrue(result.stdout.contains("no_commit, uncommitted_tail 3 (item 1)"), result.stdout)
            }
        }
    }

    @Test
    fun rejectsV2KeyAndUnknownGeneration() {
        val result =
            captureCli(
                "en",
                "verify",
                "--key",
                "example:${"00".repeat(32)}",
                "../vectors/v2/P5/commit-valid.bin",
                "../vectors/v2/P1/gen-record-three.bin",
            )
        assertEquals(1, result.code)
        assertTrue(result.stderr.contains("--key is unsupported for GTS v2"), result.stderr)
        assertTrue(result.stderr.contains("unsupported GTS generation 3"), result.stderr)
    }

    private data class CliCapture(
        val code: Int,
        val stdout: String,
        val stderr: String,
    )

    private fun captureCli(locale: String, vararg args: String): CliCapture {
        val oldOut = System.out
        val oldErr = System.err
        val stdout = ByteArrayOutputStream()
        val stderr = ByteArrayOutputStream()
        try {
            System.setOut(PrintStream(stdout, true, Charsets.UTF_8))
            System.setErr(PrintStream(stderr, true, Charsets.UTF_8))
            val code = runCliForLocale(args.toList().toTypedArray(), locale)
            return CliCapture(
                code,
                stdout.toString(Charsets.UTF_8),
                stderr.toString(Charsets.UTF_8),
            )
        } finally {
            System.setOut(oldOut)
            System.setErr(oldErr)
        }
    }
}
