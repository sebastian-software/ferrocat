#!/usr/bin/env node

const fs = require('node:fs');
const crypto = require('node:crypto');
const path = require('node:path');
const gettextParser = require('gettext-parser');
const PO = require('pofile');
const pofileTs = require('pofile-ts');
const formatjsParser = require('@formatjs/icu-messageformat-parser');
const messageformatParser = require('@messageformat/parser');

function canonicalize(value) {
  if (Array.isArray(value)) {
    return value.map(canonicalize);
  }
  if (value && typeof value === 'object') {
    const out = {};
    for (const key of Object.keys(value).sort()) {
      out[key] = canonicalize(value[key]);
    }
    return out;
  }
  return value;
}

function digest(value) {
  const rendered = JSON.stringify(canonicalize(value));
  return crypto.createHash('sha256').update(rendered).digest('hex');
}

function shouldKeepHeader(key, value) {
  return value !== '' && ![
    'MIME-Version',
    'X-Generator',
    'Content-Type',
    'Content-Transfer-Encoding'
  ].includes(key);
}

function packageVersion(name) {
  if (name === 'pofile-ts') {
    const packageJsonPath = path.join(path.dirname(require.resolve('pofile-ts')), '..', 'package.json');
    return JSON.parse(fs.readFileSync(packageJsonPath, 'utf8')).version;
  }
  return require(`${name}/package.json`).version;
}

function normalizePoSummary(parsed) {
  const headers = [];
  const rawHeaders = parsed.headers || {};
  for (const key of Object.keys(rawHeaders)) {
    const value = String(rawHeaders[key]);
    if (!shouldKeepHeader(key, value)) {
      continue;
    }
    headers.push({ key, value });
  }
  headers.sort((a, b) => (a.key === b.key ? a.value.localeCompare(b.value) : a.key.localeCompare(b.key)));

  const items = (parsed.items || []).map((item) => {
    let msgstr = [];
    if (Array.isArray(item.msgstr)) {
      msgstr = item.msgstr.map((entry) => String(entry));
    } else if (typeof item.msgstr === 'string') {
      msgstr = [item.msgstr];
    }
    return {
      msgctxt: item.msgctxt || null,
      msgid: String(item.msgid || ''),
      msgid_plural: item.msgid_plural || null,
      msgstr,
      obsolete: Boolean(item.obsolete)
    };
  });
  items.sort(comparePoItems);

  return { headers, items };
}

function normalizeGettextParserSummary(parsed) {
  const headers = [];
  const rawHeaders = parsed.headers || {};
  for (const key of Object.keys(rawHeaders)) {
    const value = String(rawHeaders[key]);
    if (!shouldKeepHeader(key, value)) {
      continue;
    }
    headers.push({ key, value });
  }
  headers.sort((a, b) => (a.key === b.key ? a.value.localeCompare(b.value) : a.key.localeCompare(b.key)));

  const items = [];
  const translations = parsed.translations || {};
  for (const [contextKey, contextItems] of Object.entries(translations)) {
    for (const [msgidKey, item] of Object.entries(contextItems || {})) {
      if (contextKey === '' && msgidKey === '') {
        continue;
      }
      items.push({
        msgctxt: item.msgctxt || null,
        msgid: String(item.msgid || ''),
        msgid_plural: item.msgid_plural || null,
        msgstr: Array.isArray(item.msgstr) ? item.msgstr.map((entry) => String(entry)) : [],
        obsolete: Boolean(item.obsolete)
      });
    }
  }
  items.sort(comparePoItems);

  return { headers, items };
}

function compareOptionalStrings(left, right) {
  if (left === null && right === null) {
    return 0;
  }
  if (left === null) {
    return -1;
  }
  if (right === null) {
    return 1;
  }
  return left.localeCompare(right);
}

