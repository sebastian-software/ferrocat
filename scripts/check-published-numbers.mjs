#!/usr/bin/env node
import { readFileSync } from "node:fs"
import { resolve } from "node:path"

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

for (const entry of mapping.entries) {
  checkEntry(entry)
}

for (const assertion of mapping.textAssertions ?? []) {
  checkTextAssertion(assertion)
}

console.log(`Published-number check passed for ${mapping.entries.length} benchmark values.`)
