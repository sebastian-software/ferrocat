import releaseManifest from "../.release-please-manifest.json"

const releaseVersion = releaseManifest["."]

if (typeof releaseVersion !== "string" || releaseVersion.trim() === "") {
  throw new Error("Missing product version in .release-please-manifest.json")
}

export const ferrocatReleaseVersion = releaseVersion
