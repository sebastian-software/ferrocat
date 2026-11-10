#!/usr/bin/env python3
"""Offline regressions for immutable GitHub release staging."""

import hashlib
import importlib.util
import io
import json
import os
import subprocess
import tempfile
import textwrap
from pathlib import Path
import tarfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parent.parent
SPEC = importlib.util.spec_from_file_location("release_cli", ROOT / "scripts/release-github-cli.py")
release_cli = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(release_cli)
VERSION = "3.5.1"
TAG = f"ferrocat-v{VERSION}"
ARTIFACT = f"ferrocat-{VERSION}-{release_cli.TARGET}"
ARCHIVE = f"{ARTIFACT}.tar.gz"
CHECKSUM = f"{ARCHIVE}.sha256"


class StagingTests(unittest.TestCase):
    def setUp(self):
        self.release = {
            "tag_name": TAG, "draft": True, "immutable": False,
            "assets": [{"name": name, "size": 123, "state": "uploaded"} for name in (ARCHIVE, CHECKSUM)],
        }
        self.omit = None
        self.bad_checksum = False
        self.commands = []
        self.environment = patch.dict(os.environ, {"GITHUB_REPOSITORY": "sebastian-software/ferrocat"})
        self.environment.start()
        self.addCleanup(self.environment.stop)
        self.gh = patch.object(release_cli, "run_gh", side_effect=self.fake_gh)
        self.gh.start()
        self.addCleanup(self.gh.stop)

    def fake_gh(self, *args):
        self.commands.append(args)
        if args[0] == "api":
            return json.dumps(self.release)
        if args[:2] == ("release", "download"):
            directory = Path(args[args.index("--dir") + 1])
            archive = directory / ARCHIVE
            with tarfile.open(archive, "w:gz") as package:
                for name in ("ferrocat", "LICENSE-MIT", "LICENSE-APACHE", "README.md", "CLI-README.md"):
                    if name == self.omit:
                        continue
                    content = b"verified fixture\n"
                    entry = tarfile.TarInfo(f"{ARTIFACT}/{name}")
                    entry.mode = 0o755 if name == "ferrocat" else 0o644
                    entry.size = len(content)
                    package.addfile(entry, io.BytesIO(content))
            digest = "0" * 64 if self.bad_checksum else hashlib.sha256(archive.read_bytes()).hexdigest()
            (directory / CHECKSUM).write_text(f"{digest}  {ARCHIVE}\n", encoding="utf-8")
        return ""

    def assert_not_published(self):
        self.assertFalse(any(command[:2] == ("release", "edit") for command in self.commands))

    def test_observed_public_immutable_empty_release_is_rejected_without_mutation(self):
        self.release.update(draft=False, immutable=True, assets=[])
        with self.assertRaisesRegex(ValueError, "mutable draft"):
            release_cli.publish(TAG, VERSION)
        self.assertEqual(len(self.commands), 1)
        self.assert_not_published()

    def test_public_mutable_release_is_also_rejected(self):
        self.release["draft"] = False
        with self.assertRaisesRegex(ValueError, "mutable draft"):
            release_cli.check_draft(TAG)
        self.assert_not_published()

    def test_missing_or_extra_assets_cannot_publish(self):
        for assets in (self.release["assets"][:1], self.release["assets"] * 2):
            with self.subTest(assets=assets):
                self.release["assets"] = assets
                with self.assertRaisesRegex(ValueError, "complete archive"):
                    release_cli.publish(TAG, VERSION)
                self.assert_not_published()

    def test_downloaded_checksum_must_match(self):
        self.bad_checksum = True
        with self.assertRaisesRegex(ValueError, "SHA-256"):
            release_cli.publish(TAG, VERSION)
        self.assert_not_published()

    def test_archive_must_include_installation_metadata(self):
        self.omit = "CLI-README.md"
        with self.assertRaisesRegex(ValueError, "README metadata"):
            release_cli.publish(TAG, VERSION)
        self.assert_not_published()

    def test_complete_draft_publishes_only_after_downloaded_artifact_validation(self):
        release_cli.publish(TAG, VERSION)
        self.assertEqual([command[:2] for command in self.commands], [("api", "repos/sebastian-software/ferrocat/releases/tags/" + TAG), ("release", "download"), ("release", "edit")])
        self.assertEqual(self.commands[-1], ("release", "edit", TAG, "--draft=false"))

    def test_release_please_creates_mutable_draft_and_tag(self):
        config = json.loads((ROOT / ".release-please-config.json").read_text())
        package = config["packages"]["."]
        self.assertIs(package.get("draft"), True)
        self.assertIs(package.get("force-tag-creation"), True)

    def test_workflow_keeps_source_binding_and_final_success_gate(self):
        workflow = (ROOT / ".github/workflows/publish.yml").read_text()
        for job_name in ("publish-cli-musl", "publish-rust", "publish-github-release"):
            job = workflow.split(f"\n  {job_name}:\n", 1)[1].split("\n  publish-", 1)[0]
            self.assertIn("ref: refs/tags/${{ needs.release-please.outputs.product_tag_name }}", job)
            self.assertIn('test "$(git rev-parse HEAD)" = "$RELEASE_SOURCE_SHA"', job)
        final_job = workflow.split("\n  publish-github-release:\n", 1)[1]
        self.assertIn("needs: [release-please, publish-cli-musl, publish-rust]", final_job)
        self.assertNotIn("always()", final_job)
        self.assertIn("scripts/release-github-cli.py publish", final_job)
        upload = workflow.split("      - name: Upload musl CLI artifact to GitHub Release\n", 1)[1].split("\n  publish-rust:", 1)[0]
        self.assertIn("--clobber", upload)
        self.assertLess(upload.index("scripts/release-github-cli.py check-draft"), upload.index("gh release upload"))
        self.assertIn("python3 scripts/test-release-staging.py", (ROOT / ".github/workflows/ci.yml").read_text())


