#!/usr/bin/env python3
"""Build only committed source snapshots and preserve explicit build receipts."""

import argparse
from datetime import datetime, timezone
import hashlib
import io
import json
import os
from pathlib import Path, PurePosixPath
import re
import subprocess
import sys
import tarfile
import tempfile


class BuildError(Exception):
    pass


def git(root, *args):
    return subprocess.check_output(["git", *args], cwd=root, stderr=subprocess.PIPE)


def resolve_revision(root, revision):
    value = (
        git(root, "rev-parse", "--verify", "--end-of-options", revision + "^{commit}")
        .decode()
        .strip()
    )
    if re.fullmatch(r"[0-9a-f]{40}", value) is None:
        raise BuildError("The build revision must resolve to a full Git commit.")
    return value


def safe_path(value):
    path = PurePosixPath(value)
    if (
        not value
        or path.is_absolute()
        or ".." in path.parts
        or path.as_posix() != value
    ):
        raise BuildError("Noncanonical build input path.")
    return value


def read_git_file(root, revision, path):
    """Recipes and manifests must also survive a normal checkout as files."""
    safe_path(path)
    raw = git(root, "ls-tree", "-z", revision, "--", path)
    entries = [item for item in raw.split(b"\0") if item]
    if len(entries) != 1:
        raise BuildError("Missing regular build input: " + path)
    header, name = entries[0].split(b"\t", 1)
    mode, kind, oid = header.decode().split()
    if mode != "100644" or kind != "blob" or name.decode() != path:
        raise BuildError("Build input must be a regular 100644 file: " + path)
    return git(root, "cat-file", "blob", oid)


def read_target(root, revision, component):
    spec = json.loads(read_git_file(root, revision, "deploy/build-targets.json"))
    if (
        not isinstance(spec, dict)
        or spec.get("schema_version") != 1
        or component not in ("new-api", "cpr")
    ):
        raise BuildError("Unsupported build target schema or component.")
    target = spec["targets"][component]
    expected = (
        "services/new-api" if component == "new-api" else "services/codex-proxy-rs"
    )
    if target["context"] != expected:
        raise BuildError("The selected source context differs from the component.")
    for key in ("context", "dockerfile", "dockerignore"):
        safe_path(target[key])
    if not re.fullmatch(
        r"https://github\.com/[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+",
        spec["source_repository"],
    ):
        raise BuildError("Invalid public source repository label.")
    return spec, target


def prepare_snapshot(root, revision, target, folder):
    """Never send untracked files, working secrets, or the root Git directory."""
    context = folder / "context"
    context.mkdir()
    raw = git(root, "archive", "--format=tar", revision + ":" + target["context"])
    with tarfile.open(fileobj=io.BytesIO(raw), mode="r:") as archive:
        members = archive.getmembers()
        for item in members:
            if ".git" in PurePosixPath(item.name).parts:
                raise BuildError("Unexpected Git metadata in build context.")
        archive.extractall(context, filter="data")
    dockerfile = folder / "Dockerfile"
    dockerfile.write_bytes(read_git_file(root, revision, target["dockerfile"]))
    dockerignore = folder / "Dockerfile.dockerignore"
    dockerignore.write_bytes(read_git_file(root, revision, target["dockerignore"]))
    tree = git(root, "rev-parse", revision + ":" + target["context"]).decode().strip()
    return context, dockerfile, tree


def build_command(
    docker,
    config,
    component,
    revision,
    source_url,
    target,
    tag,
    platform,
    registry,
    context,
    dockerfile,
    metadata,
    builder=None,
    go_proxy="https://proxy.golang.org,direct",
):
    if component not in ("new-api", "cpr") or platform not in (
        "linux/amd64",
        "linux/arm64",
    ):
        raise BuildError("Unsupported component or platform.")
    if registry not in ("docker.io", "docker.m.daocloud.io"):
        raise BuildError("Registry must be an explicitly supported same-digest source.")
    if go_proxy not in ("https://proxy.golang.org,direct", "https://goproxy.cn,direct"):
        raise BuildError(
            "Go proxy must use an explicitly supported checksum-verified source."
        )
    if not re.fullmatch(r"[a-z0-9][a-z0-9/_.-]*:[A-Za-z0-9_.-]+", tag):
        raise BuildError("Image tag must be a local, non-secret name and tag.")
    command = [docker]
    if config is not None:
        command += ["--config", str(config)]
    command += [
        "buildx",
        "build",
        "--load",
        "--progress",
        "plain",
        "--platform",
        platform,
        "--file",
        str(dockerfile),
        "--tag",
        tag,
        "--metadata-file",
        str(metadata),
        "--label",
        "org.opencontainers.image.revision=" + revision,
        "--label",
        "org.opencontainers.image.source=" + source_url,
        "--build-arg",
        "IMAGE_REGISTRY=" + registry,
    ]
    if target.get("target"):
        command += ["--target", target["target"]]
    if component == "cpr":
        command += [
            "--build-arg",
            "CPR_GIT_SHA=" + revision,
            "--build-arg",
            "CARGO_BUILD_JOBS=2",
        ]
    else:
        command += ["--build-arg", "BUILD_JOBS=2", "--build-arg", "GOPROXY=" + go_proxy]
    if builder:
        command += ["--builder", builder]
    return command + [str(context)]


