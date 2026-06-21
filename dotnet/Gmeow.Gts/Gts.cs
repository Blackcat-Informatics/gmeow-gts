// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;

namespace Gmeow.Gts;

public enum GtsStatus
{
    Ok = 0,
    InvalidArgument = 1,
    Io = 2,
    Parse = 3,
    Diagnostic = 4,
    Internal = 5,
    Panic = 6,
}

[Flags]
public enum GtsUnpackFlags : uint
{
    None = 0,
    IncludeSuppressed = 1u << 0,
    AllowSymlinks = 1u << 1,
    AllowSpecial = 1u << 2,
    SameOwner = 1u << 3,
    PreserveSetid = 1u << 4,
}

public sealed class GtsException : Exception
{
    public GtsException(string operation, GtsStatus status, string code, string detail)
        : base(Format(operation, status, code, detail))
    {
        Operation = operation;
        Status = status;
        Code = code;
        Detail = detail;
    }

    public string Operation { get; }
    public GtsStatus Status { get; }
    public string Code { get; }
    public string Detail { get; }

    private static string Format(string operation, GtsStatus status, string code, string detail)
    {
        var message = $"{operation} failed with {status}";
        if (!string.IsNullOrEmpty(code))
        {
            message += $" ({code})";
        }
        if (!string.IsNullOrEmpty(detail))
        {
            message += $": {detail}";
        }
        return message;
    }
}

public static class Gts
{
    public static uint AbiVersion => NativeMethods.gts_abi_version();

    public static string Version => CopyUtf8(NativeMethods.gts_version());

    public static string BuildMetadataJson()
    {
        return CallString("gts_build_metadata_json", NativeMethods.gts_build_metadata_json);
    }

    public static string CapabilitiesJson()
    {
        return CallString("gts_capabilities_json", NativeMethods.gts_capabilities_json);
    }

    public static string ReadJson(byte[] data)
    {
        ArgumentNullException.ThrowIfNull(data);
        return CallString("gts_read_json", (out NativeMethods.GtsBuffer buffer, out IntPtr error) =>
            NativeMethods.gts_read_json(data, ToUIntPtr(data.Length), out buffer, out error));
    }

    public static string VerifyJson(byte[] data)
    {
        ArgumentNullException.ThrowIfNull(data);
        return CallString("gts_verify_json", (out NativeMethods.GtsBuffer buffer, out IntPtr error) =>
            NativeMethods.gts_verify_json(data, ToUIntPtr(data.Length), out buffer, out error));
    }

    public static string ToNQuads(byte[] data)
    {
        ArgumentNullException.ThrowIfNull(data);
        return CallString("gts_to_nquads", (out NativeMethods.GtsBuffer buffer, out IntPtr error) =>
            NativeMethods.gts_to_nquads(data, ToUIntPtr(data.Length), out buffer, out error));
    }

    public static byte[] FromNQuads(string text)
    {
        ArgumentNullException.ThrowIfNull(text);
        byte[] textBytes = Encoding.UTF8.GetBytes(text);
        return CallBytes("gts_from_nquads", (out NativeMethods.GtsBuffer buffer, out IntPtr error) =>
            NativeMethods.gts_from_nquads(textBytes, ToUIntPtr(textBytes.Length), out buffer, out error));
    }

    public static byte[] FilesPack(IEnumerable<string> paths)
    {
        ArgumentNullException.ThrowIfNull(paths);
        var nativePaths = new List<IntPtr>();
        try
        {
            foreach (string path in paths)
            {
                ArgumentNullException.ThrowIfNull(path);
                nativePaths.Add(Marshal.StringToCoTaskMemUTF8(path));
            }
            IntPtr[] rawPaths = nativePaths.ToArray();
            return CallBytes("gts_files_pack", (out NativeMethods.GtsBuffer buffer, out IntPtr error) =>
                NativeMethods.gts_files_pack(rawPaths, ToUIntPtr(rawPaths.Length), out buffer, out error));
        }
        finally
        {
            foreach (IntPtr path in nativePaths)
            {
                Marshal.FreeCoTaskMem(path);
            }
        }
    }

