<?php
// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

declare(strict_types=1);

namespace Gmeow\Gts;

use RuntimeException;

final class GtsException extends RuntimeException
{
    public function __construct(
        public readonly string $operation,
        public readonly int $status,
        public readonly string $errorCode,
        public readonly string $detail
    ) {
        parent::__construct(self::format($operation, $status, $errorCode, $detail), $status);
    }

    private static function format(string $operation, int $status, string $code, string $detail): string
    {
        $message = sprintf('%s failed with %s', $operation, GtsStatus::name($status));
        if ($code !== '') {
            $message .= sprintf(' (%s)', $code);
        }
        if ($detail !== '') {
            $message .= sprintf(': %s', $detail);
        }
        return $message;
    }
}
