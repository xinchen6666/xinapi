#!/usr/bin/env python3
"""Check committed project documents, task structure and credential signatures.

This is a bounded Markdown checker and signature scan, not a security guarantee
or proof of build/runtime completion. It never scans uncommitted worktree data.
"""

import argparse
from collections import Counter
from dataclasses import dataclass
import html
import json
from pathlib import Path
import posixpath
import re
import subprocess
import sys
from urllib.parse import unquote, urlsplit

AUDITED_BASELINE = "072e434a528bb9295d9c6d4ee6000c91b03485ca"
MAX_BLOB_BYTES = 8 * 1024 * 1024
TASK_IDS = tuple(f"X{i:02d}" for i in range(1, 19))
SIGNATURES = {
    "github-token": re.compile(
        r"\b(?:gh[pousr]_[A-Za-z0-9]{24,}|github_pat_[A-Za-z0-9_]{24,})"
    ),
    "api-key": re.compile(r"\bsk[-_](?:proj-)?[A-Za-z0-9_-]{28,}"),
    "google-api-key": re.compile(r"\bAIza[A-Za-z0-9_-]{30,}"),
    "jwt": re.compile(
        r"\beyJ[A-Za-z0-9_-]{8,}\.[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}"
    ),
    "private-key": re.compile(
        r"-----BEGIN (?:RSA |EC |DSA |OPENSSH |ENCRYPTED )?PRIVATE KEY-----\s+"
        r"[A-Za-z0-9+/=\r\n\t ]{80,}-----END (?:RSA |EC |DSA |OPENSSH |ENCRYPTED )?PRIVATE KEY-----"
    ),
    "credential-url": re.compile(
        r"(?i)\b(?:https?|socks5h?|postgres(?:ql)?|redis|mysql|mongodb(?:\+srv)?)://"
        r"[^\s/:@<>\"'`]*:[^\s/@<>\"'`]{3,}@"
    ),
    "secret-url-query": re.compile(
        r"(?i)\bhttps?://[^\s<>\"'`]*[?&](?:access_token|refresh_token|token|api_key|key|auth)="
        r"[A-Za-z0-9_~+/%=-]{16,}"
    ),
}
INLINE_LINK = re.compile(
    r"!?\[[^\]\n]*\]\(\s*(?:<([^>\n]+)>|([^\s)\n]+))(?:\s+[\"'][^\n]*?[\"'])?\s*\)"
)
REFERENCE_LINK = re.compile(r"!?\[([^\]\n]+)\]\[([^\]\n]*)\]")
REFERENCE_DEF = re.compile(r"^ {0,3}\[([^\]\n]+)\]:\s*(?:<([^>\n]+)>|([^\s\n]+))")
FENCE = re.compile(r"^ {0,3}(`{3,}|~{3,})(.*)$")
EXPLICIT_ANCHOR = re.compile(r"\b(?:id|name)\s*=\s*[\"']([^\"']+)[\"']", re.I)
LINE_ANCHOR = re.compile(r"L([1-9]\d*)(?:-L?([1-9]\d*))?")


class CheckError(Exception):
    pass


@dataclass
class Snapshot:
    tree: str
    files: dict
    modes: dict


def resolve_ref(root, ref, kind="tree"):
    try:
        value = subprocess.check_output(
            [
                "git",
                "rev-parse",
                "--verify",
                "--end-of-options",
                ref + "^{" + kind + "}",
            ],
            cwd=root,
            stderr=subprocess.DEVNULL,
            text=True,
        ).strip()
    except (subprocess.SubprocessError, OSError):
        raise CheckError("invalid-git-ref") from None
    if not re.fullmatch(r"[0-9a-f]{40}|[0-9a-f]{64}", value):
        raise CheckError("invalid-git-object")
    return value


