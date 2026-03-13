import { readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "..");

function detectLinuxLibc() {
  const report = process.report?.getReport?.();
  const glibcVersion = report?.header?.glibcVersionRuntime;

  if (typeof glibcVersion === "string" && glibcVersion.length > 0) {
    return "glibc";
  }

  return "musl";
}

function resolveWorkspacePackageDir() {
  if (process.platform === "darwin" && process.arch === "arm64") {
    return "ferrocat-darwin-arm64";
  }

  if (process.platform === "linux" && process.arch === "x64") {
    if (detectLinuxLibc() === "glibc") {
      return "ferrocat-linux-x64-gnu";
    }

    return null;
  }

  if (process.platform === "linux" && process.arch === "arm64") {
    if (detectLinuxLibc() === "glibc") {
      return "ferrocat-linux-arm64-gnu";
    }

    return null;
  }

  if (process.platform === "win32" && process.arch === "x64") {
    return "ferrocat-winx64-msvc";
  }

  return null;
}

const packageDirName = resolveWorkspacePackageDir();

if (!packageDirName) {
  console.log(`Skipping native smoke test for unsupported platform ${process.platform}/${process.arch}`);
  process.exit(0);
}

const packageJson = JSON.parse(
  readFileSync(path.join(repoRoot, "packages", packageDirName, "package.json"), "utf8")
);

if (typeof packageJson.main !== "string" || packageJson.main.length === 0) {
  console.log(`Skipping native smoke test for placeholder package ${packageJson.name}`);
  process.exit(0);
}

const { default: binding } = await import("../packages/ferrocat/index.js");

const requiredExports = [
  "parsePoJson",
  "stringifyPoJson",
  "compileIcuJson",
  "bindingVersion"
];

for (const key of requiredExports) {
  if (!(key in binding)) {
    throw new Error(`Missing expected Ferrocat export: ${key}`);
  }
}

console.log(
  `Loaded Ferrocat native binding with exports: ${requiredExports.join(", ")}`
);
