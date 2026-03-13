"use strict";

function detectLinuxLibc() {
  const report = process.report?.getReport?.();
  const glibcVersion = report?.header?.glibcVersionRuntime;

  if (typeof glibcVersion === "string" && glibcVersion.length > 0) {
    return "glibc";
  }

  return "musl";
}

function resolvePackageName() {
  if (process.platform === "darwin" && process.arch === "arm64") {
    return "ferrocat-darwin-arm64";
  }

  if (process.platform === "linux" && process.arch === "x64") {
    if (detectLinuxLibc() === "glibc") {
      return "ferrocat-linux-x64-gnu";
    }

    throw new Error("Ferrocat does not yet ship a linux-x64 musl package.");
  }

  if (process.platform === "linux" && process.arch === "arm64") {
    if (detectLinuxLibc() === "glibc") {
      return "ferrocat-linux-arm64-gnu";
    }

    throw new Error("Ferrocat does not yet ship a linux-arm64 musl package.");
  }

  if (process.platform === "win32" && process.arch === "x64") {
    return "ferrocat-win32-x64-msvc";
  }

  throw new Error(
    `Unsupported Ferrocat platform: ${process.platform}/${process.arch}`
  );
}

function loadBinding() {
  const packageName = resolvePackageName();

  try {
    return require(packageName);
  } catch (error) {
    throw new Error(
      `Failed to load the native Ferrocat package "${packageName}". ` +
        "Make sure optional dependencies were installed for this platform.",
      { cause: error }
    );
  }
}

module.exports = loadBinding();
