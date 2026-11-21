#!/usr/bin/env node
// Validate the schema's deliberately small JSON Schema vocabulary without a dependency.
import { readFile, readdir } from "node:fs/promises";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(fileURLToPath(new URL("..", import.meta.url)));
const schema = JSON.parse(
  await readFile(resolve(root, "benchmark/compare-report.schema.json"), "utf8"),
);

function validate(value, rule, path) {
  const supported = new Set([
    "$schema",
    "title",
    "description",
    "type",
    "required",
    "properties",
    "items",
    "minimum",
  ]);
  for (const key of Object.keys(rule)) {
    if (!supported.has(key)) throw new Error(`${path}: unsupported schema keyword ${key}`);
  }
  const types = Array.isArray(rule.type) ? rule.type : [rule.type];
  const matches = (type) => {
    if (type === "null") return value === null;
    if (type === "array") return Array.isArray(value);
    if (type === "object")
      return value !== null && typeof value === "object" && !Array.isArray(value);
    if (type === "integer") return Number.isInteger(value);
    if (type === "number") return typeof value === "number" && Number.isFinite(value);
    return typeof value === type;
  };
  if (!types.some(matches)) throw new Error(`${path}: expected ${types.join(" or ")}`);
  if (rule.minimum !== undefined && value < rule.minimum) throw new Error(`${path}: below minimum`);
  for (const key of rule.required ?? []) {
    if (!Object.hasOwn(value, key)) throw new Error(`${path}: missing ${key}`);
  }
  for (const [key, child] of Object.entries(rule.properties ?? {})) {
    if (Object.hasOwn(value, key)) validate(value[key], child, `${path}.${key}`);
  }
  if (rule.items) value.forEach((item, i) => validate(item, rule.items, `${path}[${i}]`));
}
export function validateReport(value) {
  validate(value, schema, "report");
}

export async function validateReports(paths = []) {
  if (paths.length === 0) {
    paths = (await readdir(resolve(root, "benchmark/results")))
      .filter((name) => name.endsWith(".json"))
      .map((name) => resolve(root, "benchmark/results", name));
  }
  for (const path of paths) {
    validateReport(JSON.parse(await readFile(path, "utf8")));
    console.log(`Valid benchmark report: ${path}`);
  }
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  await validateReports(process.argv.slice(2));
}
