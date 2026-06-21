<?php
// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

declare(strict_types=1);

use Gmeow\Gts\Gts;
use Gmeow\Gts\GtsException;
use Gmeow\Gts\GtsStatus;

require __DIR__ . '/autoload.php';

if ($argc !== 2) {
    fwrite(STDERR, "usage: php -d ffi.enable=1 php/tests/smoke.php vectors/01-minimal.gts\n");
    exit(2);
}

try {
    $gts = Gts::load();
    if ($gts->abiVersion() !== 1) {
        throw new RuntimeException(sprintf('Unexpected ABI version: %d', $gts->abiVersion()));
    }
    if ($gts->version() === '') {
        throw new RuntimeException('Empty library version.');
    }

    $input = file_get_contents($argv[1]);
    if ($input === false) {
        throw new RuntimeException(sprintf('Unable to read vector: %s', $argv[1]));
    }

    expectJsonProperty('build metadata', $gts->buildMetadataJson(), 'schema', 'gts-capi-build-v1');
    expectJsonProperty('capabilities', $gts->capabilitiesJson(), 'schema', 'gts-capi-capabilities-v1');
    expectJsonProperty('read JSON', $gts->readJson($input), 'schema', 'gts-capi-read-v1');
    expectJsonProperty('verify JSON', $gts->verifyJson($input), 'schema', 'gts-capi-verify-v1');

    $nquads = $gts->toNQuads($input);
    expectContains('N-Quads', $nquads, '"Cat"@en');

    $roundTrip = $gts->fromNQuads($nquads);
    if ($roundTrip === '') {
        throw new RuntimeException('Round-trip GTS output was empty.');
    }

    try {
        $gts->fromNQuads("<https://example/s> <https://example/p> .\n");
        throw new RuntimeException('Bad N-Quads did not fail.');
    } catch (GtsException $error) {
        if ($error->status !== GtsStatus::PARSE) {
            throw new RuntimeException(sprintf('Expected parse status, got %s.', GtsStatus::name($error->status)));
        }
        if ($error->errorCode === '' || $error->detail === '') {
            throw new RuntimeException('Structured error did not include code and detail.');
        }
    }

    $temp = sys_get_temp_dir() . '/gts-php-smoke-' . bin2hex(random_bytes(8));
    $sourceDir = $temp . '/src';
    $unpackDir = $temp . '/unpack';
    try {
        if (!mkdir($sourceDir, 0777, true) && !is_dir($sourceDir)) {
            throw new RuntimeException(sprintf('Unable to create %s.', $sourceDir));
        }
        file_put_contents($sourceDir . '/a.txt', "hello\n");

        $packed = $gts->filesPack([$sourceDir]);
        expectJsonProperty('files diff', $gts->filesDiffJson($packed, $sourceDir), 'clean', true);
        expectJsonProperty('files unpack', $gts->filesUnpack($packed, $unpackDir), 'ok', true);
        if (!is_file($unpackDir . '/a.txt')) {
            throw new RuntimeException('Unpacked file missing.');
        }
    } finally {
        removeTree($temp);
    }
} catch (Throwable $error) {
    fwrite(STDERR, $error . PHP_EOL);
    exit(1);
}

function expectJsonProperty(string $label, string $json, string $property, mixed $expected): void
{
    $decoded = json_decode($json, true, 512, JSON_THROW_ON_ERROR);
    if (!is_array($decoded) || !array_key_exists($property, $decoded)) {
        throw new RuntimeException(sprintf('%s missing JSON property %s.', $label, $property));
    }
    if ($decoded[$property] !== $expected) {
        throw new RuntimeException(sprintf('%s JSON property %s had an unexpected value.', $label, $property));
    }
}

function expectContains(string $label, string $haystack, string $needle): void
{
    if (!str_contains($haystack, $needle)) {
        throw new RuntimeException(sprintf('%s did not contain %s.', $label, $needle));
    }
}

function removeTree(string $path): void
{
    if (!file_exists($path)) {
        return;
    }
    if (is_file($path) || is_link($path)) {
        unlink($path);
        return;
    }
    foreach (scandir($path) ?: [] as $entry) {
        if ($entry === '.' || $entry === '..') {
            continue;
        }
        removeTree($path . '/' . $entry);
    }
    rmdir($path);
}
