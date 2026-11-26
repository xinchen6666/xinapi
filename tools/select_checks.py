#!/usr/bin/env python3
"""Select affected image builds without interpolating PR input into shell code."""

import argparse
import json
import os
from pathlib import Path
import re
import subprocess
import sys


def affected_targets(paths):
    new_api = False
    cpr = False
    common = (".github/", "deploy/build-targets.json", "tools/build_image.py")
    for path in paths:
        if path.startswith(common):
            new_api = cpr = True
        if path.startswith(("services/new-api/", "deploy/docker/new-api.")):
            new_api = True
        if path.startswith(("services/codex-proxy-rs/", "deploy/docker/cpr.")):
            cpr = True
    return {"new-api": new_api, "cpr": cpr}


def changed_paths(root, base, head):
    if not re.fullmatch(r"[0-9a-f]{40}", head):
        raise ValueError("Head must be a full commit SHA.")
    subprocess.run(
        ["git", "cat-file", "-e", head + "^{commit}"],
        cwd=root,
        check=True,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.PIPE,
    )
    if not base or base == "0" * 40:
        return None
    if not re.fullmatch(r"[0-9a-f]{40}", base):
        raise ValueError("Base must be a full commit SHA.")
    subprocess.run(
        ["git", "cat-file", "-e", base + "^{commit}"],
        cwd=root,
        check=True,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.PIPE,
    )
    output = subprocess.check_output(
        ["git", "diff", "--name-only", "--no-renames", "-z", base, head, "--"], cwd=root
    )
    return [item.decode("utf-8") for item in output.split(b"\0") if item]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base", default=os.environ.get("BASE_SHA", ""))
    parser.add_argument("--head", default=os.environ.get("HEAD_SHA", ""))
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[1]
    head = (
        args.head
        or subprocess.check_output(
            ["git", "rev-parse", "HEAD"], cwd=root, text=True
        ).strip()
    )
    paths = changed_paths(root, args.base, head)
    selected = (
        {"new-api": True, "cpr": True} if paths is None else affected_targets(paths)
    )
    result = {
        "new_api": str(selected["new-api"]).lower(),
        "cpr": str(selected["cpr"]).lower(),
    }
    if os.environ.get("GITHUB_OUTPUT"):
        with open(os.environ["GITHUB_OUTPUT"], "a", encoding="utf-8") as stream:
            for name, value in result.items():
                stream.write(name + "=" + value + "\n")
    print(
        json.dumps(
            {"changed_paths": None if paths is None else len(paths), "builds": result},
            sort_keys=True,
        )
    )


if __name__ == "__main__":
    try:
        main()
    except (ValueError, OSError, subprocess.CalledProcessError) as error:
        print("Cannot select checks:", type(error).__name__, file=sys.stderr)
        sys.exit(1)