class UploadWorkflowTests(unittest.TestCase):
    """Execute the actual upload step with a local gh fixture, never GitHub."""

    def run_uploads(self, draft, immutable, attempts):
        workflow = (ROOT / ".github/workflows/publish.yml").read_text()
        upload = workflow.split("      - name: Upload musl CLI artifact to GitHub Release\n", 1)[1].split("\n  publish-rust:", 1)[0]
        style, body = upload.split("        run: ", 1)[1].split("\n", 1)
        self.assertIn(style, ("|", ">-"))
        script = textwrap.dedent(body) if style == "|" else " ".join(body.splitlines())
        script = script.replace("${{ steps.cli-release.outputs.tag }}", "$RELEASE_TAG")
        script = script.replace("${{ steps.cli-release.outputs.artifact_name }}", "$ARTIFACT_NAME")
        with tempfile.TemporaryDirectory(prefix="ferrocat-upload-test-") as directory:
            fixture = Path(directory)
            state_file = fixture / "state.json"
            state_file.write_text(json.dumps({
                "release": {"tag_name": TAG, "draft": draft, "immutable": immutable, "assets": []},
                "uploads": 0,
            }))
            gh = fixture / "gh"
            gh.write_text("""#!/usr/bin/env python3
import json
import os
from pathlib import Path
import sys
state_file = Path(os.environ["FERROCAT_UPLOAD_FIXTURE"])
state = json.loads(state_file.read_text())
args = sys.argv[1:]
if args[0] == "api":
    print(json.dumps(state["release"]))
    sys.exit(0)
if args[:2] != ["release", "upload"] or "--clobber" not in args:
    sys.exit(2)
state["uploads"] += 1
files = [Path(arg).name for arg in args[3:] if not arg.startswith("--")]
# Model gh uploading the archive successfully, then failing on its checksum.
state["release"]["assets"] = files[:1] if state["uploads"] == 1 else files
state_file.write_text(json.dumps(state))
sys.exit(1 if state["uploads"] == 1 else 0)
""")
            gh.chmod(0o755)
            environment = {
                **os.environ, "PATH": directory + os.pathsep + os.environ["PATH"],
                "GITHUB_REPOSITORY": "sebastian-software/ferrocat",
                "FERROCAT_UPLOAD_FIXTURE": str(state_file),
                "RELEASE_TAG": TAG, "RELEASE_VERSION": VERSION, "ARTIFACT_NAME": ARTIFACT,
            }
            results = [subprocess.run(
                ["bash", "-e", "-c", script], cwd=ROOT, env=environment,
                capture_output=True, text=True,
            ) for _ in range(attempts)]
            return results, json.loads(state_file.read_text())

    def test_partial_draft_upload_can_resume_after_fresh_mutability_check(self):
        results, state = self.run_uploads(draft=True, immutable=False, attempts=2)
        self.assertEqual([result.returncode for result in results], [1, 0])
        self.assertEqual(state["uploads"], 2)
        self.assertEqual(state["release"]["assets"], [ARCHIVE, CHECKSUM])

    def test_public_release_is_rejected_before_any_upload(self):
        for immutable in (False, True):
            with self.subTest(immutable=immutable):
                results, state = self.run_uploads(draft=False, immutable=immutable, attempts=1)
                self.assertNotEqual(results[0].returncode, 0)
                self.assertIn("matching mutable draft release is required", results[0].stderr)
                self.assertEqual(state["uploads"], 0)
                self.assertEqual(state["release"]["assets"], [])


if __name__ == "__main__":
    unittest.main()