def load_snapshot(root, ref):
    tree = resolve_ref(root, ref)
    try:
        raw = subprocess.check_output(
            ["git", "ls-tree", "-r", "-l", "-z", tree],
            cwd=root,
            stderr=subprocess.DEVNULL,
        )
    except (subprocess.SubprocessError, OSError):
        raise CheckError("unreadable-git-tree") from None
    entries = []
    modes = {}
    for record in raw.split(b"\0"):
        if not record:
            continue
        header, name = record.split(b"\t", 1)
        mode, kind, oid, size = header.decode().split()
        path = name.decode("utf-8")
        if path in modes:
            raise CheckError("duplicate-git-path")
        modes[path] = mode
        if kind == "blob":
            if int(size) > MAX_BLOB_BYTES:
                raise CheckError("git-blob-size-limit")
            entries.append((path, oid))
        elif kind != "commit":
            raise CheckError("unexpected-git-entry")
    files = {}
    process = subprocess.Popen(
        ["git", "cat-file", "--batch"],
        cwd=root,
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
    )
    try:
        for path, oid in entries:
            process.stdin.write((oid + "\n").encode())
            process.stdin.flush()
            header = process.stdout.readline().split()
            if len(header) != 3 or header[0].decode() != oid or header[1] != b"blob":
                raise CheckError("unreadable-git-blob")
            size = int(header[2])
            if size > MAX_BLOB_BYTES:
                raise CheckError("git-blob-size-limit")
            data = process.stdout.read(size)
            if len(data) != size or process.stdout.read(1) != b"\n":
                raise CheckError("truncated-git-blob")
            files[path] = data
    finally:
        process.stdin.close()
        process.stdout.close()
        if process.wait() != 0:
            raise CheckError("git-object-reader-failed")
    return Snapshot(tree, files, modes)


def project_document(path):
    return path in ("README.md", "AGENTS.md") or (
        path.endswith(".md")
        and (
            path.startswith("docs/")
            or path.startswith("third_party/")
            or (path.startswith("research/") and path.count("/") == 1)
        )
    )


def prose_lines(text):
    """Keep line positions while excluding fenced examples from link checks."""
    lines = []
    fence = None
    opened_at = None
    for number, line in enumerate(text.splitlines(), 1):
        match = FENCE.match(line)
        if fence:
            if (
                match
                and match[1][0] == fence[0]
                and len(match[1]) >= len(fence)
                and not match[2].strip()
            ):
                fence = None
            lines.append("")
        elif match:
            fence = match[1]
            opened_at = number
            lines.append("")
        else:
            lines.append(line)
    return lines, opened_at if fence else None


def heading_slug(heading):
    heading = re.sub(r"\[([^\]]+)\]\([^)]+\)", r"\1", heading)
    heading = re.sub(r"<[^>]*>", "", heading)
    heading = html.unescape(heading).strip().lower()
    return re.sub(r"\s+", "-", re.sub(r"[^\w\-\s]", "", heading))


def anchor_ids(lines):
    found = set()
    headings = set()
    for line in lines:
        found.update(EXPLICIT_ANCHOR.findall(line))
        match = re.match(r"^ {0,3}#{1,6}\s+(.+?)\s*#*\s*$", line)
        if match:
            slug = heading_slug(match[1])
            candidate = slug
            count = 0
            while candidate in headings:
                count += 1
                candidate = f"{slug}-{count}"
            headings.add(candidate)
    return found | headings


