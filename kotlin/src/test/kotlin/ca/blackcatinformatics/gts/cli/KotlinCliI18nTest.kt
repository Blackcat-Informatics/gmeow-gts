// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

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
