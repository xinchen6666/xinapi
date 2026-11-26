"""Behavioral checks for immutable source import verification."""

import copy
import hashlib
import json
from pathlib import Path
import tempfile
import subprocess
import unittest

from tools.verify_upstream import GitFiles, VerificationError, object_hash, read_manifest, tree_hash, verify_component


class SourceVerificationTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.base = self.root / "services/new-api"
        self.base.mkdir(parents=True)
        data = b"unchanged upstream\n"
        (self.base / "input.txt").write_bytes(data)
        (self.base / "input.txt").chmod(0o644)
        entry = {"path": "input.txt", "mode": "100644", "size": len(data),
                 "sha256": hashlib.sha256(data).hexdigest(), "git_blob_sha": object_hash("blob", data)}
        self.pin = {"component": "new-api", "destination": "services/new-api",
                    "repository": "https://example.invalid/upstream", "commit": "b" * 40,
                    "tree": tree_hash([("input.txt", "100644", entry["git_blob_sha"])]),
                    "import_tree": tree_hash([("input.txt", "100644", entry["git_blob_sha"])]),
                    "gitlinks": {}}
        self.manifest = {"schema_version": 1, "component": "new-api", "destination": "services/new-api",
                         "import_tree_sha": self.pin["import_tree"], "file_count": 1, "files": [entry], "required_inputs": ["input.txt"],
                         "source": {"repository": self.pin["repository"], "commit": self.pin["commit"], "git_tree": self.pin["tree"]},
                         "gitlinks": []}

    def test_unchanged_source_is_accepted(self):
        self.assertEqual(verify_component(self.root, self.manifest, self.pin)["files"], 1)

    def test_tampered_bytes_and_missing_files_are_rejected(self):
        for content in (b"changed!! upstream\n", None):
            with self.subTest(content=content):
                path = self.base / "input.txt"
                if content is not None:
                    path.write_bytes(content)
                elif path.exists():
                    path.unlink()
                with self.assertRaises(VerificationError):
                    verify_component(self.root, self.manifest, self.pin)

    def test_executable_mode_and_extra_files_are_rejected(self):
        path = self.base / "input.txt"
        path.chmod(0o755)
        with self.assertRaisesRegex(VerificationError, "mode differs"):
            verify_component(self.root, self.manifest, self.pin)
        path.chmod(0o644)
        (self.base / "unexpected.txt").write_bytes(b"extra")
        with self.assertRaisesRegex(VerificationError, "inventory differs"):
            verify_component(self.root, self.manifest, self.pin)

    def test_nested_git_metadata_is_rejected(self):
        (self.base / ".git").mkdir()
        with self.assertRaisesRegex(VerificationError, "nested Git"):
            verify_component(self.root, self.manifest, self.pin)

    def test_optional_gitlink_must_be_explicitly_reviewed(self):
        manifest = copy.deepcopy(self.manifest)
        link = {"path": "modules/ui", "commit": "a" * 40,
                "disposition": "omitted_optional_source_development_gitlink"}
        manifest["gitlinks"] = [link]
        entry = manifest["files"][0]
        manifest["source"]["git_tree"] = tree_hash([("input.txt", "100644", entry["git_blob_sha"]),
                                                     (link["path"], "160000", link["commit"])])
        pin = copy.deepcopy(self.pin)
        pin["gitlinks"] = {link["path"]: link["commit"]}
        pin["tree"] = manifest["source"]["git_tree"]
        self.assertEqual(verify_component(self.root, manifest, pin)["optional_gitlinks_omitted"], 1)
        link["disposition"] = "unreviewed"
        with self.assertRaisesRegex(VerificationError, "unreviewed gitlink"):
            verify_component(self.root, manifest, pin)

    def test_path_collisions_and_traversal_are_rejected(self):
        for entries in ([('x', '100644', 'a' * 40), ('x', '100644', 'a' * 40)],
                        [('x', '100644', 'a' * 40), ('x/y', '100644', 'a' * 40)],
                        [('../x', '100644', 'a' * 40)]):
            with self.subTest(entries=entries), self.assertRaises(VerificationError):
                tree_hash(entries)

    def test_manifest_identity_cannot_redirect_validation(self):
        changes = [("component", "codex-proxy-rs"), ("destination", "services/codex-proxy-rs")]
        for field, value in changes:
            manifest = copy.deepcopy(self.manifest)
            manifest[field] = value
            with self.subTest(field=field), self.assertRaises(VerificationError):
                verify_component(self.root, manifest, self.pin)
        for field in ("repository", "commit", "git_tree"):
            manifest = copy.deepcopy(self.manifest)
            manifest["source"][field] = "changed"
            with self.subTest(field=field), self.assertRaises(VerificationError):
                verify_component(self.root, manifest, self.pin)

    def test_intent_to_add_empty_file_is_not_a_committable_source(self):
        subprocess.run(["git", "init", "-q"], cwd=self.root, check=True)
        empty = self.base / "VERSION"
        empty.touch()
        subprocess.run(["git", "add", "-N", "services/new-api/VERSION"], cwd=self.root, check=True)
        with self.assertRaisesRegex(VerificationError, "intent-to-add"):
            GitFiles(self.root)

    def test_intent_to_add_replacing_tracked_nonempty_file_is_rejected(self):
        subprocess.run(["git", "init", "-q"], cwd=self.root, check=True)
        path = self.base / "VERSION"
        path.write_bytes(b"old-version\n")
        subprocess.run(["git", "add", "services/new-api/VERSION"], cwd=self.root, check=True)
        subprocess.run(["git", "-c", "user.name=source-verifier-test", "-c", "user.email=test@example.invalid",
                        "commit", "-q", "-m", "fixture"], cwd=self.root, check=True)
        subprocess.run(["git", "rm", "--cached", "-q", "services/new-api/VERSION"], cwd=self.root, check=True)
        path.write_bytes(b"")
        subprocess.run(["git", "add", "-N", "services/new-api/VERSION"], cwd=self.root, check=True)
        with self.assertRaisesRegex(VerificationError, "intent-to-add"):
            GitFiles(self.root)

    def test_group_execute_bit_matches_git_owner_bit_rule(self):
        path = self.base / "input.txt"
        path.chmod(0o654)
        self.assertEqual(verify_component(self.root, self.manifest, self.pin)["files"], 1)
        manifest = copy.deepcopy(self.manifest)
        manifest["files"][0]["mode"] = "100755"
        with self.assertRaisesRegex(VerificationError, "mode differs"):
            verify_component(self.root, manifest, self.pin)

    def test_source_directory_links_are_rejected(self):
        for linked_path in ("services/new-api", "services"):
            with self.subTest(linked_path=linked_path), tempfile.TemporaryDirectory() as directory:
                root = Path(directory) / "project"
                external = Path(directory) / "external"
                if linked_path == "services/new-api":
                    (root / "services").mkdir(parents=True)
                    external.mkdir()
                    source = external
                else:
                    root.mkdir()
                    (external / "new-api").mkdir(parents=True)
                    source = external / "new-api"
                (source / "input.txt").write_bytes(b"unchanged upstream\n")
                (source / "input.txt").chmod(0o644)
                (root / linked_path).symlink_to(external, target_is_directory=True)
                with self.assertRaises(VerificationError):
                    verify_component(root, self.manifest, self.pin)

    def test_manifest_worktree_file_and_parent_links_are_rejected(self):
        for link_kind in ("manifest", "directory"):
            with self.subTest(link_kind=link_kind), tempfile.TemporaryDirectory() as directory:
                root = Path(directory) / "project"
                root.mkdir()
                external = Path(directory) / "external"
                external.mkdir()
                target = external / "manifest.json"
                target.write_text(json.dumps(self.manifest))
                if link_kind == "manifest":
                    (root / "third_party").mkdir()
                    (root / "third_party/manifest.json").symlink_to(target)
                else:
                    (root / "third_party").symlink_to(external, target_is_directory=True)
                with self.assertRaises(VerificationError):
                    read_manifest(root, "third_party/manifest.json")

    def test_manifest_worktree_requires_a_regular_json_object(self):
        directory = self.root / "third_party"
        directory.mkdir()
        path = directory / "manifest.json"
        path.write_text(json.dumps(self.manifest))
        path.chmod(0o644)
        self.assertEqual(read_manifest(self.root, "third_party/manifest.json"), self.manifest)
        path.chmod(0o755)
        with self.assertRaises(VerificationError):
            read_manifest(self.root, "third_party/manifest.json")
        path.chmod(0o644)
        path.write_text("[]")
        with self.assertRaises(VerificationError):
            read_manifest(self.root, "third_party/manifest.json")

    def test_manifest_git_modes_are_checked_in_index_and_history(self):
        subprocess.run(["git", "init", "-q"], cwd=self.root, check=True)
        relative = "third_party/manifest.json"
        oid = subprocess.check_output(["git", "hash-object", "-w", "--stdin"], cwd=self.root,
                                      input=json.dumps(self.manifest).encode()).decode().strip()
        for mode in ("100644", "120000", "100755"):
            subprocess.run(["git", "update-index", "--add", "--cacheinfo", mode, oid, relative],
                           cwd=self.root, check=True)
            tree = subprocess.check_output(["git", "write-tree"], cwd=self.root, text=True).strip()
            for revision in (None, tree):
                with self.subTest(mode=mode, historical=revision is not None):
                    reader = GitFiles(self.root, revision)
                    try:
                        if mode == "100644":
                            self.assertEqual(read_manifest(self.root, relative, reader), self.manifest)
                        else:
                            with self.assertRaises(VerificationError):
                                read_manifest(self.root, relative, reader)
                    finally:
                        reader.close()

    def test_index_and_history_do_not_read_changed_worktree_files(self):
        subprocess.run(["git", "init", "-q"], cwd=self.root, check=True)
        directory = self.root / "third_party"
        directory.mkdir()
        relative = "third_party/manifest.json"
        path = self.root / relative
        path.write_text(json.dumps(self.manifest))
        path.chmod(0o644)
        subprocess.run(["git", "add", "services", "third_party"], cwd=self.root, check=True)
        tree = subprocess.check_output(["git", "write-tree"], cwd=self.root, text=True).strip()
        path.write_text("invalid worktree JSON")
        (self.base / "input.txt").write_bytes(b"modified but not staged")
        for revision in (None, tree):
            reader = GitFiles(self.root, revision)
            try:
                manifest = read_manifest(self.root, relative, reader)
                self.assertEqual(verify_component(self.root, manifest, self.pin, reader)["files"], 1)
            finally:
                reader.close()
        subprocess.run(["git", "add", "services/new-api/input.txt"], cwd=self.root, check=True)
        reader = GitFiles(self.root)
        try:
            with self.assertRaises(VerificationError):
                verify_component(self.root, read_manifest(self.root, relative, reader), self.pin, reader)
        finally:
            reader.close()
        reader = GitFiles(self.root, tree)
        try:
            self.assertEqual(verify_component(self.root, read_manifest(self.root, relative, reader),
                                             self.pin, reader)["files"], 1)
        finally:
            reader.close()

    def test_symlink_outside_source_is_rejected(self):
        outside = self.root / 'outside.txt'
        outside.write_bytes(b'external')
        path = self.base / 'input.txt'
        path.unlink()
        path.symlink_to('../../outside.txt')
        with self.assertRaises(VerificationError):
            verify_component(self.root, self.manifest, self.pin)


if __name__ == "__main__":
    unittest.main()
