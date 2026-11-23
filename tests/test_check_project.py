"""Regression tests for Git-only project checks; all credentials are generated fixtures."""

import contextlib
from dataclasses import replace
import io
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

from tools.check_project import (
    CheckError,
    Snapshot,
    check_project,
    load_snapshot,
    main,
    resolve_ref,
)


def fixture(extra=None):
    files = {
        "README.md": b"# Project\n",
        "AGENTS.md": b"# Work rules\n",
        "docs/PLAN.md": "\n".join(
            f"### X{i:02d} Step\n**前置／范围：** none\n" for i in range(1, 19)
        ).encode(),
        "docs/STATUS.md": "\n".join(
            f"| X{i:02d} | pending | owner | result |" for i in range(1, 19)
        ).encode(),
    }
    files.update(extra or {})
    return Snapshot("a" * 40, files, {name: "100644" for name in files})


def categories(report):
    return {issue["category"] for issue in report["issues"]}


class ProjectChecksTests(unittest.TestCase):
    def test_links_to_markdown_json_code_images_and_explicit_anchors(self):
        current = fixture(
            {
                "README.md": b"[Guide](docs/guide.md#named) [JSON](docs/data.json) [code](tools/sample.py#L1-L2)\n"
                b"[space](<docs/with space.md>) ![Image](assets/image.png) [ref][guide]\n"
                b"[guide]: docs/guide.md#topic-1\n",
                "docs/guide.md": b'# Topic\n<a id="named"></a>\n# Topic\n',
                "docs/with space.md": b"# Space\n",
                "docs/data.json": b'{"value": true}\n',
                "tools/sample.py": b"line1\nline2\n",
                "assets/image.png": b"\x89PNG\x00fixture",
                "services/new-api/docs/external.md": b"[not our link check](missing-upstream.md)\n",
            }
        )
        report = check_project(current, fixture())
        self.assertTrue(report["ok"], report["issues"])
        self.assertEqual(report["counts"]["upstream_markdown_not_link_checked"], 1)

    def test_missing_links_anchors_and_invalid_line_ranges_are_rejected(self):
        cases = {
            "[missing](missing.md)": "missing-link-target",
            "[heading](docs/target.md#missing)": "missing-heading-anchor",
            "[line](docs/target.md#L8)": "invalid-line-anchor",
            "[range](docs/target.md#L2-L1)": "invalid-line-anchor",
            "[outside](../../not-project.md)": "link-outside-repository",
            "[local](/data/local.md)": "nonportable-link",
            "[reference][missing]": "missing-link-reference",
        }
        for text, category in cases.items():
            with self.subTest(text=text):
                current = fixture(
                    {
                        "README.md": text.encode(),
                        "docs/target.md": b"# Heading\nsecond\n",
                    }
                )
                self.assertIn(category, categories(check_project(current, fixture())))

    def test_fenced_examples_are_not_link_targets_but_still_scanned_for_secrets(self):
        report = check_project(
            fixture({"README.md": b"```text\n[example](missing.md)\n```\n"}), fixture()
        )
        self.assertTrue(report["ok"], report["issues"])
        report = check_project(
            fixture({"README.md": b"~~~text\nnot closed\n"}), fixture()
        )
        self.assertIn("unclosed-code-fence", categories(report))
        secret = "ghp_" + "F" * 36
        report = check_project(
            fixture({"README.md": f"```\n{secret}\n```\n".encode()}), fixture()
        )
        self.assertIn("github-token", categories(report))
        self.assertNotIn(secret, json.dumps(report))

    def test_only_identical_audited_matches_at_the_same_source_path_are_allowed(self):
        path = "services/new-api/example.txt"
        secret = "ghp_" + "A" * 36
        baseline = fixture({path: secret.encode()})
        report = check_project(fixture({path: secret.encode()}), baseline)
        self.assertTrue(report["ok"], report["issues"])
        self.assertEqual(report["counts"]["reviewed_baseline_matches"], 1)
        changes = [
            {path: (secret + "\n" + secret).encode()},
            {"services/new-api/new-file.txt": secret.encode()},
            {path: ("ghp_" + "B" * 36).encode()},
        ]
        for extra in changes:
            with self.subTest(extra=tuple(extra)):
                report = check_project(fixture(extra), baseline)
                self.assertIn("github-token", categories(report))
                self.assertNotIn(secret, json.dumps(report))
        report = check_project(
            fixture({"README.md": secret.encode()}),
            fixture({"README.md": secret.encode()}),
        )
        self.assertIn("github-token", categories(report))

    def test_new_credentials_in_other_upstream_files_are_not_bulk_excluded(self):
        secret = "sk-" + "N" * 40
        report = check_project(
            fixture({"services/codex-proxy-rs/new.rs": secret.encode()}), fixture()
        )
        self.assertIn("api-key", categories(report))
        self.assertNotIn(secret, json.dumps(report))

    def test_subscription_url_tokens_are_not_printed(self):
        url = "https://example.invalid/subscribe?token=" + "T" * 32
        report = check_project(
            fixture({"services/new-api/new-config.txt": url.encode()}), fixture()
        )
        self.assertIn("secret-url-query", categories(report))
        self.assertNotIn(url, json.dumps(report))

    def test_private_key_entity_and_credential_url_are_reported_without_values(self):
        entity = (
            "-----"
            + "BEGIN PRIVATE KEY"
            + "-----\n"
            + "A" * 100
            + "\n"
            + "-----"
            + "END PRIVATE KEY"
            + "-----\n"
        )
        url = "https://" + "operator:" + "fixture-" + "Z" * 24 + "@example.invalid/path"
        report = check_project(
            fixture({"docs/demo.md": (entity + url).encode()}), fixture()
        )
        self.assertIn("private-key", categories(report))
        self.assertIn("credential-url", categories(report))
        self.assertNotIn(entity, json.dumps(report))
        self.assertNotIn(url, json.dumps(report))
        self.assertTrue(
            all(set(item) == {"path", "line", "category"} for item in report["issues"])
        )

    def test_tasks_require_complete_definitions_valid_status_and_worklog(self):
        good = fixture()
        invalid = {
            "docs/PLAN.md": good.files["docs/PLAN.md"].replace(b"### X18", b"### X19"),
            "docs/STATUS.md": good.files["docs/STATUS.md"].replace(
                b"| X01 | pending |", b"| X01 | guessed |"
            ),
        }
        report = check_project(fixture(invalid), fixture())
        self.assertIn("task-definitions-not-x01-x18", categories(report))
        self.assertIn("invalid-task-status", categories(report))
        state = good.files["docs/STATUS.md"].replace(
            b"| X01 | pending |", b"| X01 | done |"
        )
        self.assertIn(
            "completed-task-missing-worklog",
            categories(check_project(fixture({"docs/STATUS.md": state}), fixture())),
        )

    def test_forward_dependencies_and_unmet_completed_dependencies_are_rejected(self):
        plan = (
            fixture()
            .files["docs/PLAN.md"]
            .replace(
                "**前置／范围：** none".encode(), "**前置／范围：** X02".encode(), 1
            )
        )
        self.assertIn(
            "invalid-task-dependency",
            categories(check_project(fixture({"docs/PLAN.md": plan}), fixture())),
        )
        plan = (
            fixture()
            .files["docs/PLAN.md"]
            .replace(
                "### X02 Step\n**前置／范围：** none".encode(),
                "### X02 Step\n**前置／范围：** X01".encode(),
            )
        )
        state = (
            fixture()
            .files["docs/STATUS.md"]
            .replace(b"| X02 | pending |", b"| X02 | done |")
        )
        report = check_project(
            fixture(
                {
                    "docs/PLAN.md": plan,
                    "docs/STATUS.md": state,
                    "docs/worklog/X02.md": b"# Record\n",
                }
            ),
            fixture(),
        )
        self.assertIn("completed-task-dependency-not-done", categories(report))

    def test_gitlinks_are_rejected(self):
        current = fixture()
        current.modes["services/accidental-submodule"] = "160000"
        self.assertIn("nested-gitlink", categories(check_project(current, fixture())))


class GitSnapshotTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.git("init", "-q")
        for path, content in fixture().files.items():
            file = self.root / path
            file.parent.mkdir(parents=True, exist_ok=True)
            file.write_bytes(content)
        self.git("add", ".")
        self.commit = self.save("baseline")

    def git(self, *args):
        return subprocess.check_output(
            ["git", *args], cwd=self.root, stderr=subprocess.DEVNULL, text=True
        ).strip()

    def save(self, message):
        self.git(
            "-c",
            "user.name=check-project-test",
            "-c",
            "user.email=test@example.invalid",
            "commit",
            "-q",
            "-m",
            message,
        )
        return self.git("rev-parse", "HEAD")

    def test_only_committed_files_are_read_and_ref_options_cannot_escape(self):
        secret = "ghp_" + "W" * 36
        (self.root / "README.md").write_text(secret)
        (self.root / "untracked-secret.txt").write_text(secret)
        snapshot = load_snapshot(self.root, "HEAD")
        self.assertEqual(snapshot.files["README.md"], b"# Project\n")
        self.assertNotIn("untracked-secret.txt", snapshot.files)
        for ref in ("--help", "--output=unwanted", "HEAD;touch unwanted"):
            with self.subTest(ref=ref), self.assertRaises(CheckError):
                resolve_ref(self.root, ref)
        self.assertFalse((self.root / "unwanted").exists())

    def test_cli_rejects_new_secret_and_does_not_print_it(self):
        secret = "ghp_" + "Q" * 36
        path = self.root / "services/new-api/new-fixture.txt"
        path.parent.mkdir(parents=True)
        path.write_text(secret)
        self.git("add", "services")
        current = self.save("new fixture")
        output = io.StringIO()
        with (
            patch("tools.check_project.AUDITED_BASELINE", self.commit),
            contextlib.redirect_stdout(output),
        ):
            result = main(["--git-ref", current], self.root)
        self.assertEqual(result, 1)
        report = json.loads(output.getvalue())
        self.assertIn("github-token", categories(report))
        self.assertNotIn(secret, output.getvalue())

    def test_invalid_ref_and_bounded_blob_fail_safely(self):
        output = io.StringIO()
        supplied = "--output=" + "ghp_" + "R" * 36
        with (
            patch("tools.check_project.AUDITED_BASELINE", self.commit),
            contextlib.redirect_stdout(output),
        ):
            result = main(["--git-ref=" + supplied], self.root)
        self.assertEqual(result, 1)
        self.assertIn("invalid-git-ref", categories(json.loads(output.getvalue())))
        self.assertNotIn(supplied, output.getvalue())
        with (
            patch("tools.check_project.MAX_BLOB_BYTES", 3),
            self.assertRaises(CheckError),
        ):
            load_snapshot(self.root, "HEAD")

    def test_cli_baseline_cannot_be_changed_to_new_head(self):
        (self.root / "new.txt").write_text("new source\n")
        self.git("add", "new.txt")
        self.save("later")
        output = io.StringIO()
        with (
            patch("tools.check_project.AUDITED_BASELINE", self.commit),
            contextlib.redirect_stdout(output),
        ):
            result = main(["--baseline", "HEAD"], self.root)
        self.assertEqual(result, 1)
        self.assertIn("baseline-not-audited", categories(json.loads(output.getvalue())))


if __name__ == "__main__":
    unittest.main()