def check_project(snapshot, baseline):
    issues = []
    counts = Counter()

    def issue(path, line, category):
        issues.append({"path": path, "line": line, "category": category})

    # Scan even code blocks and upstream services. Exempt only the same reviewed
    # signature at the same upstream path, respecting its original occurrence count.
    for path, mode in sorted(snapshot.modes.items()):
        if mode == "160000":
            issue(path, 1, "nested-gitlink")
    for path, data in sorted(snapshot.files.items()):
        counts["files_scanned"] += 1
        text = data.decode("utf-8", "replace")
        approved = Counter()
        if path.startswith(("services/new-api/", "services/codex-proxy-rs/")):
            old = baseline.files.get(path, b"").decode("utf-8", "replace")
            for category, pattern in SIGNATURES.items():
                approved.update((category, m[0]) for m in pattern.finditer(old))
        for category, pattern in SIGNATURES.items():
            for match in pattern.finditer(text):
                counts["signature_matches"] += 1
                key = (category, match[0])
                if approved[key]:
                    approved[key] -= 1
                    counts["reviewed_baseline_matches"] += 1
                else:
                    issue(path, text.count("\n", 0, match.start()) + 1, category)

    documents = {}
    for path in sorted(snapshot.files):
        if not project_document(path):
            if path.startswith("services/") and path.endswith(".md"):
                counts["upstream_markdown_not_link_checked"] += 1
            continue
        counts["documents_checked"] += 1
        if snapshot.modes.get(path) not in ("100644", "100755"):
            issue(path, 1, "document-not-regular")
            continue
        try:
            text = snapshot.files[path].decode("utf-8")
        except UnicodeDecodeError:
            issue(path, 1, "document-not-utf8")
            continue
        lines, unclosed = prose_lines(text)
        if unclosed:
            issue(path, unclosed, "unclosed-code-fence")
        documents[path] = (lines, anchor_ids(lines))
    directories = {posixpath.dirname(path) for path in snapshot.modes}
    directories |= {
        parent for path in tuple(directories) for parent in parents_of(path)
    }
    for path, (lines, _) in documents.items():
        references = {}
        targets = []
        for number, line in enumerate(lines, 1):
            definition = REFERENCE_DEF.match(line)
            if definition:
                references[definition[1].strip().casefold()] = (
                    definition[2] or definition[3]
                )
                targets.append((number, definition[2] or definition[3]))
            else:
                line = re.sub(r"`+[^`]*`+", "", line)
                targets.extend(
                    (number, match[1] or match[2])
                    for match in INLINE_LINK.finditer(line)
                )
        for number, line in enumerate(lines, 1):
            for match in REFERENCE_LINK.finditer(re.sub(r"`+[^`]*`+", "", line)):
                label = (match[2] or match[1]).strip().casefold()
                if label not in references:
                    issue(path, number, "missing-link-reference")
        for number, target in targets:
            try:
                parsed = urlsplit(html.unescape(target))
            except ValueError:
                issue(path, number, "invalid-link")
                continue
            if parsed.scheme in ("https", "http", "mailto") or parsed.netloc:
                continue
            counts["internal_links_checked"] += 1
            if parsed.scheme or parsed.path.startswith("/") or "\\" in parsed.path:
                issue(path, number, "nonportable-link")
                continue
            decoded = unquote(parsed.path)
            if decoded.startswith("/") or "\\" in decoded:
                issue(path, number, "nonportable-link")
                continue
            destination = (
                posixpath.normpath(posixpath.join(posixpath.dirname(path), decoded))
                if decoded
                else path
            )
            if destination == ".." or destination.startswith("../"):
                issue(path, number, "link-outside-repository")
                continue
            if (
                destination not in snapshot.modes
                and destination not in directories
                and destination != "."
            ):
                issue(path, number, "missing-link-target")
                continue
            fragment = unquote(parsed.fragment)
            if not fragment:
                continue
            line_anchor = LINE_ANCHOR.fullmatch(fragment)
            if line_anchor:
                target_bytes = snapshot.files.get(destination, b"")
                target_lines = len(target_bytes.decode("utf-8", "replace").splitlines())
                start = int(line_anchor[1])
                end = int(line_anchor[2] or start)
                if start > end or end > target_lines:
                    issue(path, number, "invalid-line-anchor")
            elif destination.endswith(".md") and destination in snapshot.files:
                if destination in documents:
                    ids = documents[destination][1]
                else:
                    target_lines, _ = prose_lines(
                        snapshot.files[destination].decode("utf-8", "replace")
                    )
                    ids = anchor_ids(target_lines)
                if fragment not in ids:
                    issue(path, number, "missing-heading-anchor")
    check_tasks(documents, snapshot.modes, issue, counts)
    return {
        "ok": not issues,
        "tree": snapshot.tree,
        "baseline_tree": baseline.tree,
        "counts": dict(sorted(counts.items())),
        "issues": issues,
        "scope": "Committed files; project Markdown links; structural task checks; all-file signature scan",
        "limitations": [
            "Not a full Markdown parser or exhaustive credential detector",
            "Reviewed signature exemptions apply only to identical upstream matches at the audited path",
            "No network, worktree content, upstream execution or build/runtime completion verification",
        ],
    }


