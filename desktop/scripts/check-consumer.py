import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile


def cargo(arguments, environment, capture=False):
    return subprocess.run(
        ["cargo", *arguments],
        check=True,
        env=environment,
        text=True,
        capture_output=capture,
    )


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


def verify_graph(metadata, consumer):
    require(
        Path(metadata["workspace_root"]) == consumer,
        "The consumer must resolve as an independent workspace",
    )
    packages = metadata["packages"]
    engines = [package for package in packages if package["name"] == "gpui"]
    require(
        len(engines) == 1 and engines[0]["version"] == "0.2.2",
        "The consumer must resolve exactly one supported gpui 0.2.2 engine",
    )
    libc = [package for package in packages if package["name"] == "libc"]
    require(
        len(libc) == 1 and libc[0]["version"] == "0.2.189",
        "The adapter manifest must constrain libc to the compatible 0.2.189 release",
    )
    adapter = next(package for package in packages if package["name"] == "rstate-gpui")
    nodes = {node["id"]: node for node in metadata["resolve"]["nodes"]}
    engine = engines[0]["id"]
    for node in [nodes[metadata["resolve"]["root"]], nodes[adapter["id"]]]:
        require(
            any(dependency["name"] == "gpui" and dependency["pkg"] == engine
                for dependency in node["deps"]),
            "The consumer and adapter must use the same GPUI package identity",
        )


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--offline", action="store_true")
    options = parser.parse_args()
    desktop = Path(__file__).resolve().parents[1]
    root = desktop.parent
    fixture = desktop / "fixtures" / "consumer"
    environment = os.environ.copy()
    environment.setdefault("CARGO_BUILD_JOBS", "2")
    environment.setdefault("CARGO_TARGET_DIR", str(desktop / "target"))
    with tempfile.TemporaryDirectory(prefix="rstate-gpui-consumer-") as temporary:
        consumer = Path(temporary)
        shutil.copytree(fixture / "src", consumer / "src")
        manifest = (fixture / "Cargo.toml").read_text()
        manifest = manifest.replace(
            'path = "../../../crates/rstate"',
            f"path = {json.dumps(str(root / 'crates' / 'rstate'))}",
        ).replace(
            'path = "../../rstate-gpui"',
            f"path = {json.dumps(str(desktop / 'rstate-gpui'))}",
        )
        (consumer / "Cargo.toml").write_text(manifest)
        arguments = ["--manifest-path", str(consumer / "Cargo.toml")]
        if options.offline:
            arguments.append("--offline")
        require(not (consumer / "Cargo.lock").exists(), "The consumer must start without a lockfile")
        cargo(["generate-lockfile", *arguments], environment)
        metadata = cargo(
            ["metadata", "--format-version", "1", "--locked", *arguments],
            environment,
            capture=True,
        )
        verify_graph(json.loads(metadata.stdout), consumer)
        cargo(["check", "--locked", "--all-targets", *arguments], environment)
    print("Fresh consumer passed: independent lockfile, matching GPUI types, compatible libc, public API check")


if __name__ == "__main__":
    try:
        main()
    except subprocess.CalledProcessError as error:
        if error.stderr:
            print(error.stderr, file=sys.stderr)
        raise SystemExit(error.returncode) from None
    except RuntimeError as error:
        raise SystemExit(str(error)) from None
