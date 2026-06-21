<?php
// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

declare(strict_types=1);

namespace Gmeow\Gts;

use FFI;
use FFI\CData;
use InvalidArgumentException;
use RuntimeException;

final class Gts
{
    private const CDEF_TEMPLATE = <<<'CDEF'
typedef unsigned char uint8_t;
typedef unsigned int uint32_t;
@GTS_SIZE_T_TYPEDEF@

typedef enum gts_status {
  GTS_STATUS_OK = 0,
  GTS_STATUS_INVALID_ARGUMENT = 1,
  GTS_STATUS_IO = 2,
  GTS_STATUS_PARSE = 3,
  GTS_STATUS_DIAGNOSTIC = 4,
  GTS_STATUS_INTERNAL = 5,
  GTS_STATUS_PANIC = 6
} gts_status;

typedef struct gts_buffer {
  uint8_t *data;
  size_t len;
  size_t capacity;
} gts_buffer;

typedef struct gts_error gts_error;

uint32_t gts_abi_version(void);
const char *gts_version(void);

void gts_buffer_free(gts_buffer *buffer);
void gts_error_free(gts_error *error);
const char *gts_error_code(const gts_error *error);
const char *gts_error_message(const gts_error *error);

gts_status gts_build_metadata_json(gts_buffer *out, gts_error **error);
gts_status gts_capabilities_json(gts_buffer *out, gts_error **error);
gts_status gts_read_json(const uint8_t *data, size_t len, gts_buffer *out, gts_error **error);
gts_status gts_verify_json(const uint8_t *data, size_t len, gts_buffer *out, gts_error **error);
gts_status gts_to_nquads(const uint8_t *data, size_t len, gts_buffer *out, gts_error **error);
gts_status gts_from_nquads(const char *text, size_t len, gts_buffer *out, gts_error **error);

gts_status gts_files_pack(const char *const *paths, size_t path_count, gts_buffer *out, gts_error **error);
gts_status gts_files_unpack(
  const uint8_t *data,
  size_t len,
  const char *dest,
  uint32_t flags,
  gts_buffer *out,
  gts_error **error
);
gts_status gts_files_diff_json(
  const uint8_t *data,
  size_t len,
  const char *directory,
  gts_buffer *out,
  gts_error **error
);
CDEF;

    private FFI $ffi;

    public function __construct(?string $library = null)
    {
        $this->ffi = FFI::cdef(self::cdef(), $library ?? self::defaultLibrary());
    }

    public static function load(?string $library = null): self
    {
        return new self($library);
    }

    public static function defaultLibrary(): string
    {
        $fromEnv = getenv('GTS_LIBGTS');
        if (is_string($fromEnv) && $fromEnv !== '') {
            return $fromEnv;
        }
        return match (PHP_OS_FAMILY) {
            'Windows' => 'gts.dll',
            'Darwin' => 'libgts.dylib',
            default => 'libgts.so',
        };
    }

    public function abiVersion(): int
    {
        return (int) $this->ffi->gts_abi_version();
    }

    public function version(): string
    {
        return self::copyCString($this->ffi->gts_version());
    }

    public function buildMetadataJson(): string
    {
        return $this->callBuffer(
            'gts_build_metadata_json',
            fn (CData $out, CData $error): int => (int) $this->ffi->gts_build_metadata_json($out, $error)
        );
    }

    public function capabilitiesJson(): string
    {
        return $this->callBuffer(
            'gts_capabilities_json',
            fn (CData $out, CData $error): int => (int) $this->ffi->gts_capabilities_json($out, $error)
        );
    }

    public function readJson(string $data): string
    {
        return $this->withBytes(
            $data,
            'uint8_t',
            fn (CData $dataPtr, int $len): string => $this->callBuffer(
                'gts_read_json',
                fn (CData $out, CData $error): int => (int) $this->ffi->gts_read_json(
                    $dataPtr,
                    $len,
                    $out,
                    $error
                )
            )
        );
    }

    public function verifyJson(string $data): string
    {
        return $this->withBytes(
            $data,
            'uint8_t',
            fn (CData $dataPtr, int $len): string => $this->callBuffer(
                'gts_verify_json',
                fn (CData $out, CData $error): int => (int) $this->ffi->gts_verify_json(
                    $dataPtr,
                    $len,
                    $out,
                    $error
                )
            )
        );
    }

    public function toNQuads(string $data): string
    {
        return $this->withBytes(
            $data,
            'uint8_t',
            fn (CData $dataPtr, int $len): string => $this->callBuffer(
                'gts_to_nquads',
                fn (CData $out, CData $error): int => (int) $this->ffi->gts_to_nquads(
                    $dataPtr,
                    $len,
                    $out,
                    $error
                )
            )
        );
    }

    public function fromNQuads(string $text): string
    {
        return $this->withBytes(
            $text,
            'char',
            fn (CData $textPtr, int $len): string => $this->callBuffer(
                'gts_from_nquads',
                fn (CData $out, CData $error): int => (int) $this->ffi->gts_from_nquads(
                    $textPtr,
                    $len,
                    $out,
                    $error
                )
            )
        );
    }

