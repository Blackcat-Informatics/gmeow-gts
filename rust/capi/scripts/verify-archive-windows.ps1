# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0

[CmdletBinding()]
param(
  [Parameter(Mandatory = $true, Position = 0)]
  [string]$Archive
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Join-ArchivePath {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Base,
    [Parameter(Mandatory = $true)]
    [string]$Relative
  )

  $path = $Base
  foreach ($part in ($Relative -split "/")) {
    $path = Join-Path -Path $path -ChildPath $part
  }
  return $path
}

function Require-ArchiveMember {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Prefix,
    [Parameter(Mandatory = $true)]
    [string]$Relative
  )

  $path = Join-ArchivePath -Base $Prefix -Relative $Relative
  if (-not (Test-Path -LiteralPath $path)) {
    throw "missing archive member: $Relative"
  }
  return $path
}

function Read-TrimmedText {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Path
  )

  return (Get-Content -LiteralPath $Path -Raw).Trim()
}

$scriptDir = Split-Path -Parent $PSCommandPath
$root = (Resolve-Path (Join-Path $scriptDir "../../..")).Path
$archivePath = (Resolve-Path $Archive).Path
$tmp = Join-Path ([System.IO.Path]::GetTempPath()) ("gts-capi-archive-" + [System.Guid]::NewGuid().ToString("N"))

