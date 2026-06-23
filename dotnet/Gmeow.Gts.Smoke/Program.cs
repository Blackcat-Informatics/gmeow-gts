// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

using System;
using System.IO;
using System.Text.Json;
using Gmeow.Gts;

internal static class Program
{
    private static int Main(string[] args)
    {
        try
        {
            if (args.Length != 3)
            {
                Console.Error.WriteLine(
                    "usage: Gmeow.Gts.Smoke vectors/01-minimal.gts vectors/04-damaged-frame.gts vectors/28-empty-file.gts");
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
            byte[] damaged = File.ReadAllBytes(args[1]);
            byte[] empty = File.ReadAllBytes(args[2]);

            ExpectJsonPropertyEquals("build metadata", Gts.BuildMetadataJson(), "schema", "gts-capi-build-v1");
            ExpectJsonPropertyEquals("capabilities", Gts.CapabilitiesJson(), "schema", "gts-capi-capabilities-v1");
            string cleanRead = Gts.ReadJson(input);
            ExpectJsonPropertyEquals("dotnet clean-read read JSON", cleanRead, "schema", "gts-capi-read-v1");
            ExpectJsonPropertyEquals("dotnet clean-read read JSON", cleanRead, "clean", true);
            ExpectJsonPropertyEquals("verify JSON", Gts.VerifyJson(input), "schema", "gts-capi-verify-v1");

            string damagedRead = Gts.ReadJson(damaged);
            ExpectJsonPropertyEquals("dotnet damaged-diagnostic-read read JSON", damagedRead, "schema", "gts-capi-read-v1");
            ExpectJsonPropertyEquals("dotnet damaged-diagnostic-read read JSON", damagedRead, "clean", false);
            ExpectDiagnostic("dotnet damaged-diagnostic-read read JSON", damagedRead, "DamagedFrame");
            ExpectGtsException(
                "dotnet damaged-diagnostic-read to_nquads",
                () => Gts.ToNQuads(damaged),
                GtsStatus.Diagnostic);

            string emptyRead = Gts.ReadJson(empty);
            ExpectJsonPropertyEquals("dotnet empty-malformed-refusal read JSON", emptyRead, "schema", "gts-capi-read-v1");
            ExpectJsonPropertyEquals("dotnet empty-malformed-refusal read JSON", emptyRead, "clean", false);
            ExpectDiagnostic("dotnet empty-malformed-refusal read JSON", emptyRead, "EmptyFile");
            ExpectGtsException(
                "dotnet empty-malformed-refusal to_nquads",
                () => Gts.ToNQuads(empty),
                GtsStatus.Diagnostic);

            string nquads = Gts.ToNQuads(input);
            ExpectContains("N-Quads", nquads, "\"Cat\"@en");

            byte[] roundTrip = Gts.FromNQuads(nquads);
            if (roundTrip.Length == 0)
            {
                throw new InvalidOperationException("Round-trip GTS output was empty");
            }

            ExpectGtsException(
                "dotnet malformed-nquads-refusal from_nquads",
                () => Gts.FromNQuads(Environment.GetEnvironmentVariable("GTS_WRAPPER_BAD_NQUADS") ??
                                     "<https://example/s> <https://example/p> .\n"),
                GtsStatus.Parse);

            string temp = Path.Combine(Path.GetTempPath(), "gts-dotnet-smoke-" + Guid.NewGuid().ToString("N"));
            try
            {
                string sourceDir = Path.Combine(temp, "src");
                string unpackDir = Path.Combine(temp, "unpack");
                Directory.CreateDirectory(sourceDir);
                File.WriteAllText(Path.Combine(sourceDir, "a.txt"), "hello\n");

                byte[] packed = Gts.FilesPack(new[] { sourceDir });
                ExpectJsonPropertyEquals("files diff", Gts.FilesDiffJson(packed, sourceDir), "clean", true);
                ExpectJsonPropertyEquals("files unpack", Gts.FilesUnpack(packed, unpackDir), "ok", true);
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

    private static void ExpectJsonPropertyEquals(string label, string json, string propertyName, string expected)
    {
        using JsonDocument document = JsonDocument.Parse(json);
        if (!document.RootElement.TryGetProperty(propertyName, out JsonElement property))
        {
            throw new InvalidOperationException($"{label} missing JSON property {propertyName}");
        }
        string? actual = property.GetString();
        if (!string.Equals(actual, expected, StringComparison.Ordinal))
        {
            throw new InvalidOperationException($"{label} JSON property {propertyName} expected {expected}, got {actual}");
        }
    }

    private static void ExpectJsonPropertyEquals(string label, string json, string propertyName, bool expected)
    {
        using JsonDocument document = JsonDocument.Parse(json);
        if (!document.RootElement.TryGetProperty(propertyName, out JsonElement property))
        {
            throw new InvalidOperationException($"{label} missing JSON property {propertyName}");
        }
        bool actual = property.GetBoolean();
        if (actual != expected)
        {
            throw new InvalidOperationException($"{label} JSON property {propertyName} expected {expected}, got {actual}");
        }
    }

    private static void ExpectDiagnostic(string label, string json, string expectedCode)
    {
        using JsonDocument document = JsonDocument.Parse(json);
        if (!document.RootElement.TryGetProperty("diagnostics", out JsonElement diagnostics) ||
            diagnostics.ValueKind != JsonValueKind.Array)
        {
            throw new InvalidOperationException($"{label} missing diagnostics array");
        }
        foreach (JsonElement diagnostic in diagnostics.EnumerateArray())
        {
            if (diagnostic.TryGetProperty("code", out JsonElement code) &&
                string.Equals(code.GetString(), expectedCode, StringComparison.Ordinal))
            {
                return;
            }
        }
        throw new InvalidOperationException($"{label} missing diagnostic {expectedCode}");
    }

    private static void ExpectGtsException(string label, Func<object> action, GtsStatus expected)
    {
        try
        {
            _ = action();
        }
        catch (GtsException error) when (error.Status == expected)
        {
            if (string.IsNullOrEmpty(error.Code) || string.IsNullOrEmpty(error.Detail))
            {
                throw new InvalidOperationException($"{label} structured error did not include code and detail");
            }
            return;
        }
        throw new InvalidOperationException($"{label} did not fail with {expected}");
    }
}
