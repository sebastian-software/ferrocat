<?php

declare(strict_types=1);

require __DIR__ . '/vendor/autoload.php';

use Gettext\Loader\PoLoader;
use Gettext\Generator\PoGenerator;
use Gettext\Merge;

const TOOL = 'gettext/gettext';

function tool_version(): string
{
    $installed = @json_decode(
        (string) @file_get_contents(__DIR__ . '/vendor/composer/installed.json'),
        true
    );
    foreach ($installed['packages'] ?? [] as $package) {
        if (($package['name'] ?? '') === TOOL) {
            return TOOL . '@' . ltrim((string) ($package['version'] ?? '?'), 'v');
        }
    }
    return TOOL . '@unknown';
}

// Same canonical JSON + SHA-256 as the Python/Node adapters so the cross-tool
// digest matches: object keys sorted recursively, compact separators, UTF-8.
function canonicalize($value)
{
    if (is_array($value)) {
        if (array_is_list($value)) {
            return array_map('canonicalize', $value);
        }
        ksort($value);
        $out = [];
        foreach ($value as $key => $item) {
            $out[$key] = canonicalize($item);
        }
        return $out;
    }
    return $value;
}

function digest($value): string
{
    $rendered = json_encode(canonicalize($value), JSON_UNESCAPED_UNICODE | JSON_UNESCAPED_SLASHES);
    return hash('sha256', (string) $rendered);
}

function should_keep_header(string $key, string $value): bool
{
    return $value !== '' && !in_array($key, [
        'MIME-Version',
        'X-Generator',
        'Content-Type',
        'Content-Transfer-Encoding',
    ], true);
}

function normalize_summary($translations): array
{
    $headers = [];
    foreach ($translations->getHeaders() as $key => $value) {
        $key = (string) $key;
        $value = (string) $value;
        if (!should_keep_header($key, $value)) {
            continue;
        }
        $headers[] = ['key' => $key, 'value' => $value];
    }
    usort($headers, fn ($a, $b) => [$a['key'], $a['value']] <=> [$b['key'], $b['value']]);

    $items = [];
    foreach ($translations as $translation) {
        $plural = $translation->getPlural();
        $plural = ($plural === null || $plural === '') ? null : $plural;
        if ($plural !== null) {
            // getTranslation() is msgstr[0]; getPluralTranslations() is msgstr[1..].
            $msgstr = array_merge(
                [(string) ($translation->getTranslation() ?? '')],
                array_map('strval', array_values($translation->getPluralTranslations()))
            );
        } else {
            $msgstr = [(string) ($translation->getTranslation() ?? '')];
        }
        $context = $translation->getContext();
        $context = ($context === null || $context === '') ? null : $context;
        $items[] = [
            'msgctxt' => $context,
            'msgid' => $translation->getOriginal(),
            'msgid_plural' => $plural,
            'msgstr' => $msgstr,
            'obsolete' => $translation->isDisabled(),
        ];
    }
    usort($items, function ($a, $b) {
        $ka = [
            $a['msgctxt'] !== null ? 1 : 0,
            $a['msgctxt'] ?? '',
            $a['msgid'],
            $a['msgid_plural'] !== null ? 1 : 0,
            $a['msgid_plural'] ?? '',
            $a['msgstr'],
            $a['obsolete'] ? 1 : 0,
        ];
        $kb = [
            $b['msgctxt'] !== null ? 1 : 0,
            $b['msgctxt'] ?? '',
            $b['msgid'],
            $b['msgid_plural'] !== null ? 1 : 0,
            $b['msgid_plural'] ?? '',
            $b['msgstr'],
            $b['obsolete'] ? 1 : 0,
        ];
        return $ka <=> $kb;
    });

    return ['headers' => $headers, 'items' => $items];
}

function success_response(array $request, array $extra): array
{
    return [
        'implementation' => $request['implementation'],
        'workload' => $request['workload'],
        'fixture' => $request['fixture'],
        'success' => true,
        'semantic_digest' => $extra['semantic_digest'],
        'elapsed_ns' => $extra['elapsed_ns'],
        'bytes_processed' => $extra['bytes_processed'],
        'items_processed' => $extra['items_processed'],
        'tool_version' => tool_version(),
        'po_summary' => $extra['po_summary'] ?? null,
        'icu_summary' => null,
        'po_output_path' => $extra['po_output_path'] ?? null,
    ];
}

// Merge existing into the template by exact identity, matching ferrocat's model:
// template decides which messages exist, existing translations are carried over,
// and messages dropped from the template are kept as obsolete (disabled).
function merge_catalogs($existing, $template)
{
    $merged = $existing->mergeWith($template, Merge::TRANSLATIONS_THEIRS | Merge::HEADERS_OURS);
    foreach ($existing as $id => $translation) {
        if ($translation->isDisabled() || $translation->getOriginal() === '') {
            continue;
        }
        if ($template->find($translation->getContext(), $translation->getOriginal()) === null) {
            $obsolete = clone $translation;
            $obsolete->disable(true);
            $merged->add($obsolete);
        }
    }
    return $merged;
}