New-Item -ItemType Directory -Path $tmp | Out-Null
try {
  & tar -C $tmp -xzf $archivePath
  if ($LASTEXITCODE -ne 0) {
    throw "tar failed while extracting $archivePath"
  }

  $topLevel = @(Get-ChildItem -LiteralPath $tmp -Directory)
  if ($topLevel.Count -ne 1) {
    throw "archive must contain exactly one top-level directory; found $($topLevel.Count)"
  }
  $prefix = $topLevel[0].FullName

  $required = @(
    "README.md",
    "include/gts.h",
    "include/gts/gts.hpp",
    "lib/pkgconfig/gts.pc",
    "lib/cmake/Gts/GtsConfig.cmake",
    "licenses/LICENSE-MIT",
    "licenses/LICENSE-APACHE",
    "licenses/LICENSING.md",
    "licenses/LICENSES",
    "share/gts/archive.json",
    "share/gts/VERSION",
    "share/gts/ABI_VERSION",
    "bin/gts.dll",
    "lib/gts.dll.lib",
    "lib/gts.lib"
  )
  foreach ($member in $required) {
    [void](Require-ArchiveMember -Prefix $prefix -Relative $member)
  }

  $headerPath = Join-ArchivePath -Base $prefix -Relative "include/gts.h"
  $header = Get-Content -LiteralPath $headerPath -Raw
  $abiMatch = [regex]::Match($header, "(?m)^#define\s+GTS_ABI_VERSION\s+([0-9]+)u?\b")
  if (-not $abiMatch.Success) {
    throw "could not determine GTS_ABI_VERSION from include/gts.h"
  }
  $headerAbi = [uint32]$abiMatch.Groups[1].Value

  $abiMetadataPath = Join-ArchivePath -Base $prefix -Relative "share/gts/ABI_VERSION"
  $archiveAbi = [uint32](Read-TrimmedText -Path $abiMetadataPath)
  if ($archiveAbi -ne $headerAbi) {
    throw "ABI_VERSION metadata $archiveAbi does not match header ABI $headerAbi"
  }

  $versionPath = Join-ArchivePath -Base $prefix -Relative "share/gts/VERSION"
  $version = Read-TrimmedText -Path $versionPath
  if ([string]::IsNullOrWhiteSpace($version)) {
    throw "share/gts/VERSION is empty"
  }

  $archiveJsonPath = Join-ArchivePath -Base $prefix -Relative "share/gts/archive.json"
  $archiveJson = Get-Content -LiteralPath $archiveJsonPath -Raw | ConvertFrom-Json
  if ($archiveJson.schema -ne "gts-capi-archive-v1") {
    throw "unexpected archive schema: $($archiveJson.schema)"
  }
  if ($archiveJson.package -ne "gmeow-gts-capi") {
    throw "unexpected archive package: $($archiveJson.package)"
  }
  if ($archiveJson.os -ne "windows") {
    throw "Windows verifier received non-Windows archive metadata: $($archiveJson.os)"
  }
  if ($archiveJson.version -ne $version) {
    throw "archive.json version $($archiveJson.version) does not match VERSION $version"
  }
  if ([uint32]$archiveJson.abi_version -ne $headerAbi) {
    throw "archive.json ABI $($archiveJson.abi_version) does not match header ABI $headerAbi"
  }

  $dllPath = Join-ArchivePath -Base $prefix -Relative "bin/gts.dll"
  $binDir = Split-Path -Parent $dllPath
  $oldPath = [Environment]::GetEnvironmentVariable("PATH", "Process")
  [Environment]::SetEnvironmentVariable("PATH", "$binDir;$oldPath", "Process")

  $smokeSource = @"
using System;
using System.IO;
using System.Runtime.InteropServices;
using System.Text;

namespace Gts.Capi.Verify {
  public sealed class GtsArchiveSmoke : IDisposable {
    [StructLayout(LayoutKind.Sequential)]
    private struct GtsBuffer {
      public IntPtr data;
      public UIntPtr len;
      public UIntPtr capacity;
    }

    [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
    private delegate uint AbiVersionDelegate();

    [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
    private delegate int ReadJsonDelegate(IntPtr data, UIntPtr len, ref GtsBuffer output, ref IntPtr error);

    [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
    private delegate void BufferFreeDelegate(ref GtsBuffer buffer);

    [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
    private delegate void ErrorFreeDelegate(IntPtr error);

    [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
    private delegate IntPtr ErrorTextDelegate(IntPtr error);

    private readonly IntPtr library;
    private readonly AbiVersionDelegate abiVersion;
    private readonly ReadJsonDelegate readJson;
    private readonly BufferFreeDelegate bufferFree;
    private readonly ErrorFreeDelegate errorFree;
    private readonly ErrorTextDelegate errorCode;
    private readonly ErrorTextDelegate errorMessage;

    public GtsArchiveSmoke(string dllPath) {
      library = NativeLibrary.Load(dllPath);
      abiVersion = Load<AbiVersionDelegate>("gts_abi_version");
      readJson = Load<ReadJsonDelegate>("gts_read_json");
      bufferFree = Load<BufferFreeDelegate>("gts_buffer_free");
      errorFree = Load<ErrorFreeDelegate>("gts_error_free");
      errorCode = Load<ErrorTextDelegate>("gts_error_code");
      errorMessage = Load<ErrorTextDelegate>("gts_error_message");
    }

    private T Load<T>(string symbol) {
      IntPtr ptr = NativeLibrary.GetExport(library, symbol);
      return (T)(object)Marshal.GetDelegateForFunctionPointer(ptr, typeof(T));
    }

    private static string Utf8CString(IntPtr ptr) {
      if (ptr == IntPtr.Zero) {
	return "";
      }
      int len = 0;
      while (Marshal.ReadByte(ptr, len) != 0) {
	len++;
      }
      byte[] bytes = new byte[len];
      Marshal.Copy(ptr, bytes, 0, len);
      return Encoding.UTF8.GetString(bytes);
    }

    private static string BufferText(GtsBuffer buffer) {
      ulong len64 = buffer.len.ToUInt64();
      if (len64 > int.MaxValue) {
	throw new InvalidOperationException("gts_read_json output is too large to verify in memory");
      }
      byte[] bytes = new byte[(int)len64];
      Marshal.Copy(buffer.data, bytes, 0, bytes.Length);
      return Encoding.UTF8.GetString(bytes);
    }

    private string DescribeError(IntPtr error) {
      if (error == IntPtr.Zero) {
	return "no structured error";
      }
      string code = Utf8CString(errorCode(error));
      string message = Utf8CString(errorMessage(error));
      return code + ": " + message;
    }

    public void VerifyCleanRead(string vectorPath, uint expectedAbi) {
      uint actualAbi = abiVersion();
      if (actualAbi != expectedAbi) {
	throw new InvalidOperationException("gts_abi_version() " + actualAbi + " does not match archive ABI " + expectedAbi);
      }

      byte[] data = File.ReadAllBytes(vectorPath);
      GCHandle handle = GCHandle.Alloc(data, GCHandleType.Pinned);
      GtsBuffer output = new GtsBuffer();
      IntPtr error = IntPtr.Zero;
      try {
	int status = readJson(handle.AddrOfPinnedObject(), new UIntPtr((ulong)data.LongLength), ref output, ref error);
	if (status != 0) {
	  string detail = DescribeError(error);
	  if (error != IntPtr.Zero) {
	    errorFree(error);
	    error = IntPtr.Zero;
	  }
	  throw new InvalidOperationException("gts_read_json failed with status " + status + " (" + detail + ")");
	}

	string json = BufferText(output);
	if (!json.Contains("\"schema\":\"gts-capi-read-v1\"")) {
	  throw new InvalidOperationException("gts_read_json output did not contain the read report schema");
	}
	if (!json.Contains("\"clean\":true")) {
	  throw new InvalidOperationException("gts_read_json output did not report the clean vector as clean");
	}
      } finally {
	handle.Free();
	if (output.data != IntPtr.Zero) {
	  bufferFree(ref output);
	}
	if (error != IntPtr.Zero) {
	  errorFree(error);
	}
      }
    }

    public void Dispose() {
      if (library != IntPtr.Zero) {
	NativeLibrary.Free(library);
      }
    }
  }
}
"@

  Add-Type -TypeDefinition $smokeSource -Language CSharp
  $cleanVector = Join-Path $root "vectors/01-minimal.gts"
  $smoke = [Gts.Capi.Verify.GtsArchiveSmoke]::new($dllPath)
  try {
    $smoke.VerifyCleanRead($cleanVector, $headerAbi)
  } finally {
    $smoke.Dispose()
  }

  Write-Host "archive verification OK: $archivePath"
} finally {
  if (Test-Path -LiteralPath $tmp) {
    Remove-Item -LiteralPath $tmp -Recurse -Force
  }
}
