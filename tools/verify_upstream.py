#!/usr/bin/env python3
"""Verify the unchanged X02 source baseline without network or build scripts.

Default checks working files. --index checks exactly what Git will commit.
--git-ref COMMIT checks a historical baseline after later product changes.
"""

import argparse
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import subprocess
import sys


PINS = {
    "new-api": {
        "component": "new-api", "destination": "services/new-api",
        "repository": "https://github.com/QuantumNous/new-api",
        "commit": "8542c9d6490b4330fe18160930f81ae7428bc777",
        "tree": "11dbbb8a51210a38b84d9f9358c56c9d0415f5e8",
        "import_tree": "11dbbb8a51210a38b84d9f9358c56c9d0415f5e8", "gitlinks": {},
    },
    "codex-proxy-rs": {
        "component": "codex-proxy-rs", "destination": "services/codex-proxy-rs",
        "repository": "https://github.com/zyycn/codex-proxy-rs",
        "commit": "58cbb607505dda4a1cee6e01b9f76e267ad3c261",
        "tree": "d606f1081ffdeb53dbbd9efd89c6edef9427887e",
        "import_tree": "3edc9042af879db4d0d8c5a09c39d2b142f5994c",
        "gitlinks": {"modules/ui": "501495d5b1e9d65840d4cdee5e0e228f361613ee",
                     "modules/plugins": "b02d267a19fd39bd004f408a685c4db51b555c99"},
    },
}


class VerificationError(Exception):
    pass


def require(condition, message):
    if not condition:
        raise VerificationError(message)


def object_hash(kind, data):
    return hashlib.sha1(kind.encode() + b" " + str(len(data)).encode() + b"\0" + data).hexdigest()


def tree_hash(entries):
    """Reconstruct a Git tree, including explicitly omitted optional gitlinks."""
    root = {}
    for path, mode, oid in entries:
        parts = PurePosixPath(path).parts
        require(parts and not path.startswith("/") and ".." not in parts and ".git" not in parts,
                f"unsafe manifest path: {path}")
        require(path == PurePosixPath(path).as_posix(), f"noncanonical path: {path}")
        require(mode in ("100644", "100755", "120000", "160000"), f"unsupported mode: {path}")
        require(re.fullmatch(r"[0-9a-f]{40}", oid) is not None, f"invalid object id: {path}")
        node = root
        for part in parts[:-1]:
            node = node.setdefault(part, {})
            require(isinstance(node, dict), f"file/directory collision: {path}")
        require(parts[-1] not in node, f"duplicate path: {path}")
        node[parts[-1]] = (mode, oid)

    def digest(node):
        data = bytearray()
        order = sorted(node.items(), key=lambda item: (item[0] + ("/" if isinstance(item[1], dict) else "")).encode())
        for name, value in order:
            mode, oid = ("40000", digest(value)) if isinstance(value, dict) else value
            data.extend(mode.encode() + b" " + name.encode() + b"\0" + bytes.fromhex(oid))
        return object_hash("tree", bytes(data))

    return digest(root)


class GitFiles:
    """One batch reader for exact index or historical object bytes."""
    def __init__(self, root, git_ref=None):
        self.root = root
        if git_ref is None:
            common = ["git", "diff", "--cached", "--name-status", "--no-renames", "-z"]
            scope = ["--", "services/new-api", "services/codex-proxy-rs"]
            visible = subprocess.check_output(common + ["--ita-visible-in-index"] + scope, cwd=root)
            actual = subprocess.check_output(common + ["--ita-invisible-in-index"] + scope, cwd=root)
            require(visible == actual, "intent-to-add source entries are not committed files")
            raw = subprocess.check_output(["git", "ls-files", "--stage", "-z"], cwd=root)
        else:
            revision = subprocess.check_output(
                ["git", "rev-parse", "--verify", "--end-of-options", git_ref + "^{tree}"],
                cwd=root, text=True).strip()
            raw = subprocess.check_output(["git", "ls-tree", "-r", "-z", revision], cwd=root)
        self.files = {}
        for record in raw.split(b"\0"):
            if not record:
                continue
            info, name = record.split(b"\t", 1)
            mode, middle, end = info.decode().split()
            if git_ref is None:
                require(end == "0", "unmerged index")
                oid = middle
            else:
                require(middle in ("blob", "commit"), "unexpected tree entry")
                oid = end
            path = name.decode()
            require(path not in self.files, f"duplicate Git index path: {path}")
            self.files[path] = (mode, oid)
        self.batch = subprocess.Popen(["git", "cat-file", "--batch"], cwd=root,
                                      stdin=subprocess.PIPE, stdout=subprocess.PIPE)

    def read(self, path):
        require(path in self.files, f"not tracked: {path}")
        mode, oid = self.files[path]
        require(mode != "160000", f"unexpected nested Git repository: {path}")
        self.batch.stdin.write((oid + "\n").encode())
        self.batch.stdin.flush()
        header = self.batch.stdout.readline().split()
        require(len(header) == 3 and header[1] == b"blob", f"unreadable blob: {path}")
        data = self.batch.stdout.read(int(header[2]))
        require(self.batch.stdout.read(1) == b"\n", f"truncated Git blob: {path}")
        return mode, data

    def close(self):
        self.batch.stdin.close()
        self.batch.stdout.close()
        require(self.batch.wait() == 0, "git cat-file failed")


