#!/usr/bin/env node
import { readdirSync, readFileSync } from "node:fs"
import { join, relative, resolve } from "node:path"

const repoRoot = resolve(import.meta.dirname, "..")
const mappingPath = resolve(repoRoot, "benchmark/published-numbers.json")
const mapping = JSON.parse(readFileSync(mappingPath, "utf8"))

function readJson(path) {
  return JSON.parse(readFileSync(resolve(repoRoot, path), "utf8"))
}

function readText(path) {
  return readFileSync(resolve(repoRoot, path), "utf8")
}

function getPath(value, path) {
  return path.split(".").reduce((current, segment) => {
    if (current == null || !(segment in current)) {
      throw new Error(`Missing metric path ${path}`)
    }
    return current[segment]
  }, value)
}

function checkEntry(entry) {
  const report = readJson(entry.report)
  const scenario = report.scenarios.find((candidate) => candidate.id === entry.scenario)
  if (!scenario) {
    throw new Error(`${entry.id}: missing scenario ${entry.scenario} in ${entry.report}`)
  }

  const actual = Number(getPath(scenario, entry.metric))
  if (!Number.isFinite(actual)) {
    throw new Error(`${entry.id}: metric ${entry.metric} is not numeric`)
  }

  const delta = Math.abs(actual - entry.published)
  if (delta > entry.tolerance) {
    throw new Error(
      `${entry.id}: published ${entry.published} differs from ${actual.toFixed(3)} by ${delta.toFixed(3)} (tolerance ${entry.tolerance})`,
    )
  }

  for (const location of entry.locations ?? []) {
    const text = readText(location.file)
    if (!text.includes(location.contains)) {
      throw new Error(`${entry.id}: ${location.file} does not contain ${JSON.stringify(location.contains)}`)
    }
  }
}

function checkTextAssertion(assertion) {
  const text = readText(assertion.file)
  if (assertion.contains && !text.includes(assertion.contains)) {
    throw new Error(`${assertion.id}: ${assertion.file} does not contain ${JSON.stringify(assertion.contains)}`)
  }
  if (assertion.notContains && text.includes(assertion.notContains)) {
    throw new Error(`${assertion.id}: ${assertion.file} still contains ${JSON.stringify(assertion.notContains)}`)
  }
}

// Every throughput number in user-facing docs must be registered above (so it
// is validated against a benchmark report) or explicitly allowlisted in
// `allowedUnregisteredNumbers` (for claims that are not ferrocat benchmark
// results). This catches numbers added to pages the mapping does not know.
const THROUGHPUT_PATTERN = /\b\d+(?:\.\d+)?\s?[MG]iB\/s\b/g

function sweptFiles() {
  const files = ["README.md"]
  const routesRoot = resolve(repoRoot, "docs/app/routes")
  for (const entry of readdirSync(routesRoot, { recursive: true, withFileTypes: true })) {
    if (entry.isFile() && /\.(mdx?|tsx?)$/.test(entry.name)) {
      files.push(relative(repoRoot, join(entry.parentPath, entry.name)))
    }
  }
  return files
}

function sweepThroughputNumbers() {
  const registeredByFile = new Map()
  const register = (file, contains) => {
    if (!registeredByFile.has(file)) {
      registeredByFile.set(file, [])
    }
    registeredByFile.get(file).push(contains)
  }
  for (const entry of mapping.entries) {
    for (const location of entry.locations ?? []) {
      register(location.file, location.contains)
    }
  }
  for (const assertion of mapping.textAssertions ?? []) {
    if (assertion.contains) {
      register(assertion.file, assertion.contains)
    }
  }
  const allowed = mapping.allowedUnregisteredNumbers ?? []

  let sweptCount = 0
  for (const file of sweptFiles()) {
    const text = readText(file)
    for (const match of text.match(THROUGHPUT_PATTERN) ?? []) {
      sweptCount += 1
      const isRegistered = (registeredByFile.get(file) ?? []).some((contains) => contains.includes(match))
      const isAllowed = allowed.some((item) => item.file === file && item.value === match)
      if (!isRegistered && !isAllowed) {
        throw new Error(
          `${file}: throughput number ${JSON.stringify(match)} is not registered in benchmark/published-numbers.json; ` +
            "add it to an entry's locations/textAssertions or to allowedUnregisteredNumbers with a reason",
        )
      }
    }
  }
  return sweptCount
}

for (const entry of mapping.entries) {
  checkEntry(entry)
}

for (const assertion of mapping.textAssertions ?? []) {
  checkTextAssertion(assertion)
}

const sweptCount = sweepThroughputNumbers()

console.log(
  `Published-number check passed for ${mapping.entries.length} benchmark values and ${sweptCount} swept throughput mentions.`,
)
