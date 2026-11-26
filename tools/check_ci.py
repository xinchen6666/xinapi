"""Select CI work and check the checkout without historical verification receipts."""

from collections import Counter
import json
import os
from pathlib import Path
import subprocess
import sys

from check_project import MAX_BLOB_BYTES, SIGNATURES, check_documentation
from select_checks import affected_targets


REVIEWED_SOURCE = "refs/remotes/origin/task/X11-admin-oauth"
PUBLIC_ENV = "services/codex-proxy-rs/frontend/.env"
SHARED_INPUTS = {".github/workflows/x03.yml", "deploy/build-targets.json"}


def git(*args):
    return subprocess.check_output(["git", *args], stderr=subprocess.DEVNULL)


def paths_at(*args):
    return git(*args).decode("utf-8").split("\0")[:-1]


def base_ref():
    base = os.environ.get("CI_BASE", "")
    if base.strip("0"):
        return base
    if (
        subprocess.run(
            ["git", "rev-parse", "--verify", "--quiet", "HEAD^"],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        ).returncode
        == 0
    ):
        return "HEAD^"
    return ""


def changed_paths(base):
    if not base:
        return paths_at("ls-files", "-z")
    return paths_at(
        "diff",
        "--name-only",
        "--no-renames",
        "-z",
        "--end-of-options",
        base,
        "HEAD",
        "--",
    )


def signature_base(base):
    if os.environ.get("CI_STACK_CONSOLIDATION") != "true" or not base:
        return base
    if paths_at("ls-tree", "--name-only", "-z", base, "--", "services"):
        return base
    for ancestor, descendant in ((base, REVIEWED_SOURCE), (REVIEWED_SOURCE, "HEAD")):
        if subprocess.run(
            ["git", "merge-base", "--is-ancestor", "--", ancestor, descendant],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        ).returncode:
            return base
    print("Credential scan covers changes since the reviewed X11 branch.")
    return REVIEWED_SOURCE


def select():
    dispatch = os.environ.get("GITHUB_EVENT_NAME") == "workflow_dispatch"
    paths = changed_paths(base_ref()) if not dispatch else []
    inputs = [
        p
        for p in paths
        if (not p.startswith(".github/") or p in SHARED_INPUTS)
        and p != "tools/build_image.py"
        and (
            not p.lower().endswith(".md")
            or Path(p)
            .name.upper()
            .startswith(("LICENSE", "NOTICE", "THIRD-PARTY-LICENSES"))
        )
    ]
    targets = affected_targets(inputs)
    if dispatch:
        targets = dict.fromkeys(targets, True)
    outputs = {
        "new_api": targets["new-api"],
        "new_api_web": any(p.startswith("services/new-api/web/") for p in inputs),
        "cpr": targets["cpr"],
        "cpr_web": any(
            p.startswith("services/codex-proxy-rs/frontend/") for p in inputs
        ),
        "compose": any(
            p in {"compose.yaml", "tools/init_compose.py"}
            or p.startswith("deploy/compose/")
            for p in paths
        ),
        "images": [component for component, selected in targets.items() if selected],
    }
    with open(os.environ["GITHUB_OUTPUT"], "a", encoding="utf-8") as output:
        for key, value in outputs.items():
            output.write(f"{key}={json.dumps(value, separators=(',', ':'))}\n")
    print(json.dumps(outputs, sort_keys=True))


def check():
    issues = []
    counts = Counter()

    def issue(path, line, category):
        issues.append({"path": path, "line": line, "category": category})

    base = base_ref()
    component_paths = set(changed_paths(base))
    credential_base = signature_base(base)
    changed = (
        component_paths
        if credential_base == base
        else set(changed_paths(credential_base))
    )
    modes = dict(
        (path, mode)
        for mode, path in (
            record.split(" ", 1)
            for record in paths_at("ls-files", "-z", "--format=%(objectmode) %(path)")
        )
    )
    ignored = set(paths_at("ls-files", "-ci", "-z", "--exclude-from=.gitignore"))
    for path in sorted(ignored - {PUBLIC_ENV}):
        issue(path, 1, "private-or-generated-path")
    files = {}
    for name, mode in modes.items():
        path = Path(name)
        if mode == "160000":
            issue(name, 1, "nested-gitlink")
        elif mode == "120000":
            if name in changed:
                issue(name, 1, "tracked-symlink")
        elif name not in ignored or name == PUBLIC_ENV:
            if path.stat().st_size <= MAX_BLOB_BYTES:
                files[name] = path.read_bytes()
            elif name in changed:
                issue(name, 1, "file-size-limit")
    if PUBLIC_ENV in files and any(
        line.partition("=")[0] != "CPR_WEB_BASE_PATH"
        for line in files[PUBLIC_ENV].decode("utf-8").splitlines()
        if line.strip() and not line.lstrip().startswith("#")
    ):
        issue(PUBLIC_ENV, 1, "unexpected-public-config-variable")
    check_documentation(files, modes, issue, counts)
    for path in sorted(component_paths):
        if (
            path not in files
            or not path.startswith("services/")
            or not path.lower().endswith(".md")
        ):
            continue
        try:
            files[path].decode("utf-8")
        except UnicodeDecodeError:
            issue(path, 1, "document-not-utf8")
        counts["component_documents_checked"] += 1

    for path in sorted(changed & files.keys()):
        counts["credential_files_checked"] += 1
        current = files[path].decode("utf-8", "replace")
        matches = {
            category: list(pattern.finditer(current))
            for category, pattern in SIGNATURES.items()
        }
        if not any(matches.values()):
            continue
        old = (
            subprocess.run(
                [
                    "git",
                    "show",
                    "--no-textconv",
                    "--end-of-options",
                    f"{credential_base}:{path}",
                ],
                stdout=subprocess.PIPE,
                stderr=subprocess.DEVNULL,
            ).stdout.decode("utf-8", "replace")
            if credential_base
            else ""
        )
        for category, candidates in matches.items():
            if not candidates:
                continue
            unchanged = Counter(
                match[0] for match in SIGNATURES[category].finditer(old)
            )
            for match in candidates:
                if unchanged[match[0]]:
                    unchanged[match[0]] -= 1
                else:
                    issue(path, current.count("\n", 0, match.start()) + 1, category)
    print(json.dumps({"counts": counts, "issues": issues}, ensure_ascii=False))
    return bool(issues)


if __name__ == "__main__":
    try:
        if sys.argv[1:] == ["select"]:
            select()
        elif sys.argv[1:] == ["check"]:
            sys.exit(check())
        else:
            sys.exit("Use check_ci.py select or check.")
    except (OSError, ValueError, subprocess.SubprocessError) as error:
        sys.exit(f"CI input could not be read: {type(error).__name__}")