    /**
     * @param list<string> $paths
     */
    public function filesPack(array $paths): string
    {
        $paths = array_values($paths);
        [$pathPointers, $pathBuffers] = $this->nativePathList($paths);
        $pathCount = count($paths);

        return $this->callBuffer(
            'gts_files_pack',
            function (CData $out, CData $error) use ($pathPointers, $pathBuffers, $pathCount): int {
                return (int) $this->ffi->gts_files_pack($pathPointers, $pathCount, $out, $error);
            }
        );
    }

    public function filesUnpack(string $data, string $destination, int $flags = GtsUnpackFlags::NONE): string
    {
        $destinationPtr = $this->nativeString($destination, 'destination');

        return $this->withBytes(
            $data,
            'uint8_t',
            fn (CData $dataPtr, int $len): string => $this->callBuffer(
                'gts_files_unpack',
                fn (CData $out, CData $error): int => (int) $this->ffi->gts_files_unpack(
                    $dataPtr,
                    $len,
                    $destinationPtr,
                    $flags,
                    $out,
                    $error
                )
            )
        );
    }

    public function filesDiffJson(string $data, string $directory): string
    {
        $directoryPtr = $this->nativeString($directory, 'directory');

        return $this->withBytes(
            $data,
            'uint8_t',
            fn (CData $dataPtr, int $len): string => $this->callBuffer(
                'gts_files_diff_json',
                fn (CData $out, CData $error): int => (int) $this->ffi->gts_files_diff_json(
                    $dataPtr,
                    $len,
                    $directoryPtr,
                    $out,
                    $error
                )
            )
        );
    }

    /**
     * @param callable(CData, CData): int $call
     */
    private function callBuffer(string $operation, callable $call): string
    {
        $out = $this->ffi->new('gts_buffer');
        $error = $this->ffi->new('gts_error *');

        try {
            $status = $call(FFI::addr($out), FFI::addr($error));
            if ($status !== GtsStatus::OK) {
                throw $this->buildException($operation, $status, $error);
            }
            if (!FFI::isNull($error)) {
                throw $this->buildException($operation, GtsStatus::INTERNAL, $error);
            }
            return self::copyBuffer($out);
        } finally {
            $this->ffi->gts_buffer_free(FFI::addr($out));
        }
    }

    private static function cdef(): string
    {
        $sizeT = PHP_OS_FAMILY === 'Windows'
            ? 'typedef unsigned long long size_t;'
            : 'typedef unsigned long size_t;';
        return str_replace('@GTS_SIZE_T_TYPEDEF@', $sizeT, self::CDEF_TEMPLATE);
    }

    /**
     * @param callable(CData, int): string $call
     */
    private function withBytes(string $data, string $elementType, callable $call): string
    {
        $len = strlen($data);
        $buffer = $this->ffi->new(sprintf('%s[%d]', $elementType, max(1, $len)));
        if ($len > 0) {
            FFI::memcpy($buffer, $data, $len);
        }
        return $call($buffer, $len);
    }

    /**
     * @param list<string> $paths
     * @return array{0: CData, 1: list<CData>}
     */
    private function nativePathList(array $paths): array
    {
        if ($paths === []) {
            throw new InvalidArgumentException('Path list must not be empty.');
        }

        $pathBuffers = [];
        $pathPointers = $this->ffi->new(sprintf('const char *[%d]', count($paths)));
        foreach ($paths as $index => $path) {
            if (!is_string($path)) {
                throw new InvalidArgumentException(sprintf('Path at index %d is not a string.', $index));
            }
            $pathBuffers[$index] = $this->nativeString($path, sprintf('source[%d]', $index));
            $pathPointers[$index] = $this->ffi->cast('char *', $pathBuffers[$index]);
        }

        return [$pathPointers, $pathBuffers];
    }

    private function nativeString(string $value, string $name): CData
    {
        if (str_contains($value, "\0")) {
            throw new InvalidArgumentException(sprintf('%s contains a NUL byte.', $name));
        }
        $len = strlen($value);
        $buffer = $this->ffi->new(sprintf('char[%d]', $len + 1));
        if ($len > 0) {
            FFI::memcpy($buffer, $value, $len);
        }
        $buffer[$len] = "\0";
        return $buffer;
    }

    private function buildException(string $operation, int $status, CData $error): GtsException
    {
        $code = '';
        $detail = '';

        try {
            if (!FFI::isNull($error)) {
                $code = self::copyCString($this->ffi->gts_error_code($error));
                $detail = self::copyCString($this->ffi->gts_error_message($error));
            }
        } finally {
            if (!FFI::isNull($error)) {
                $this->ffi->gts_error_free($error);
            }
        }

        return new GtsException($operation, $status, $code, $detail);
    }

    private static function copyBuffer(CData $buffer): string
    {
        $len = self::checkedLength((int) $buffer->len);
        if ($len === 0) {
            return '';
        }
        if (FFI::isNull($buffer->data)) {
            throw new RuntimeException('C ABI returned a null data pointer with non-zero length.');
        }
        return FFI::string($buffer->data, $len);
    }

    private static function copyCString(mixed $value): string
    {
        if (is_string($value)) {
            return $value;
        }
        if ($value === null || FFI::isNull($value)) {
            return '';
        }
        return FFI::string($value);
    }

    private static function checkedLength(int $len): int
    {
        if ($len < 0) {
            throw new RuntimeException(sprintf('C ABI returned a negative buffer length: %d.', $len));
        }
        return $len;
    }
}