def main(root=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("component", choices=("new-api", "cpr"))
    parser.add_argument("--revision", default="HEAD")
    parser.add_argument(
        "--platform", default="linux/amd64", choices=("linux/amd64", "linux/arm64")
    )
    parser.add_argument(
        "--registry", default="docker.io", choices=("docker.io", "docker.m.daocloud.io")
    )
    parser.add_argument(
        "--go-proxy",
        default="https://proxy.golang.org,direct",
        choices=("https://proxy.golang.org,direct", "https://goproxy.cn,direct"),
    )
    parser.add_argument(
        "--docker-config",
        type=Path,
        help="optional isolated Docker CLI config directory",
    )
    parser.add_argument("--builder")
    parser.add_argument("--tag")
    parser.add_argument("--output", type=Path, help="new JSON receipt under .runtime/")
    args = parser.parse_args()
    root = Path(root) if root is not None else Path(__file__).resolve().parents[1]
    revision = resolve_revision(root, args.revision)
    spec, target = read_target(root, revision, args.component)
    tag = args.tag or target["image_name"] + ":" + revision[:12]
    runtime = root / ".runtime"
    runtime.mkdir(exist_ok=True)
    started = datetime.now(timezone.utc)
    output = (
        args.output
        or runtime
        / "builds"
        / (
            args.component
            + "-"
            + revision[:12]
            + "-"
            + started.strftime("%Y%m%dT%H%M%S%f")
            + ".json"
        )
    ).resolve()
    if (
        not output.is_relative_to(runtime.resolve())
        or output.suffix != ".json"
        or output.exists()
    ):
        raise BuildError(
            "Build receipt must be a new JSON file under ignored .runtime/."
        )
    output.parent.mkdir(parents=True, exist_ok=True)
    metadata = output.with_suffix(".buildkit.json")
    log = output.with_suffix(".log")
    if metadata.exists() or log.exists():
        raise BuildError("Build metadata or log already exists.")
    env = dict(os.environ)
    for key in tuple(env):
        if key in (
            "GITHUB_TOKEN",
            "GH_TOKEN",
            "XINAPI_ONETIME_AUTH",
            "GIT_CURL_VERBOSE",
        ) or key.startswith("GIT_TRACE"):
            env.pop(key)
    receipt = {
        "schema_version": 1,
        "component": args.component,
        "revision": revision,
        "platform": args.platform,
        "registry": args.registry,
        "image_tag": tag,
        "go_proxy": args.go_proxy if args.component == "new-api" else None,
        "builder_tool_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        "started_at": started.isoformat(),
        "result": "not_run",
        "source_snapshot": "committed Git tree",
        "runtime_service_started": False,
        "log_file": str(log.relative_to(root)),
        "metadata_file": str(metadata.relative_to(root)),
        "image_id": None,
    }
    temporary = runtime / "build-contexts"
    temporary.mkdir(exist_ok=True)
    with tempfile.TemporaryDirectory(
        prefix=args.component + "-", dir=temporary
    ) as folder:
        context, dockerfile, tree = prepare_snapshot(
            root, revision, target, Path(folder)
        )
        receipt.update(
            source_tree=tree,
            dockerfile_sha256=hashlib.sha256(dockerfile.read_bytes()).hexdigest(),
            dockerignore_sha256=hashlib.sha256(
                dockerfile.with_name("Dockerfile.dockerignore").read_bytes()
            ).hexdigest(),
        )
        command = build_command(
            "docker",
            args.docker_config,
            args.component,
            revision,
            spec["source_repository"],
            target,
            tag,
            args.platform,
            args.registry,
            context,
            dockerfile,
            metadata,
            args.builder,
            args.go_proxy,
        )
        print(
            "Building committed",
            args.component,
            revision,
            args.platform,
            "via",
            args.registry,
            flush=True,
        )
        try:
            with log.open("wb") as stream:
                result = subprocess.run(
                    command,
                    cwd=root,
                    env=env,
                    stdout=stream,
                    stderr=subprocess.STDOUT,
                    timeout=3600,
                )
            receipt["exit_code"] = result.returncode
            receipt["result"] = "passed" if result.returncode == 0 else "failed"
            if result.returncode == 0:
                inspect = ["docker"]
                if args.docker_config is not None:
                    inspect += ["--config", str(args.docker_config)]
                info = subprocess.check_output(
                    inspect + ["image", "inspect", tag], env=env
                )
                image = json.loads(info)[0]
                labels = image.get("Config", {}).get("Labels") or {}
                if labels.get("org.opencontainers.image.revision") != revision:
                    raise BuildError(
                        "Loaded image does not carry the selected source revision."
                    )
                receipt["image_id"] = image["Id"]
                receipt["buildkit"] = (
                    json.loads(metadata.read_bytes()) if metadata.exists() else {}
                )
        except subprocess.TimeoutExpired:
            receipt.update(result="failed", exit_code=124, failure="build_timeout")
        except (
            BuildError,
            OSError,
            ValueError,
            KeyError,
            subprocess.CalledProcessError,
        ) as error:
            receipt.update(result="failed", exit_code=1, failure=type(error).__name__)
        receipt["finished_at"] = datetime.now(timezone.utc).isoformat()
        output.write_text(json.dumps(receipt, ensure_ascii=False, indent=2) + "\n")
    print("Build", receipt["result"], "receipt:", output.relative_to(root), flush=True)
    if receipt["result"] != "passed":
        print(
            "See the local log for the actual dependency, network or compiler failure.",
            file=sys.stderr,
        )
    return 0 if receipt["result"] == "passed" else receipt.get("exit_code", 1)


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (
        BuildError,
        OSError,
        ValueError,
        KeyError,
        subprocess.CalledProcessError,
    ) as error:
        print(
            "Build preparation failed:",
            type(error).__name__,
            str(error)[:250],
            file=sys.stderr,
        )
        sys.exit(1)
