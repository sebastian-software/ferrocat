#!/usr/bin/env python3
"""Publish a mutable draft only after its downloaded CLI assets are complete."""

import argparse
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import subprocess
import tarfile
import tempfile

TARGET = "x86_64-unknown-linux-musl"


def run_gh(*args):
    return subprocess.check_output(["gh", *args], text=True)


def check_draft(tag):
    repository = os.environ["GITHUB_REPOSITORY"]
    release = json.loads(run_gh("api", f"repos/{repository}/releases/tags/{tag}"))
    if (
        release.get("tag_name") != tag
        or release.get("draft") is not True
        or release.get("immutable") is True
    ):
        raise ValueError("A matching mutable draft release is required; leave public releases untouched")
    return release


def publish(tag, version):
    if tag != f"ferrocat-v{version}":
        raise ValueError("Release tag and version do not match")
    release = check_draft(tag)
    artifact = f"ferrocat-{version}-{TARGET}"
    archive_name = f"{artifact}.tar.gz"
    checksum_name = f"{archive_name}.sha256"
    assets = release.get("assets", [])
    if (
        len(assets) != 2
        or {asset.get("name") for asset in assets} != {archive_name, checksum_name}
        or any(asset.get("size", 0) <= 0 or asset.get("state") != "uploaded" for asset in assets)
    ):
        raise ValueError("Draft must contain exactly the complete archive and checksum asset pair")

    with tempfile.TemporaryDirectory(prefix="ferrocat-release-") as directory:
        run_gh(
            "release", "download", tag, "--pattern", archive_name,
            "--pattern", checksum_name, "--dir", directory,
        )
        archive = Path(directory, archive_name)
        checksum = Path(directory, checksum_name).read_text(encoding="utf-8")
        match = re.fullmatch(r"([0-9a-f]{64})  " + re.escape(archive_name) + r"\n", checksum)
        if not match or hashlib.sha256(archive.read_bytes()).hexdigest() != match.group(1):
            raise ValueError("Downloaded archive does not match its canonical SHA-256 checksum")
        expected = {
            f"{artifact}/{name}"
            for name in ("ferrocat", "LICENSE-MIT", "LICENSE-APACHE", "README.md", "CLI-README.md")
        }
        with tarfile.open(archive, "r:gz") as package:
            files = []
            for member in package.getmembers():
                path = PurePosixPath(member.name)
                if path.is_absolute() or ".." in path.parts or path.parts[0] != artifact:
                    raise ValueError("Archive contains an unsafe or unexpected path")
                if member.isdir():
                    continue
                if not member.isfile() or member.size <= 0:
                    raise ValueError("Archive must contain nonempty regular files")
                files.append(member.name)
                if member.name == f"{artifact}/ferrocat" and not member.mode & 0o111:
                    raise ValueError("Packaged CLI must be executable")
            if len(files) != len(expected) or set(files) != expected:
                raise ValueError("Archive is missing the CLI, licenses, or README metadata")
    run_gh("release", "edit", tag, "--draft=false")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=("check-draft", "publish"))
    parser.add_argument("tag")
    parser.add_argument("version")
    args = parser.parse_args()
    if args.tag != f"ferrocat-v{args.version}":
        parser.error("release tag and version do not match")
    if args.mode == "check-draft":
        check_draft(args.tag)
    else:
        publish(args.tag, args.version)


if __name__ == "__main__":
    main()