    public static string FilesUnpack(byte[] data, string destination, GtsUnpackFlags flags = GtsUnpackFlags.None)
    {
        ArgumentNullException.ThrowIfNull(data);
        ArgumentNullException.ThrowIfNull(destination);
        IntPtr nativeDestination = Marshal.StringToCoTaskMemUTF8(destination);
        try
        {
            return CallString("gts_files_unpack", (out NativeMethods.GtsBuffer buffer, out IntPtr error) =>
                NativeMethods.gts_files_unpack(
                    data,
                    ToUIntPtr(data.Length),
                    nativeDestination,
                    (uint)flags,
                    out buffer,
                    out error));
        }
        finally
        {
            Marshal.FreeCoTaskMem(nativeDestination);
        }
    }

    public static string FilesDiffJson(byte[] data, string directory)
    {
        ArgumentNullException.ThrowIfNull(data);
        ArgumentNullException.ThrowIfNull(directory);
        IntPtr nativeDirectory = Marshal.StringToCoTaskMemUTF8(directory);
        try
        {
            return CallString("gts_files_diff_json", (out NativeMethods.GtsBuffer buffer, out IntPtr error) =>
                NativeMethods.gts_files_diff_json(data, ToUIntPtr(data.Length), nativeDirectory, out buffer, out error));
        }
        finally
        {
            Marshal.FreeCoTaskMem(nativeDirectory);
        }
    }

    private delegate GtsStatus BufferCall(out NativeMethods.GtsBuffer buffer, out IntPtr error);

    private static string CallString(string operation, BufferCall call)
    {
        return CallBuffer(operation, call, CopyUtf8);
    }

    private static byte[] CallBytes(string operation, BufferCall call)
    {
        return CallBuffer(operation, call, CopyBytes);
    }

    private static T CallBuffer<T>(string operation, BufferCall call, Func<NativeMethods.GtsBuffer, T> copy)
    {
        NativeMethods.GtsBuffer buffer = default;
        IntPtr error = IntPtr.Zero;
        GtsStatus status = call(out buffer, out error);
        if (status != GtsStatus.Ok)
        {
            throw BuildException(operation, status, error);
        }
        if (error != IntPtr.Zero)
        {
            throw BuildException(operation, GtsStatus.Internal, error);
        }
        try
        {
            return copy(buffer);
        }
        finally
        {
            NativeMethods.gts_buffer_free(ref buffer);
        }
    }

    private static GtsException BuildException(string operation, GtsStatus status, IntPtr error)
    {
        string code = string.Empty;
        string detail = string.Empty;
        try
        {
            if (error != IntPtr.Zero)
            {
                code = CopyUtf8(NativeMethods.gts_error_code(error));
                detail = CopyUtf8(NativeMethods.gts_error_message(error));
            }
        }
        finally
        {
            if (error != IntPtr.Zero)
            {
                NativeMethods.gts_error_free(error);
            }
        }
        return new GtsException(operation, status, code, detail);
    }

    private static string CopyUtf8(NativeMethods.GtsBuffer buffer)
    {
        byte[] bytes = CopyBytes(buffer);
        return Encoding.UTF8.GetString(bytes);
    }

    private static byte[] CopyBytes(NativeMethods.GtsBuffer buffer)
    {
        int length = CheckedLength(buffer.Len);
        if (length == 0)
        {
            return Array.Empty<byte>();
        }
        var bytes = new byte[length];
        Marshal.Copy(buffer.Data, bytes, 0, length);
        return bytes;
    }

    private static string CopyUtf8(IntPtr value)
    {
        return value == IntPtr.Zero ? string.Empty : Marshal.PtrToStringUTF8(value) ?? string.Empty;
    }

    private static UIntPtr ToUIntPtr(int value)
    {
        if (value < 0)
        {
            throw new ArgumentOutOfRangeException(nameof(value), value, "Length must be non-negative.");
        }
        return new UIntPtr((uint)value);
    }

    private static int CheckedLength(UIntPtr value)
    {
        ulong length = value.ToUInt64();
        if (length > int.MaxValue)
        {
            throw new InvalidOperationException($"C ABI output is too large for a managed byte array: {length} bytes.");
        }
        return (int)length;
    }
}
