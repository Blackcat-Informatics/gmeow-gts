<?php
// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

declare(strict_types=1);

use Gmeow\Gts\Gts;
use Gmeow\Gts\GtsException;
use Gmeow\Gts\GtsStatus;

require __DIR__ . '/autoload.php';

if ($argc !== 4) {
    fwrite(
        STDERR,
        "usage: php -d ffi.enable=1 php/tests/smoke.php vectors/01-minimal.gts vectors/04-damaged-frame.gts vectors/28-empty-file.gts\n"
    );
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
    $damaged = file_get_contents($argv[2]);
    if ($damaged === false) {
        throw new RuntimeException(sprintf('Unable to read vector: %s', $argv[2]));
    }
    $empty = file_get_contents($argv[3]);
    if ($empty === false) {
        throw new RuntimeException(sprintf('Unable to read vector: %s', $argv[3]));
    }

    expectJsonProperty('build metadata', $gts->buildMetadataJson(), 'schema', 'gts-capi-build-v1');
    expectJsonProperty('capabilities', $gts->capabilitiesJson(), 'schema', 'gts-capi-capabilities-v1');
    $cleanRead = $gts->readJson($input);
    expectJsonProperty('php clean-read read JSON', $cleanRead, 'schema', 'gts-capi-read-v1');
    expectJsonProperty('php clean-read read JSON', $cleanRead, 'clean', true);
    expectJsonProperty('verify JSON', $gts->verifyJson($input), 'schema', 'gts-capi-verify-v1');

    $damagedRead = $gts->readJson($damaged);
    expectJsonProperty('php damaged-diagnostic-read read JSON', $damagedRead, 'schema', 'gts-capi-read-v1');
    expectJsonProperty('php damaged-diagnostic-read read JSON', $damagedRead, 'clean', false);
    expectDiagnostic('php damaged-diagnostic-read read JSON', $damagedRead, 'DamagedFrame');
    expectGtsException(
        'php damaged-diagnostic-read toNQuads',
        fn (): string => $gts->toNQuads($damaged),
        GtsStatus::DIAGNOSTIC
    );

    $emptyRead = $gts->readJson($empty);
    expectJsonProperty('php empty-malformed-refusal read JSON', $emptyRead, 'schema', 'gts-capi-read-v1');
    expectJsonProperty('php empty-malformed-refusal read JSON', $emptyRead, 'clean', false);
    expectDiagnostic('php empty-malformed-refusal read JSON', $emptyRead, 'EmptyFile');
    expectGtsException(
        'php empty-malformed-refusal toNQuads',
        fn (): string => $gts->toNQuads($empty),
        GtsStatus::DIAGNOSTIC
    );

    $nquads = $gts->toNQuads($input);
    expectContains('N-Quads', $nquads, '"Cat"@en');

    $roundTrip = $gts->fromNQuads($nquads);
    if ($roundTrip === '') {
        throw new RuntimeException('Round-trip GTS output was empty.');
    }

    expectGtsException(
        'php malformed-nquads-refusal fromNQuads',
        fn (): string => $gts->fromNQuads(getenv('GTS_WRAPPER_BAD_NQUADS') ?: "<https://example/s> <https://example/p> .\n"),
        GtsStatus::PARSE
    );

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

function expectDiagnostic(string $label, string $json, string $expectedCode): void
{
    $decoded = json_decode($json, true, 512, JSON_THROW_ON_ERROR);
    if (!is_array($decoded) || !isset($decoded['diagnostics']) || !is_array($decoded['diagnostics'])) {
        throw new RuntimeException(sprintf('%s missing diagnostics array.', $label));
    }
    foreach ($decoded['diagnostics'] as $diagnostic) {
        if (is_array($diagnostic) && ($diagnostic['code'] ?? null) === $expectedCode) {
            return;
        }
    }
    throw new RuntimeException(sprintf('%s missing diagnostic %s.', $label, $expectedCode));
}

/**
 * @param callable(): mixed $callback
 */
function expectGtsException(string $label, callable $callback, int $expectedStatus): void
{
    try {
        $callback();
    } catch (GtsException $error) {
        if ($error->status !== $expectedStatus) {
            throw new RuntimeException(sprintf('%s expected %s, got %s.', $label, GtsStatus::name($expectedStatus), GtsStatus::name($error->status)));
        }
        if ($error->errorCode === '' || $error->detail === '') {
            throw new RuntimeException(sprintf('%s structured error did not include code and detail.', $label));
        }
        return;
    }
    throw new RuntimeException(sprintf('%s did not fail with %s.', $label, GtsStatus::name($expectedStatus)));
}

function removeTree(string $path): void
{
    if (!file_exists($path) && !is_link($path)) {
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
