#!/usr/bin/env node
/*
 * Generates the table behind the default `OrderBy::Msgid` catalog identity
 * order.
 *
 * Lingui orders catalogs with `new Intl.Collator("en-US")`, which resolves to
 * the unmodified CLDR root collation because English carries no tailoring of
 * its own. Rather than linking ICU4X and its ~1.3 MB of baked data, this derives
 * the part of the root order source messages normally exercise: primary
 * weights for Latin text, punctuation, symbols and digits, plus secondary
 * accent ranks and canonical decompositions.
 *
 * Run with `--check` to verify the checked-in table is current.
 */

import { readFileSync, writeFileSync } from "node:fs"
import path from "node:path"
import { fileURLToPath } from "node:url"

const OUTPUT = path.join(
  path.dirname(fileURLToPath(import.meta.url)),
  "..",
  "crates",
  "ferrocat-po",
  "src",
  "api",
  "collation_table.rs"
)

const collator = new Intl.Collator("en-US")

/*
 * Printable ASCII, common typographic UI characters, Latin-1 Supplement and
 * Latin Extended-A. Anything else sorts after this table by code point.
 */
function repertoire() {
  const chars = []
  for (let code = 0x20; code < 0x7f; code++) chars.push(String.fromCodePoint(code))
  for (const character of "‘’“”–—…·•€£¥©®™°±×÷§¶†‡→←↑↓«»‹›„‚") chars.push(character)
  for (let code = 0xa1; code <= 0x01_7f; code++) chars.push(String.fromCodePoint(code))
  return chars
}

const isCombining = (character) => /\p{Mn}/u.test(character)

function baseOf(character) {
  const stripped = [...character.normalize("NFD")]
    .filter((candidate) => !isCombining(candidate))
    .join("")
  return (stripped || character).toLowerCase()
}

const RANGE_START = 0x20
const RANGE_END = 0x01_7f
const MARK_START = 0x0300
const MARK_END = 0x036f

function compareBytes(left, right) {
  const length = Math.min(left.length, right.length)
  for (let index = 0; index < length; index++) {
    if (left[index] !== right[index]) return left[index] - right[index]
  }
  return left.length - right.length
}

/*
 * Exhaustively check a focused accent-position corpus against the real oracle.
 * The 427 NFC/NFD forms produce 90,951 pairs and cover zero, one, and two marks
 * across three equal primary characters. Intl-equal canonical spellings may
 * still use the raw identity tie-break in Rust, so only non-equal pairs constrain
 * this secondary encoding.
 */
function verifySecondaryEncoding(secondaryWeightOf) {
  const accents = [
    "",
    "\u0300",
    "\u0301",
    "\u0302",
    "\u0303",
    "\u0308",
    "\u030a",
    "\u0327",
    "\u0328",
  ]
  const forms = new Set()
  for (const first of accents) {
    for (const second of accents) {
      for (const third of accents) {
        if ([first, second, third].filter(Boolean).length > 2) continue
        const decomposed = `a${first}a${second}a${third}`
        forms.add(decomposed)
        forms.add(decomposed.normalize("NFC"))
      }
    }
  }

  const secondaryKey = (text) => {
    const key = []
    let primaryPosition = 0
    for (const character of text.normalize("NFD")) {
      const code = character.codePointAt(0)
      if (code >= MARK_START && code <= MARK_END) {
        const position = 0xffff_ffff - primaryPosition
        key.push(
          position >>> 24,
          (position >>> 16) & 0xff,
          (position >>> 8) & 0xff,
          position & 0xff,
          secondaryWeightOf.get(character)
        )
      } else {
        primaryPosition++
      }
    }
    return key
  }

  const corpus = [...forms]
  for (let left = 0; left < corpus.length; left++) {
    for (let right = left + 1; right < corpus.length; right++) {
      const oracle = Math.sign(collator.compare(corpus[left], corpus[right]))
      if (oracle === 0) continue
      const encoded = Math.sign(
        compareBytes(secondaryKey(corpus[left]), secondaryKey(corpus[right]))
      )
      if (encoded !== oracle) {
        throw new Error(
          `Secondary encoding disagrees with Intl.Collator for ${JSON.stringify(corpus[left])} and ${JSON.stringify(corpus[right])}.`
        )
      }
    }
  }
}

