"""Build-recipe regression tests that do not require Docker or network."""

import contextlib
import io
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

from tools.build_image import (
    BuildError,
    build_command,
    main,
    prepare_snapshot,
    read_target,
    resolve_revision,
    safe_path,
)


class BuildSnapshotTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        subprocess.run(["git", "init", "-q"], cwd=self.root, check=True)
        source = self.root / "services/new-api"
        source.mkdir(parents=True)
        (source / "main.go").write_text("package main\n")
        (source / ".dockerignore").write_text(".env\n")
        deploy = self.root / "deploy/docker"
        deploy.mkdir(parents=True)
        (deploy / "new-api.Dockerfile").write_text(
            "FROM scratch\nCOPY main.go /source\n"
        )
        self.target = {
            "context": "services/new-api",
            "dockerfile": "deploy/docker/new-api.Dockerfile",
            "dockerignore": "services/new-api/.dockerignore",
            "target": None,
            "image_name": "xinapi/new-api",
        }
        spec = {
            "schema_version": 1,
            "source_repository": "https://github.com/example/xinapi",
            "targets": {"new-api": self.target},
        }
        (self.root / "deploy/build-targets.json").write_text(json.dumps(spec))
        subprocess.run(["git", "add", "services", "deploy"], cwd=self.root, check=True)
        subprocess.run(
            [
                "git",
                "-c",
                "user.name=build-test",
                "-c",
                "user.email=test@example.invalid",
                "commit",
                "-q",
                "-m",
                "fixture",
            ],
            cwd=self.root,
            check=True,
        )
        self.revision = resolve_revision(self.root, "HEAD")

    def test_snapshot_ignores_untracked_and_changed_worktree_files(self):
        (self.root / "services/new-api/.env").write_text("LOCAL_ONLY=dont-send\n")
        (self.root / "services/new-api/main.go").write_text("modified worktree\n")
        folder = self.root / "snapshot"
        folder.mkdir()
        context, dockerfile, tree = prepare_snapshot(
            self.root, self.revision, self.target, folder
        )
        self.assertEqual((context / "main.go").read_text(), "package main\n")
        self.assertFalse((context / ".env").exists())
        self.assertFalse((context / ".git").exists())
        self.assertEqual(dockerfile.read_text(), "FROM scratch\nCOPY main.go /source\n")
        self.assertEqual(len(tree), 40)
        self.assertEqual((folder / "Dockerfile.dockerignore").read_text(), ".env\n")

    def test_recipe_comes_from_selected_commit_not_worktree(self):
        (self.root / "deploy/build-targets.json").write_text("corrupt uncommitted JSON")
        spec, target = read_target(self.root, self.revision, "new-api")
        self.assertEqual(target, self.target)
        self.assertEqual(spec["source_repository"], "https://github.com/example/xinapi")

    def test_revision_must_resolve_to_an_existing_commit(self):
        self.assertEqual(resolve_revision(self.root, "HEAD"), self.revision)
        for revision in ("--help", "nonexistent-commit"):
            with (
                self.subTest(revision=revision),
                self.assertRaises(subprocess.CalledProcessError),
            ):
                resolve_revision(self.root, revision)

    def test_safe_input_paths_reject_traversal_and_absolute_paths(self):
        for path in ("../source", "/source", "source/../outside", "source//input", ""):
            with self.subTest(path=path), self.assertRaises(BuildError):
                safe_path(path)
        self.assertEqual(
            safe_path("deploy/docker/cpr.Dockerfile"), "deploy/docker/cpr.Dockerfile"
        )

    def test_build_never_pushes_images_or_includes_git_credentials(self):
        command = build_command(
            "docker",
            Path("isolated-config"),
            "new-api",
            self.revision,
            "https://github.com/example/xinapi",
            self.target,
            "xinapi/new-api:test",
            "linux/amd64",
            "docker.io",
            Path("snapshot/context"),
            Path("snapshot/Dockerfile"),
            Path("receipt.json"),
        )
        self.assertIn("--load", command)
        self.assertNotIn("--push", command)
        self.assertNotIn("--secret", command)
        self.assertIn("IMAGE_REGISTRY=docker.io", command)
        self.assertIn("BUILD_JOBS=2", command)
        self.assertIn("org.opencontainers.image.revision=" + self.revision, command)
        self.assertEqual(command[-1], "snapshot/context")

    def test_unsupported_registry_and_tags_are_rejected(self):
        for registry, tag in (
            ("untrusted.example", "xinapi/test:1"),
            ("docker.io", "https://example.invalid/image:tag"),
        ):
            with (
                self.subTest(registry=registry, tag=tag),
                self.assertRaises(BuildError),
            ):
                build_command(
                    "docker",
                    None,
                    "new-api",
                    self.revision,
                    "https://github.com/example/xinapi",
                    self.target,
                    tag,
                    "linux/amd64",
                    registry,
                    Path("context"),
                    Path("Dockerfile"),
                    Path("metadata"),
                )

    def test_build_recipe_symlink_modes_are_rejected(self):
        path = "deploy/build-targets.json"
        oid = subprocess.check_output(
            ["git", "rev-parse", "HEAD:" + path], cwd=self.root, text=True
        ).strip()
        subprocess.run(
            ["git", "update-index", "--cacheinfo", "120000", oid, path],
            cwd=self.root,
            check=True,
        )
        subprocess.run(
            [
                "git",
                "-c",
                "user.name=build-test",
                "-c",
                "user.email=test@example.invalid",
                "commit",
                "-q",
                "-m",
                "invalid-mode-fixture",
            ],
            cwd=self.root,
            check=True,
        )
        with self.assertRaises(BuildError):
            read_target(self.root, resolve_revision(self.root, "HEAD"), "new-api")

    def test_failed_build_and_stale_loaded_tag_never_report_success(self):
        for build_exit in (7, 0):
            with self.subTest(build_exit=build_exit):
                binary_dir = self.root / ("bin-" + str(build_exit))
                binary_dir.mkdir()
                executable = binary_dir / "docker"
                executable.write_text(
                    "#!/usr/bin/env python3\nimport json,sys\n"
                    "if 'build' in sys.argv:\n    raise SystemExit("
                    + str(build_exit)
                    + ")\n"
                    "print(json.dumps([{'Id':'old-image','Config':{'Labels':None}}]))\n"
                )
                executable.chmod(0o755)
                output = (
                    self.root / ".runtime" / ("result-" + str(build_exit) + ".json")
                )
                with (
                    patch.dict(
                        os.environ,
                        {"PATH": str(binary_dir) + os.pathsep + os.environ["PATH"]},
                    ),
                    patch.object(
                        sys,
                        "argv",
                        ["build_image.py", "new-api", "--output", str(output)],
                    ),
                    contextlib.redirect_stdout(io.StringIO()),
                    contextlib.redirect_stderr(io.StringIO()),
                ):
                    result = main(self.root)
                self.assertNotEqual(result, 0)
                receipt = json.loads(output.read_text())
                self.assertEqual(receipt["result"], "failed")
                self.assertIsNone(receipt["image_id"])
                self.assertEqual(receipt["revision"], self.revision)
                self.assertFalse(receipt["runtime_service_started"])

    def test_cpr_target_has_explicit_runtime_and_resource_budget(self):
        target = {"target": "runtime"}
        command = build_command(
            "docker",
            None,
            "cpr",
            self.revision,
            "https://github.com/example/xinapi",
            target,
            "xinapi/cpr:test",
            "linux/amd64",
            "docker.m.daocloud.io",
            Path("context"),
            Path("Dockerfile"),
            Path("metadata"),
            "local-builder",
        )
        self.assertIn("runtime", command)
        self.assertIn("CARGO_BUILD_JOBS=2", command)
        self.assertIn("CPR_GIT_SHA=" + self.revision, command)
        self.assertIn("local-builder", command)


if __name__ == "__main__":
    unittest.main()