function compareStringArrays(left, right) {
  const limit = Math.min(left.length, right.length);
  for (let index = 0; index < limit; index += 1) {
    const result = left[index].localeCompare(right[index]);
    if (result !== 0) {
      return result;
    }
  }
  if (left.length < right.length) {
    return -1;
  }
  if (left.length > right.length) {
    return 1;
  }
  return 0;
}

function comparePoItems(left, right) {
  let result = compareOptionalStrings(left.msgctxt, right.msgctxt);
  if (result !== 0) {
    return result;
  }

  result = left.msgid.localeCompare(right.msgid);
  if (result !== 0) {
    return result;
  }

  result = compareOptionalStrings(left.msgid_plural, right.msgid_plural);
  if (result !== 0) {
    return result;
  }

  result = compareStringArrays(left.msgstr, right.msgstr);
  if (result !== 0) {
    return result;
  }

  if (left.obsolete === right.obsolete) {
    return 0;
  }
  return left.obsolete ? 1 : -1;
}

function summarizeFormatjsAst(ast) {
  const collector = {
    variable_names: new Set(),
    selector_kinds: new Set(),
    selectors: new Set(),
    plural_categories: new Set(),
    tag_names: new Set(),
    formatter_kinds: new Set(),
    literal_segments: 0,
    argument_count: 0,
    pound_count: 0,
    max_depth: 0
  };

  function visitNodes(nodes, depth) {
    collector.max_depth = Math.max(collector.max_depth, depth);
    for (const node of nodes || []) {
      visitNode(node, depth);
    }
  }

  function visitNode(node, depth) {
    collector.max_depth = Math.max(collector.max_depth, depth);
    switch (node.type) {
      case 0:
        collector.literal_segments += 1;
        break;
      case 1:
        collector.argument_count += 1;
        collector.variable_names.add(node.value);
        break;
      case 2:
        collector.argument_count += 1;
        collector.variable_names.add(node.value);
        collector.formatter_kinds.add('number');
        break;
      case 3:
        collector.argument_count += 1;
        collector.variable_names.add(node.value);
        collector.formatter_kinds.add('date');
        break;
      case 4:
        collector.argument_count += 1;
        collector.variable_names.add(node.value);
        collector.formatter_kinds.add('time');
        break;
      case 5:
        collector.argument_count += 1;
        collector.variable_names.add(node.value);
        collector.selector_kinds.add('select');
        for (const [selector, option] of Object.entries(node.options || {})) {
          collector.selectors.add(selector);
          visitNodes(option.value || option, depth + 1);
        }
        break;
      case 6:
        collector.argument_count += 1;
        collector.variable_names.add(node.value);
        collector.selector_kinds.add(node.pluralType === 'ordinal' ? 'selectordinal' : 'plural');
        for (const [selector, option] of Object.entries(node.options || {})) {
          collector.selectors.add(selector);
          collector.plural_categories.add(selector);
          visitNodes(option.value || option, depth + 1);
        }
        break;
      case 7:
        collector.pound_count += 1;
        break;
      case 8:
        collector.tag_names.add(node.value);
        visitNodes(node.children || [], depth + 1);
        break;
      default:
        break;
    }
  }

  visitNodes(ast, 1);
  return {
    variable_names: Array.from(collector.variable_names).sort(),
    selector_kinds: Array.from(collector.selector_kinds).sort(),
    selectors: Array.from(collector.selectors).sort(),
    plural_categories: Array.from(collector.plural_categories).sort(),
    tag_names: Array.from(collector.tag_names).sort(),
    formatter_kinds: Array.from(collector.formatter_kinds).sort(),
    literal_segments: collector.literal_segments,
    argument_count: collector.argument_count,
    pound_count: collector.pound_count,
    max_depth: collector.max_depth
  };
}

