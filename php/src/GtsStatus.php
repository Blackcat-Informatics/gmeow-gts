<?php
// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

declare(strict_types=1);

namespace Gmeow\Gts;

final class GtsStatus
{
    public const OK = 0;
    public const INVALID_ARGUMENT = 1;
    public const IO = 2;
    public const PARSE = 3;
    public const DIAGNOSTIC = 4;
    public const INTERNAL = 5;
    public const PANIC = 6;

    public static function name(int $status): string
    {
        return match ($status) {
            self::OK => 'OK',
            self::INVALID_ARGUMENT => 'INVALID_ARGUMENT',
            self::IO => 'IO',
            self::PARSE => 'PARSE',
            self::DIAGNOSTIC => 'DIAGNOSTIC',
            self::INTERNAL => 'INTERNAL',
            self::PANIC => 'PANIC',
            default => 'UNKNOWN',
        };
    }
}