// Work around a PoGenerator bug: for obsolete (#~) multi-line messages it only
// prefixes the first line with "#~ ", leaving continuation lines unprefixed, so
// the output is not round-trippable. Re-prefix continuation lines inside any
// block that contains a "#~" marker. Runs outside the timed loop.
function fix_obsolete_wrapping(string $po): string
{
    $out = [];
    $block_obsolete = false;
    foreach (explode("\n", $po) as $line) {
        if ($line === '') {
            $block_obsolete = false;
            $out[] = $line;
            continue;
        }
        if (str_starts_with($line, '#~')) {
            $block_obsolete = true;
        }
        if ($block_obsolete && str_starts_with($line, '"')) {
            $line = '#~ ' . $line;
        }
        $out[] = $line;
    }
    return implode("\n", $out);
}

function read_file_arg(array $request, string $key): string
{
    $path = $request[$key] ?? null;
    if ($path === null) {
        fwrite(STDERR, "missing request field: {$key}\n");
        exit(1);
    }
    return (string) file_get_contents($path);
}

function main(): void
{
    global $argv;
    if (in_array('--check', $argv, true)) {
        echo tool_version();
        return;
    }

    $request = json_decode((string) file_get_contents('php://stdin'), true);
    if ($request['implementation'] !== 'php-gettext') {
        fwrite(STDERR, "unsupported php benchmark implementation: {$request['implementation']}\n");
        exit(1);
    }

    $loader = new PoLoader();
    $generator = new PoGenerator();
    $iterations = (int) $request['iterations'];
    $operation = (string) $request['operation'];

    if ($operation === 'parse') {
        $content = read_file_arg($request, 'po_input_path');
        $parsed = null;
        $started = hrtime(true);
        for ($i = 0; $i < $iterations; $i++) {
            $parsed = $loader->loadString($content);
        }
        $elapsed = hrtime(true) - $started;
        $summary = normalize_summary($parsed);
        echo json_encode(success_response($request, [
            'semantic_digest' => digest($summary),
            'elapsed_ns' => $elapsed,
            'bytes_processed' => strlen($content) * $iterations,
            'items_processed' => count($summary['items']) * $iterations,
            'po_summary' => $request['capture_artifacts'] ? $summary : null,
        ]), JSON_UNESCAPED_UNICODE);
        return;
    }

    if ($operation === 'stringify') {
        $content = read_file_arg($request, 'po_input_path');
        $parsed = $loader->loadString($content);
        $rendered = '';
        $started = hrtime(true);
        for ($i = 0; $i < $iterations; $i++) {
            $rendered = $generator->generateString($parsed);
        }
        $elapsed = hrtime(true) - $started;
        $summary = normalize_summary($loader->loadString($rendered));
        if ($request['capture_artifacts'] && !empty($request['po_output_path'])) {
            file_put_contents($request['po_output_path'], $rendered);
        }
        echo json_encode(success_response($request, [
            'semantic_digest' => digest($summary),
            'elapsed_ns' => $elapsed,
            'bytes_processed' => strlen($rendered) * $iterations,
            'items_processed' => count($summary['items']) * $iterations,
            'po_output_path' => $request['capture_artifacts'] ? ($request['po_output_path'] ?? null) : null,
        ]), JSON_UNESCAPED_UNICODE);
        return;
    }

    if ($operation === 'merge' || $operation === 'update-catalog') {
        $existingContent = read_file_arg($request, 'existing_po_path');
        $templateContent = read_file_arg($request, 'pot_path');
        $rendered = '';
        $started = hrtime(true);
        for ($i = 0; $i < $iterations; $i++) {
            $merged = merge_catalogs(
                $loader->loadString($existingContent),
                $loader->loadString($templateContent)
            );
            $rendered = $generator->generateString($merged);
        }
        $elapsed = hrtime(true) - $started;
        // Repair PoGenerator's obsolete multi-line output before validation.
        $rendered = fix_obsolete_wrapping($rendered);
        $summary = normalize_summary($loader->loadString($rendered));
        if ($request['capture_artifacts'] && !empty($request['po_output_path'])) {
            file_put_contents($request['po_output_path'], $rendered);
        }
        echo json_encode(success_response($request, [
            'semantic_digest' => digest($summary),
            'elapsed_ns' => $elapsed,
            'bytes_processed' => strlen($rendered) * $iterations,
            'items_processed' => count($summary['items']) * $iterations,
            'po_output_path' => $request['capture_artifacts'] ? ($request['po_output_path'] ?? null) : null,
        ]), JSON_UNESCAPED_UNICODE);
        return;
    }

    fwrite(STDERR, "unsupported php benchmark operation: {$operation}\n");
    exit(1);
}

main();
