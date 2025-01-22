#!/usr/bin/python3
import argparse
import os
import shutil
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

    parser.add_argument(
        "--demo-relative-path",
        "--demo_relative_path",
        "-d",
        type=Path,
        default=Path("npm-packages", "private-demos", "react-native"),
        required=False,
    )
    return parser.parse_args()


def main(args):
    demo_path = args.convex_monorepo_path / args.demo_relative_path
    convex_npm_package_path = args.convex_monorepo_path / "npm-packages" / "convex"
    print(f"Linking {demo_path} with local version of Convex")
    output = subprocess.check_output(
        ["npm", "pack"], cwd=convex_npm_package_path, encoding="utf8"
    )
    packed_file = output.splitlines()[-1].strip()
    shutil.move(
        (convex_npm_package_path / packed_file),
        (demo_path / packed_file),
    )
    subprocess.check_call(["npm", "install", packed_file], cwd=demo_path)
    print("Done!")


if __name__ == "__main__":
    args = parse_args()
    try:
        main(args)
        sys.exit(0)
    except subprocess.CalledProcessError as e:
        print("Hit an error -- exiting")
        print(e, e.stdout, e.stderr)
        sys.exit(1)