function summarizeMessageformatAst(ast) {
  const collector = {
    variable_names: new Set(),
    selector_kinds: new Set(),
    selectors: new Set(),
    plural_categories: new Set(),
    tag_names: new Set(),
    formatter_kinds: new Set(),
    literal_segments: 0,
    argument_count: 0,
    pound_count: 0,
    max_depth: 0
  };

  function visitNodes(nodes, depth) {
    collector.max_depth = Math.max(collector.max_depth, depth);
    for (const node of nodes || []) {
      visitNode(node, depth);
    }
  }

  function visitNode(node, depth) {
    collector.max_depth = Math.max(collector.max_depth, depth);
    if (!node) {
      return;
    }
    switch (node.type) {
      case 'content':
        collector.literal_segments += 1;
        break;
      case 'argument':
        collector.argument_count += 1;
        collector.variable_names.add(node.arg);
        break;
      case 'function':
        collector.argument_count += 1;
        collector.variable_names.add(node.arg);
        collector.formatter_kinds.add(String(node.key));
        break;
      case 'octothorpe':
        collector.pound_count += 1;
        break;
      case 'select':
      case 'plural':
      case 'selectordinal':
        collector.argument_count += 1;
        collector.variable_names.add(node.arg);
        collector.selector_kinds.add(node.type === 'plural' ? 'plural' : node.type);
        for (const option of node.cases || []) {
          collector.selectors.add(String(option.key));
          if (node.type !== 'select') {
            collector.plural_categories.add(String(option.key));
          }
          visitNodes(option.tokens || [], depth + 1);
        }
        break;
      case 'tag':
        collector.tag_names.add(String(node.name));
        visitNodes(node.tokens || [], depth + 1);
        break;
      default:
        if (Array.isArray(node.tokens)) {
          visitNodes(node.tokens, depth + 1);
        }
        break;
    }
  }

  visitNodes(ast, 1);
  return {
    variable_names: Array.from(collector.variable_names).sort(),
    selector_kinds: Array.from(collector.selector_kinds).sort(),
    selectors: Array.from(collector.selectors).sort(),
    plural_categories: Array.from(collector.plural_categories).sort(),
    tag_names: Array.from(collector.tag_names).sort(),
    formatter_kinds: Array.from(collector.formatter_kinds).sort(),
    literal_segments: collector.literal_segments,
    argument_count: collector.argument_count,
    pound_count: collector.pound_count,
    max_depth: collector.max_depth
  };
}

function successResponse(base, extra) {
  return {
    implementation: base.implementation,
    workload: base.workload,
    fixture: base.fixture,
    success: true,
    semantic_digest: extra.semantic_digest,
    elapsed_ns: extra.elapsed_ns,
    bytes_processed: extra.bytes_processed,
    items_processed: extra.items_processed ?? null,
    messages_processed: extra.messages_processed ?? null,
    tool_version: extra.tool_version,
    po_summary: extra.po_summary ?? null,
    icu_summary: extra.icu_summary ?? null,
    po_output_path: extra.po_output_path ?? null
  };
}

function itemKey(item) {
  return `${item.msgctxt ?? ''}\u0004${item.msgid ?? ''}\u0004${item.msgid_plural ?? ''}`;
}

function mergeMsgstr(templateItem, existingItem) {
  if (templateItem.msgid_plural) {
    const templateValues = Array.isArray(templateItem.msgstr) ? templateItem.msgstr : [];
    const existingValues = existingItem && Array.isArray(existingItem.msgstr)
      ? existingItem.msgstr.map((value) => String(value))
      : [];
    const targetLength = Math.max(
      templateValues.length,
      existingValues.length,
      templateItem.nplurals || 0,
      existingItem?.nplurals || 0,
      1
    );
    const out = [];
    for (let index = 0; index < targetLength; index += 1) {
      out.push(existingValues[index] ?? templateValues[index] ?? '');
    }
    return out;
  }

  const existingValue = existingItem && Array.isArray(existingItem.msgstr)
    ? String(existingItem.msgstr[0] ?? '')
    : '';
  return [existingValue];
}

