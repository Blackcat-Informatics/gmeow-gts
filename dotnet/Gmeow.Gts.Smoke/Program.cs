// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

using System;
using System.IO;
using Gmeow.Gts;

internal static class Program
{
    private static int Main(string[] args)
    {
        try
        {
            if (args.Length != 1)
            {
                Console.Error.WriteLine("usage: Gmeow.Gts.Smoke vectors/01-minimal.gts");
                return 2;
            }

            if (Gts.AbiVersion != 1)
            {
                throw new InvalidOperationException($"Unexpected ABI version: {Gts.AbiVersion}");
            }
            if (string.IsNullOrEmpty(Gts.Version))
            {
                throw new InvalidOperationException("Empty library version");
            }

            byte[] input = File.ReadAllBytes(args[0]);

            ExpectContains("build metadata", Gts.BuildMetadataJson(), "\"schema\":\"gts-capi-build-v1\"");
            ExpectContains("capabilities", Gts.CapabilitiesJson(), "\"schema\":\"gts-capi-capabilities-v1\"");
            ExpectContains("read JSON", Gts.ReadJson(input), "\"schema\":\"gts-capi-read-v1\"");
            ExpectContains("verify JSON", Gts.VerifyJson(input), "\"schema\":\"gts-capi-verify-v1\"");

            string nquads = Gts.ToNQuads(input);
            ExpectContains("N-Quads", nquads, "\"Cat\"@en");

            byte[] roundTrip = Gts.FromNQuads(nquads);
            if (roundTrip.Length == 0)
            {
                throw new InvalidOperationException("Round-trip GTS output was empty");
            }

            try
            {
                _ = Gts.FromNQuads("<https://example/s> <https://example/p> .\n");
                throw new InvalidOperationException("Bad N-Quads did not fail");
            }
            catch (GtsException error) when (error.Status == GtsStatus.Parse)
            {
                if (string.IsNullOrEmpty(error.Code) || string.IsNullOrEmpty(error.Detail))
                {
                    throw new InvalidOperationException("Structured error did not include code and detail");
                }
            }

            string temp = Path.Combine(Path.GetTempPath(), "gts-dotnet-smoke-" + Guid.NewGuid().ToString("N"));
            try
            {
                string sourceDir = Path.Combine(temp, "src");
                string unpackDir = Path.Combine(temp, "unpack");
                Directory.CreateDirectory(sourceDir);
                File.WriteAllText(Path.Combine(sourceDir, "a.txt"), "hello\n");

                byte[] packed = Gts.FilesPack(new[] { sourceDir });
                ExpectContains("files diff", Gts.FilesDiffJson(packed, sourceDir), "\"clean\":true");
                ExpectContains("files unpack", Gts.FilesUnpack(packed, unpackDir), "\"ok\":true");
                if (!File.Exists(Path.Combine(unpackDir, "a.txt")))
                {
                    throw new InvalidOperationException("Unpacked file missing");
                }
            }
            finally
            {
                if (Directory.Exists(temp))
                {
                    Directory.Delete(temp, recursive: true);
                }
            }
        }
        catch (Exception error)
        {
            Console.Error.WriteLine(error);
            return 1;
        }

        return 0;
    }

    private static void ExpectContains(string label, string haystack, string needle)
    {
        if (!haystack.Contains(needle, StringComparison.Ordinal))
        {
            throw new InvalidOperationException($"{label} did not contain {needle}");
        }
    }
}
