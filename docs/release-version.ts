import releaseManifest from "../.release-please-manifest.json"

const releaseVersion = releaseManifest["crates/ferrocat"]

if (typeof releaseVersion !== "string" || releaseVersion.trim() === "") {
  throw new Error("Missing crates/ferrocat version in .release-please-manifest.json")
}

export const ferrocatReleaseVersion = releaseVersion