def parents_of(path):
    while path not in ("", "."):
        path = posixpath.dirname(path)
        if path:
            yield path


def check_tasks(documents, tracked, issue, counts):
    plan = documents.get("docs/PLAN.md")
    status = documents.get("docs/STATUS.md")
    if not plan or not status:
        for path in ("docs/PLAN.md", "docs/STATUS.md"):
            if path not in documents:
                issue(path, 1, "missing-task-document")
        return
    definitions = [
        (number, m[1])
        for number, line in enumerate(plan[0], 1)
        if (m := re.match(r"^###\s+(X\d{2})\b", line))
    ]
    if [task for _, task in definitions] != list(TASK_IDS):
        issue("docs/PLAN.md", 1, "task-definitions-not-x01-x18")
    rows = []
    for number, line in enumerate(status[0], 1):
        match = re.match(r"^\|\s*(X\d{2})\s*\|\s*([^|]+?)\s*\|", line)
        if match:
            rows.append((number, match[1], match[2]))
    if [task for _, task, _ in rows] != list(TASK_IDS):
        issue("docs/STATUS.md", 1, "task-statuses-not-x01-x18")
    states = {task: state for _, task, state in rows}
    for number, task, state in rows:
        if state not in ("pending", "in_progress", "blocked", "done"):
            issue("docs/STATUS.md", number, "invalid-task-status")
        if state == "done" and f"docs/worklog/{task}.md" not in tracked:
            issue("docs/STATUS.md", number, "completed-task-missing-worklog")
    for index, (line_number, task) in enumerate(definitions):
        end = (
            definitions[index + 1][0] - 1
            if index + 1 < len(definitions)
            else len(plan[0])
        )
        for number in range(line_number, end):
            if not plan[0][number].startswith("**前置／范围"):
                continue
            line = plan[0][number]
            dependencies = set(re.findall(r"X\d{2}", line))
            for match in re.finditer(r"X(\d{2})[–—-]X(\d{2})", line):
                first, last = map(int, match.groups())
                if first <= last:
                    dependencies.update(f"X{i:02d}" for i in range(first, last + 1))
                else:
                    issue("docs/PLAN.md", number + 1, "invalid-task-dependency-range")
            for dependency in dependencies:
                if dependency not in TASK_IDS or dependency >= task:
                    issue("docs/PLAN.md", number + 1, "invalid-task-dependency")
                elif states.get(task) == "done" and states.get(dependency) != "done":
                    issue("docs/STATUS.md", 1, "completed-task-dependency-not-done")
    counts["task_definitions"] = len(definitions)
    counts["task_statuses"] = len(rows)


def main(argv=None, root=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--git-ref",
        default="HEAD",
        help="commit/tree to check; uncommitted files are ignored",
    )
    parser.add_argument(
        "--baseline",
        default=AUDITED_BASELINE,
        help="must resolve to the fixed audited source commit",
    )
    args = parser.parse_args(argv)
    root = Path(root) if root is not None else Path(__file__).resolve().parents[1]
    try:
        baseline_commit = resolve_ref(root, args.baseline, "commit")
        if baseline_commit != AUDITED_BASELINE:
            raise CheckError("baseline-not-audited")
        baseline = load_snapshot(root, baseline_commit)
        target = load_snapshot(root, args.git_ref)
        report = check_project(target, baseline)
    except CheckError as error:
        report = {
            "ok": False,
            "issues": [{"path": "(git)", "line": None, "category": str(error)}],
            "limitations": [
                "Nothing was executed or fetched; input and Git diagnostic content are not printed"
            ],
        }
    except (OSError, ValueError, subprocess.SubprocessError):
        report = {
            "ok": False,
            "issues": [
                {"path": "(git)", "line": None, "category": "git-snapshot-error"}
            ],
            "limitations": [
                "Nothing was executed or fetched; diagnostic content is suppressed to avoid leaking credentials"
            ],
        }
    print(json.dumps(report, ensure_ascii=False, sort_keys=True))
    return 0 if report["ok"] else 1


if __name__ == "__main__":
    sys.exit(main())
