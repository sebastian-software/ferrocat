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


def check_draft(tag, expected_id=None, expected_source_sha=None):
    repository = os.environ["GITHUB_REPOSITORY"]
    # GitHub's REST tag endpoint excludes drafts. gh resolves drafts through
    # its release listing; fetch that numeric ID again for the current state.
    resolved = json.loads(run_gh("release", "view", tag, "--repo", repository, "--json", "apiUrl"))
    prefix = f"https://api.github.com/repos/{repository}/releases/"
    match = re.fullmatch(re.escape(prefix) + r"([1-9][0-9]*)", resolved.get("apiUrl", ""))
    if not match:
        raise ValueError("Release lookup must resolve an unambiguous ID in this repository")
    release_id = int(match.group(1))
    release = json.loads(run_gh("api", f"repos/{repository}/releases/{release_id}"))
    if (
        release.get("id") != release_id
        or release.get("tag_name") != tag
        or release.get("draft") is not True
        or release.get("immutable") is True
    ):
        raise ValueError("A matching mutable draft release is required; leave public releases untouched")
    if expected_id is not None and release_id != expected_id:
        raise ValueError("Draft release ID does not match the reviewed recovery")
    if expected_source_sha is not None and release.get("target_commitish") != expected_source_sha:
        raise ValueError("Draft source does not match the reviewed recovery")
    return release


def check_recovery(tag, release_id, source_sha, run_id, rust_job_id):
    check_draft(tag, release_id, source_sha)
    repository = os.environ["GITHUB_REPOSITORY"]
    run = json.loads(run_gh("api", f"repos/{repository}/actions/runs/{run_id}"))
    job = json.loads(run_gh("api", f"repos/{repository}/actions/jobs/{rust_job_id}"))
    if (
        run.get("id") != run_id
        or run.get("head_sha") != source_sha
        or run.get("event") != "push"
        or run.get("path") != ".github/workflows/publish.yml"
        or run.get("status") != "completed"
        or job.get("run_id") != run_id
        or job.get("name") != "publish-rust"
        or job.get("status") != "completed"
        or job.get("conclusion") != "success"
    ):
        raise ValueError("Recovery requires the original tagged publish run's successful Rust job")


def publish(tag, version, expected_id=None, expected_source_sha=None):
    if tag != f"ferrocat-v{version}":
        raise ValueError("Release tag and version do not match")
    release = check_draft(tag, expected_id, expected_source_sha)
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
    parser.add_argument("mode", choices=("check-draft", "check-recovery", "publish"))
    parser.add_argument("tag")
    parser.add_argument("version")
    parser.add_argument("--release-id", type=int)
    parser.add_argument("--source-sha")
    parser.add_argument("--run-id", type=int)
    parser.add_argument("--rust-job-id", type=int)
    args = parser.parse_args()
    if args.tag != f"ferrocat-v{args.version}":
        parser.error("release tag and version do not match")
    if args.mode == "check-recovery":
        if not all((args.release_id, args.source_sha, args.run_id, args.rust_job_id)):
            parser.error("recovery requires release ID, source SHA, original run and successful Rust job")
        check_recovery(args.tag, args.release_id, args.source_sha, args.run_id, args.rust_job_id)
    elif args.mode == "check-draft":
        check_draft(args.tag, args.release_id, args.source_sha)
    else:
        publish(args.tag, args.version, args.release_id, args.source_sha)


if __name__ == "__main__":
    main()
