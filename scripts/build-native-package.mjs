import { copyFileSync, existsSync, readFileSync, rmSync } from "node:fs";
import { execFileSync } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const packageDir = process.cwd();
const repoRoot = path.resolve(scriptDir, "..");

const packageJson = JSON.parse(
  readFileSync(path.join(packageDir, "package.json"), "utf8")
);

const targets = {
  "ferrocat-darwin-arm64": {
    platform: "darwin",
    arch: "arm64"
  },
  "ferrocat-linux-x64-gnu": {
    platform: "linux",
    arch: "x64",
    libc: "glibc"
  },
  "ferrocat-linux-arm64-gnu": {
    platform: "linux",
    arch: "arm64",
    libc: "glibc"
  },
  "ferrocat-winx64-msvc": {
    platform: "win32",
    arch: "x64"
  }
};

function detectLinuxLibc() {
  const report = process.report?.getReport?.();
  const glibcVersion = report?.header?.glibcVersionRuntime;

  if (typeof glibcVersion === "string" && glibcVersion.length > 0) {
    return "glibc";
  }

  return "musl";
}

const target = targets[packageJson.name];
const targetPath = path.join(packageDir, "ferrocat.node");

if (!target) {
  throw new Error(`Unsupported native package target: ${packageJson.name}`);
}

if (process.platform !== target.platform || process.arch !== target.arch) {
  if (existsSync(targetPath)) {
    rmSync(targetPath);
  }
  console.log(
    `Skipping native build for ${packageJson.name} on ${process.platform}/${process.arch}`
  );
  process.exit(0);
}

if (
  target.platform === "linux" &&
  target.libc &&
  detectLinuxLibc() !== target.libc
) {
  if (existsSync(targetPath)) {
    rmSync(targetPath);
  }
  console.log(`Skipping native build for ${packageJson.name} due to libc mismatch`);
  process.exit(0);
}

const profile = process.env.FERROCAT_RUST_PROFILE === "release" ? "release" : "debug";
const extensionByPlatform = {
  darwin: "dylib",
  linux: "so",
  win32: "dll"
};

const extension = extensionByPlatform[process.platform];

if (!extension) {
  throw new Error(`Unsupported platform for native build: ${process.platform}`);
}

const cargoArgs = ["build", "--package", "ferrocat-node"];

if (profile === "release") {
  cargoArgs.push("--release");
}

execFileSync("cargo", cargoArgs, {
  cwd: repoRoot,
  stdio: "inherit"
});

const binaryName =
  process.platform === "win32" ? "ferrocat_node.dll" : `libferrocat_node.${extension}`;
const sourcePath = path.join(repoRoot, "target", profile, binaryName);

if (!existsSync(sourcePath)) {
  throw new Error(`Expected native binary at ${sourcePath}`);
}

copyFileSync(sourcePath, targetPath);