function mergePoLikeCatalog(existingDoc, templateDoc) {
  const existingActiveByKey = new Map();
  for (const item of existingDoc.items || []) {
    if (!item || item.obsolete || !item.msgid) {
      continue;
    }
    existingActiveByKey.set(itemKey(item), item);
  }

  const templateKeys = new Set();
  const mergedItems = [];
  for (const item of templateDoc.items || []) {
    if (!item || !item.msgid) {
      continue;
    }
    const key = itemKey(item);
    templateKeys.add(key);
    const existingItem = existingActiveByKey.get(key);
    item.obsolete = false;
    item.msgstr = mergeMsgstr(item, existingItem);
    mergedItems.push(item);
  }

  const obsoleteItems = [];
  for (const item of existingDoc.items || []) {
    if (!item || item.obsolete || !item.msgid) {
      continue;
    }
    if (templateKeys.has(itemKey(item))) {
      continue;
    }
    item.obsolete = true;
    obsoleteItems.push(item);
  }

  existingDoc.items = mergedItems.concat(obsoleteItems);
  return existingDoc;
}

function runPofile(request) {
  const toolVersion = `pofile@${require('pofile/package.json').version}`;

  if (request.operation === 'parse') {
    const input = fs.readFileSync(request.po_input_path, 'utf8');
    const parsed = PO.parse(input);
    let summary = normalizePoSummary(parsed);
    const start = process.hrtime.bigint();
    for (let i = 0; i < request.iterations; i += 1) {
      summary = normalizePoSummary(PO.parse(input));
    }
    const elapsed = process.hrtime.bigint() - start;
    return successResponse(request, {
      semantic_digest: digest(summary),
      elapsed_ns: Number(elapsed),
      bytes_processed: Buffer.byteLength(input, 'utf8') * request.iterations,
      items_processed: summary.items.length * request.iterations,
      tool_version: toolVersion,
      po_summary: request.capture_artifacts ? summary : null
    });
  }

  if (request.operation === 'merge' || request.operation === 'update-catalog') {
    const existingInput = fs.readFileSync(request.existing_po_path, 'utf8');
    const templateInput = fs.readFileSync(request.pot_path, 'utf8');
    let rendered = '';
    let summary = null;
    const start = process.hrtime.bigint();
    for (let i = 0; i < request.iterations; i += 1) {
      const merged = mergePoLikeCatalog(PO.parse(existingInput), PO.parse(templateInput));
      rendered = merged.toString();
      summary = normalizePoSummary(PO.parse(rendered));
    }
    const elapsed = process.hrtime.bigint() - start;
    if (request.capture_artifacts && request.po_output_path) {
      fs.writeFileSync(request.po_output_path, rendered, 'utf8');
    }
    return successResponse(request, {
      semantic_digest: digest(summary),
      elapsed_ns: Number(elapsed),
      bytes_processed: Buffer.byteLength(rendered, 'utf8') * request.iterations,
      items_processed: summary.items.length * request.iterations,
      tool_version: toolVersion,
      po_output_path: request.capture_artifacts ? request.po_output_path : null
    });
  }

  const input = fs.readFileSync(request.po_input_path, 'utf8');
  const parsed = PO.parse(input);
  let rendered = '';
  const start = process.hrtime.bigint();
  for (let i = 0; i < request.iterations; i += 1) {
    rendered = parsed.toString();
  }
  const elapsed = process.hrtime.bigint() - start;
  const reparsed = PO.parse(rendered);
  const summary = normalizePoSummary(reparsed);
  if (request.capture_artifacts && request.po_output_path) {
    fs.writeFileSync(request.po_output_path, rendered, 'utf8');
  }
  return successResponse(request, {
    semantic_digest: digest(summary),
    elapsed_ns: Number(elapsed),
    bytes_processed: Buffer.byteLength(rendered, 'utf8') * request.iterations,
    items_processed: summary.items.length * request.iterations,
    tool_version: toolVersion,
    po_output_path: request.capture_artifacts ? request.po_output_path : null
  });
}