function buildTable() {
  const chars = repertoire()

  const bases = [...new Set(chars.map(baseOf))].filter((base) => [...base].length === 1)
  bases.sort((left, right) => collator.compare(left, right) || (left < right ? -1 : 1))
  if (bases.length >= 0xff) {
    throw new Error(
      `Repertoire has ${bases.length} base characters; a primary weight must fit in a byte below 0xFF.`
    )
  }
  const weightOf = new Map(bases.map((base, index) => [base, index + 1]))

  const marks = []
  for (let code = MARK_START; code <= MARK_END; code++) {
    marks.push(String.fromCodePoint(code))
  }
  marks.sort(
    (left, right) =>
      collator.compare(`a${left}`, `a${right}`) ||
      left.codePointAt(0) - right.codePointAt(0)
  )
  const secondaryWeightOf = new Map()
  let secondaryWeight = 0
  let previousMark
  for (const mark of marks) {
    if (
      previousMark === undefined ||
      collator.compare(`a${previousMark}`, `a${mark}`) !== 0
    ) {
      secondaryWeight++
    }
    secondaryWeightOf.set(mark, secondaryWeight)
    previousMark = mark
  }
  if (secondaryWeight >= 0xff) {
    throw new Error(
      `Combining-mark block has ${secondaryWeight} distinct secondary weights; a weight must fit in a byte below 0xFF.`
    )
  }
  verifySecondaryEncoding(secondaryWeightOf)
  const markWeights = []
  for (let code = MARK_START; code <= MARK_END; code++) {
    markWeights.push(secondaryWeightOf.get(String.fromCodePoint(code)))
  }

  const rowFor = (character) => {
    const nfd = [...character.normalize("NFD")]
    const marks = nfd.filter((candidate) => isCombining(candidate))
    if (marks.length > 1) {
      throw new Error(
        `Repertoire character ${JSON.stringify(character)} has multiple canonical marks; extend the table format before adding it.`
      )
    }
    return {
      weight: weightOf.get(baseOf(character)) ?? 0,
      secondary: marks.length === 1 ? secondaryWeightOf.get(marks[0]) : 0,
      upper: character !== character.toLowerCase(),
    }
  }

  const rows = []
  for (let code = RANGE_START; code <= RANGE_END; code++) {
    rows.push(rowFor(String.fromCodePoint(code)))
  }

  const extra = chars
    .filter((character) => {
      const code = character.codePointAt(0)
      return code < RANGE_START || code > RANGE_END
    })
    .map((character) => [character, rowFor(character)])
  extra.sort(([left], [right]) => left.codePointAt(0) - right.codePointAt(0))

  return { rows, extra, markWeights }
}

/*
 * Emit format and control characters as escapes. Invisible source text would
 * be difficult to review and trips Clippy's `invisible_characters` lint.
 */
const isInvisible = (character) =>
  /[\p{Cf}\p{Cc}\p{Zl}\p{Zp}]/u.test(character) ||
  (/\p{Zs}/u.test(character) && character !== " ")

const rustChar = (character) => {
  if (isInvisible(character)) {
    return `'\\u{${character.codePointAt(0).toString(16).toUpperCase()}}'`
  }
  if (character === "\\") return "'\\\\'"
  if (character === "'") return "'\\''"
  return `'${character}'`
}

const renderRow = ({ weight, secondary, upper }) =>
  `Row { primary: ${weight}, secondary: ${secondary}, upper: ${upper} }`

function render({ rows, extra, markWeights }) {
  const dense = rows.map((row) => `    ${renderRow(row)},`).join("\n")
  const extraRows = extra
    .map(([character, row]) => `    (${rustChar(character)}, ${renderRow(row)}),`)
    .join("\n")
  const renderedMarkWeights = markWeights.join(", ")

  return `// @generated by scripts/generate-collation-table.mjs — do not edit.
//
// Derived from \`Intl.Collator("en-US")\`, which resolves to the unmodified
// CLDR root collation. Regenerate with
// \`node scripts/generate-collation-table.mjs\`.

/// Everything the key builder needs for one code point.
pub(super) struct Row {
    /// Rank of the base character in CLDR root order, or 0 when uncovered.
    pub(super) primary: u8,
    /// CLDR secondary rank contributed by a combining mark, or 0 for none.
    pub(super) secondary: u8,
    /// Whether the character is uppercase, a tertiary difference.
    pub(super) upper: bool,
}

/// First code point covered by [\`ROWS\`].
pub(super) const RANGE_START: u32 = ${RANGE_START};

/// Dense rows indexed by \`code point - RANGE_START\`.
#[rustfmt::skip]
pub(super) const ROWS: &[Row] = &[
${dense}
];

/// Covered characters above the dense range, sorted for binary search.
#[rustfmt::skip]
pub(super) const EXTRA: &[(char, Row)] = &[
${extraRows}
];

/// First combining mark covered by [\`MARK_WEIGHTS\`].
pub(super) const MARK_START: u32 = ${MARK_START};

/// CLDR secondary ranks indexed by \`code point - MARK_START\`.
#[rustfmt::skip]
pub(super) const MARK_WEIGHTS: &[u8] = &[${renderedMarkWeights}];
`
}

const rendered = render(buildTable())

if (process.argv.includes("--check")) {
  const current = readFileSync(OUTPUT, "utf8")
  if (current !== rendered) {
    console.error(
      `${OUTPUT} is out of date. Run \`node scripts/generate-collation-table.mjs\` and commit the result.`
    )
    process.exit(1)
  }
  console.log("Collation table is up to date.")
} else {
  writeFileSync(OUTPUT, rendered)
  console.log(`Wrote ${OUTPUT}`)
}
