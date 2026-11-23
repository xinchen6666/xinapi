"""Changes select only the builds affected by the source or recipe contract."""

from pathlib import Path
import subprocess
import tempfile
import unittest

from tools.select_checks import affected_targets, changed_paths


class ChangedTargetTests(unittest.TestCase):
    def test_documentation_change_needs_no_image_build(self):
        self.assertEqual(
            affected_targets(["docs/worklog/X03.md", "README.md"]),
            {"new-api": False, "cpr": False},
        )

    def test_each_service_or_recipe_selects_only_that_image(self):
        for path in ("services/new-api/main.go", "deploy/docker/new-api.Dockerfile"):
            self.assertEqual(affected_targets([path]), {"new-api": True, "cpr": False})
        for path in (
            "services/codex-proxy-rs/backend/Cargo.lock",
            "deploy/docker/cpr.Dockerfile.dockerignore",
        ):
            self.assertEqual(affected_targets([path]), {"new-api": False, "cpr": True})

    def test_shared_build_or_ci_changes_select_both_images(self):
        for path in (
            "tools/build_image.py",
            "deploy/build-targets.json",
            ".github/workflows/x03.yml",
        ):
            self.assertEqual(affected_targets([path]), {"new-api": True, "cpr": True})

    def test_ci_sha_validation_cannot_be_option_injection(self):
        with self.assertRaises(ValueError):
            changed_paths(Path.cwd(), "", "--help")

    def test_git_diff_handles_spaces_and_deletions(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            subprocess.run(["git", "init", "-q"], cwd=root, check=True)
            file = root / "source with spaces.txt"
            file.write_text("one\n")
            subprocess.run(["git", "add", "."], cwd=root, check=True)
            commit = [
                "git",
                "-c",
                "user.name=ci-test",
                "-c",
                "user.email=test@example.invalid",
                "commit",
                "-q",
                "-m",
                "fixture",
            ]
            subprocess.run(commit, cwd=root, check=True)
            base = subprocess.check_output(
                ["git", "rev-parse", "HEAD"], cwd=root, text=True
            ).strip()
            file.unlink()
            subprocess.run(["git", "add", "-u"], cwd=root, check=True)
            subprocess.run(commit, cwd=root, check=True)
            head = subprocess.check_output(
                ["git", "rev-parse", "HEAD"], cwd=root, text=True
            ).strip()
            self.assertEqual(
                changed_paths(root, base, head), ["source with spaces.txt"]
            )
            self.assertIsNone(changed_paths(root, "0" * 40, head))


if __name__ == "__main__":
    unittest.main()
