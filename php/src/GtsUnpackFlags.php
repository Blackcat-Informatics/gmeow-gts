<?php
// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

declare(strict_types=1);

namespace Gmeow\Gts;

final class GtsUnpackFlags
{
    public const NONE = 0;
    public const INCLUDE_SUPPRESSED = 1 << 0;
    public const ALLOW_SYMLINKS = 1 << 1;
    public const ALLOW_SPECIAL = 1 << 2;
    public const SAME_OWNER = 1 << 3;
    public const PRESERVE_SETID = 1 << 4;
}