function runPofileTs(request) {
  const toolVersion = `pofile-ts@${packageVersion('pofile-ts')}`;

  if (request.operation === 'parse') {
    const input = fs.readFileSync(request.po_input_path, 'utf8');
    let summary = normalizePoSummary(pofileTs.parsePo(input));
    const start = process.hrtime.bigint();
    for (let i = 0; i < request.iterations; i += 1) {
      summary = normalizePoSummary(pofileTs.parsePo(input));
    }
    const elapsed = process.hrtime.bigint() - start;
    return successResponse(request, {
      semantic_digest: digest(summary),
      elapsed_ns: Number(elapsed),
      bytes_processed: Buffer.byteLength(input, 'utf8') * request.iterations,
      items_processed: summary.items.length * request.iterations,
      tool_version: toolVersion,
      po_summary: request.capture_artifacts ? summary : null
    });
  }

  if (request.operation === 'merge' || request.operation === 'update-catalog') {
    const existingInput = fs.readFileSync(request.existing_po_path, 'utf8');
    const templateInput = fs.readFileSync(request.pot_path, 'utf8');
    let rendered = '';
    let summary = null;
    const start = process.hrtime.bigint();
    for (let i = 0; i < request.iterations; i += 1) {
      const merged = mergePoLikeCatalog(
        pofileTs.parsePo(existingInput),
        pofileTs.parsePo(templateInput)
      );
      rendered = pofileTs.stringifyPo(merged);
      summary = normalizePoSummary(pofileTs.parsePo(rendered));
    }
    const elapsed = process.hrtime.bigint() - start;
    if (request.capture_artifacts && request.po_output_path) {
      fs.writeFileSync(request.po_output_path, rendered, 'utf8');
    }
    return successResponse(request, {
      semantic_digest: digest(summary),
      elapsed_ns: Number(elapsed),
      bytes_processed: Buffer.byteLength(rendered, 'utf8') * request.iterations,
      items_processed: summary.items.length * request.iterations,
      tool_version: toolVersion,
      po_output_path: request.capture_artifacts ? request.po_output_path : null
    });
  }

  const input = fs.readFileSync(request.po_input_path, 'utf8');
  const parsed = pofileTs.parsePo(input);
  let rendered = '';
  const start = process.hrtime.bigint();
  for (let i = 0; i < request.iterations; i += 1) {
    rendered = pofileTs.stringifyPo(parsed);
  }
  const elapsed = process.hrtime.bigint() - start;
  const reparsed = pofileTs.parsePo(rendered);
  const summary = normalizePoSummary(reparsed);
  if (request.capture_artifacts && request.po_output_path) {
    fs.writeFileSync(request.po_output_path, rendered, 'utf8');
  }
  return successResponse(request, {
    semantic_digest: digest(summary),
    elapsed_ns: Number(elapsed),
    bytes_processed: Buffer.byteLength(rendered, 'utf8') * request.iterations,
    items_processed: summary.items.length * request.iterations,
    tool_version: toolVersion,
    po_output_path: request.capture_artifacts ? request.po_output_path : null
  });
}

function runGettextParser(request) {
  const input = fs.readFileSync(request.po_input_path);
  const toolVersion = `gettext-parser@${packageVersion('gettext-parser')}`;

  if (request.operation === 'parse') {
    let summary = normalizeGettextParserSummary(gettextParser.po.parse(input));
    const start = process.hrtime.bigint();
    for (let i = 0; i < request.iterations; i += 1) {
      summary = normalizeGettextParserSummary(gettextParser.po.parse(input));
    }
    const elapsed = process.hrtime.bigint() - start;
    return successResponse(request, {
      semantic_digest: digest(summary),
      elapsed_ns: Number(elapsed),
      bytes_processed: input.byteLength * request.iterations,
      items_processed: summary.items.length * request.iterations,
      tool_version: toolVersion,
      po_summary: request.capture_artifacts ? summary : null
    });
  }

  const parsed = gettextParser.po.parse(input);
  let rendered = Buffer.alloc(0);
  const start = process.hrtime.bigint();
  for (let i = 0; i < request.iterations; i += 1) {
    rendered = gettextParser.po.compile(parsed);
  }
  const elapsed = process.hrtime.bigint() - start;
  const summary = normalizeGettextParserSummary(gettextParser.po.parse(rendered));
  if (request.capture_artifacts && request.po_output_path) {
    fs.writeFileSync(request.po_output_path, rendered);
  }
  return successResponse(request, {
    semantic_digest: digest(summary),
    elapsed_ns: Number(elapsed),
    bytes_processed: rendered.byteLength * request.iterations,
    items_processed: summary.items.length * request.iterations,
    tool_version: toolVersion,
    po_output_path: request.capture_artifacts ? request.po_output_path : null
  });
}