def verify_component(root, manifest, pin, git=None):
    require(manifest.get("schema_version") == 1, "unsupported provenance schema")
    source = manifest.get("source", manifest)
    declared_tree = source.get("git_tree", source.get("git_tree_sha"))
    require(manifest.get("component") == pin["component"], "unexpected component identity")
    require(manifest.get("destination") == pin["destination"], "unexpected source destination")
    require(source.get("repository") == pin["repository"] and source.get("commit") == pin["commit"],
            "source repository/commit differs from reviewed pin")
    require(declared_tree == pin["tree"], "declared upstream source tree differs from reviewed pin")
    require(manifest.get("import_tree_sha") == pin["import_tree"],
            "declared imported file tree differs from reviewed pin")
    links = manifest.get("gitlinks", [])
    require(len(links) == len(pin["gitlinks"])
            and {item["path"]: item["commit"] for item in links} == pin["gitlinks"],
            "optional gitlinks differ from reviewed pins")
    prefix = pin["destination"]
    base = root / prefix
    files = manifest["files"]
    require(len(files) == manifest["file_count"], f"manifest file count: {prefix}")
    expected = {item["path"] for item in files}
    require(len(expected) == len(files), f"duplicate manifest entry: {prefix}")
    entries = []
    total = 0
    for item in files:
        rel = item["path"]
        require(rel == PurePosixPath(rel).as_posix() and not rel.startswith("/")
                and ".." not in PurePosixPath(rel).parts and ".git" not in PurePosixPath(rel).parts,
                f"unsafe path: {rel}")
        path = base / rel
        if git is not None:
            mode, data = git.read(prefix + "/" + rel)
        else:
            require(path.is_file() or path.is_symlink(), f"missing source: {prefix}/{rel}")
            require(path.parent.resolve().is_relative_to(base.resolve()), f"source path escapes root: {rel}")
            if path.is_symlink():
                mode, data = "120000", os.readlink(path).encode()
                require(path.resolve().is_relative_to(base.resolve()), f"symlink escapes source: {rel}")
            else:
                mode, data = ("100755" if path.stat().st_mode & 0o100 else "100644"), path.read_bytes()
        require(mode == item["mode"], f"mode differs: {prefix}/{rel}")
        require(len(data) == item["size"], f"size differs: {prefix}/{rel}")
        require(hashlib.sha256(data).hexdigest() == item["sha256"], f"content differs: {prefix}/{rel}")
        oid = object_hash("blob", data)
        require(oid == item["git_blob_sha"], f"Git blob differs: {prefix}/{rel}")
        require(not data.startswith(b"version https://git-lfs.github.com/spec/"), f"unresolved LFS: {rel}")
        entries.append((rel, mode, oid))
        total += len(data)
    if git is None:
        actual = set()
        for folder, dirs, names in os.walk(base, followlinks=False):
            require(".git" not in dirs and ".git" not in names, f"nested Git metadata: {folder}")
            for name in list(dirs):
                if (Path(folder) / name).is_symlink():
                    dirs.remove(name)
                    names.append(name)
            actual.update((Path(folder) / name).relative_to(base).as_posix() for name in names)
    else:
        actual = {name[len(prefix) + 1:] for name in git.files if name.startswith(prefix + "/")}
    require(actual == expected, f"source inventory differs: {prefix}; missing {len(expected - actual)}, extra {len(actual - expected)}")
    require(tree_hash(entries) == pin["import_tree"], f"imported file tree differs: {prefix}")
    for link in links:
        require(link.get("disposition") == "omitted_optional_source_development_gitlink",
                f"unreviewed gitlink: {link['path']}")
        entries.append((link["path"], "160000", link["commit"]))
    require(tree_hash(entries) == pin["tree"], f"upstream root tree differs: {prefix}")
    for required in manifest.get("required_inputs", []):
        require(required in expected, f"required input not present: {required}")
    return {"component": manifest["component"], "files": len(files), "bytes": total,
            "import_tree": pin["import_tree"], "tree": pin["tree"],
            "optional_gitlinks_omitted": len(links)}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    group = parser.add_mutually_exclusive_group()
    group.add_argument("--index", action="store_true")
    group.add_argument("--git-ref", help="validate an immutable historical X02 source baseline")
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[1]
    git = GitFiles(root, args.git_ref) if args.index or args.git_ref else None
    try:
        for name in ("new-api", "codex-proxy-rs"):
            path = f"third_party/{name}.provenance.json"
            raw = git.read(path)[1] if git else (root / path).read_bytes()
            result = verify_component(root, json.loads(raw), PINS[name], git)
            print(json.dumps(result, ensure_ascii=False, sort_keys=True))
    finally:
        if git:
            git.close()


if __name__ == "__main__":
    try:
        main()
    except (VerificationError, OSError, ValueError, KeyError, subprocess.SubprocessError) as error:
        print(f"Source verification failed: {error}", file=sys.stderr)
        sys.exit(1)
