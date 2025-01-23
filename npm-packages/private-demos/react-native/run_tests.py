#!/usr/bin/python3
import argparse
import os
import subprocess
import sys
from pathlib import Path

SCRIPT_DIR_NAME = os.path.dirname(os.path.abspath(__file__))


def parse_args():
    parser = argparse.ArgumentParser(
        description="Run tests for React Native using Convex"
    )
    parser.add_argument(
        "convex_monorepo_path",
        type=Path,
    )
    return parser.parse_args()


def main(args):
    react_native_private_demo_path = (
        args.convex_monorepo_path / "npm-packages" / "private-demos" / "react-native"
    )

    print("Running `just convex deploy`")
    subprocess.check_call(
        ["just", "convex", "deploy"], cwd=react_native_private_demo_path
    )
    print("Done!")

    print("Building app (may take a while)")
    # This command is really noisy, so swallow most of the output and print the
    # last bit only if the command fails
    completed_process = subprocess.run(
        ["npx", "detox", "build", "--configuration", "ios.sim.release"],
        capture_output=True,
        encoding="utf8",
        cwd=react_native_private_demo_path,
    )
    if completed_process.returncode != 0:
        raise subprocess.CalledProcessError(
            cmd=["npx", "detox", "build", "--configuration", "ios.sim.release"],
            returncode=completed_process.returncode,
            output=completed_process.stdout[-1000:],
            stderr=completed_process.stderr[-1000:],
        )

    print("Done building app!")

    print("Testing app")
    subprocess.check_call(
        ["npx", "detox", "test", "--configuration", "ios.sim.release"],
        cwd=react_native_private_demo_path,
    )
    print("Done testing app!")


if __name__ == "__main__":
    args = parse_args()
    try:
        main(args)
        sys.exit(0)
    except subprocess.CalledProcessError as e:
        print("Hit an error -- cleaning up and exiting")
        print(e, e.stdout, e.stderr)
        sys.exit(1)
