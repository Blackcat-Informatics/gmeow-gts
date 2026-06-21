// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

using System;
using System.Runtime.InteropServices;

namespace Gmeow.Gts;

internal static class NativeMethods
{
    private const string Library = "gts";

    [StructLayout(LayoutKind.Sequential)]
    internal struct GtsBuffer
    {
        internal IntPtr Data;
        internal UIntPtr Len;
        internal UIntPtr Capacity;
    }

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl)]
    internal static extern uint gts_abi_version();

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl)]
    internal static extern IntPtr gts_version();

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl)]
    internal static extern void gts_buffer_free(ref GtsBuffer buffer);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl)]
    internal static extern void gts_error_free(IntPtr error);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl)]
    internal static extern IntPtr gts_error_code(IntPtr error);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl)]
    internal static extern IntPtr gts_error_message(IntPtr error);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl)]
    internal static extern GtsStatus gts_build_metadata_json(out GtsBuffer output, out IntPtr error);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl)]
    internal static extern GtsStatus gts_capabilities_json(out GtsBuffer output, out IntPtr error);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl)]
    internal static extern GtsStatus gts_read_json(
        byte[] data,
        UIntPtr length,
        out GtsBuffer output,
        out IntPtr error);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl)]
    internal static extern GtsStatus gts_verify_json(
        byte[] data,
        UIntPtr length,
        out GtsBuffer output,
        out IntPtr error);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl)]
    internal static extern GtsStatus gts_to_nquads(
        byte[] data,
        UIntPtr length,
        out GtsBuffer output,
        out IntPtr error);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl)]
    internal static extern GtsStatus gts_from_nquads(
        byte[] text,
        UIntPtr length,
        out GtsBuffer output,
        out IntPtr error);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl)]
    internal static extern GtsStatus gts_files_pack(
        IntPtr[] paths,
        UIntPtr pathCount,
        out GtsBuffer output,
        out IntPtr error);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl)]
    internal static extern GtsStatus gts_files_unpack(
        byte[] data,
        UIntPtr length,
        IntPtr destination,
        uint flags,
        out GtsBuffer output,
        out IntPtr error);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl)]
    internal static extern GtsStatus gts_files_diff_json(
        byte[] data,
        UIntPtr length,
        IntPtr directory,
        out GtsBuffer output,
        out IntPtr error);
}