function runFormatjs(request) {
  const messages = JSON.parse(fs.readFileSync(request.icu_messages_path, 'utf8'));
  const toolVersion = `@formatjs/icu-messageformat-parser@${require('@formatjs/icu-messageformat-parser/package.json').version}`;
  let summary = null;
  const start = process.hrtime.bigint();
  for (let i = 0; i < request.iterations; i += 1) {
    summary = {
      messages: messages.map((message) => summarizeFormatjsAst(formatjsParser.parse(message, { captureLocation: false })))
    };
  }
  const elapsed = process.hrtime.bigint() - start;
  const bytes = messages.reduce((total, message) => total + Buffer.byteLength(message, 'utf8'), 0);
  return successResponse(request, {
    semantic_digest: digest(summary),
    elapsed_ns: Number(elapsed),
    bytes_processed: bytes * request.iterations,
    messages_processed: messages.length * request.iterations,
    tool_version: toolVersion,
    icu_summary: request.capture_artifacts ? summary : null
  });
}

function runMessageformat(request) {
  const messages = JSON.parse(fs.readFileSync(request.icu_messages_path, 'utf8'));
  const toolVersion = `@messageformat/parser@${require('@messageformat/parser/package.json').version}`;
  let summary = null;
  const start = process.hrtime.bigint();
  for (let i = 0; i < request.iterations; i += 1) {
    summary = {
      messages: messages.map((message) => summarizeMessageformatAst(messageformatParser.parse(message)))
    };
  }
  const elapsed = process.hrtime.bigint() - start;
  const bytes = messages.reduce((total, message) => total + Buffer.byteLength(message, 'utf8'), 0);
  return successResponse(request, {
    semantic_digest: digest(summary),
    elapsed_ns: Number(elapsed),
    bytes_processed: bytes * request.iterations,
    messages_processed: messages.length * request.iterations,
    tool_version: toolVersion,
    icu_summary: request.capture_artifacts ? summary : null
  });
}

function run() {
  if (process.argv.includes('--check')) {
    const versions = [
      `pofile@${packageVersion('pofile')}`,
      `pofile-ts@${packageVersion('pofile-ts')}`,
      `gettext-parser@${packageVersion('gettext-parser')}`,
      `@formatjs/icu-messageformat-parser@${require('@formatjs/icu-messageformat-parser/package.json').version}`,
      `@messageformat/parser@${require('@messageformat/parser/package.json').version}`
    ];
    process.stdout.write(versions.join(', '));
    return;
  }

  const request = JSON.parse(fs.readFileSync(0, 'utf8'));
  let result;
  switch (request.implementation) {
    case 'pofile':
      result = runPofile(request);
      break;
    case 'pofile-ts':
      result = runPofileTs(request);
      break;
    case 'gettext-parser':
      result = runGettextParser(request);
      break;
    case 'formatjs-icu-parser':
      result = runFormatjs(request);
      break;
    case 'messageformat-parser':
      result = runMessageformat(request);
      break;
    default:
      throw new Error(`unsupported node benchmark implementation: ${request.implementation}`);
  }
  process.stdout.write(JSON.stringify(result));
}

run();
