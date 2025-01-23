import json
import os

RELATIVE_PATH_TO_TYPES = "../../convex/dist/esm-types"

RELATIVE_PATH_TO_OUTPUT_FILE = "../src/lib/generated/convexServerTypes.json"

# The entrypoints and helpers from `convex` NPM package used in Convex server functions
SERVER_ENTRYPOINTS = ["server", "values", "type_utils.d.ts"]

# For VS Code
PATH_PREFIX = "file:///convex/"


def main():
    this_directory = os.path.dirname(os.path.abspath(__file__))
    convex_build_directory = os.path.join(this_directory, RELATIVE_PATH_TO_TYPES)

    result = {}
    for entry_point in SERVER_ENTRYPOINTS:
        result.update(build_entrypoint(convex_build_directory, entry_point))

    output_path = os.path.join(this_directory, RELATIVE_PATH_TO_OUTPUT_FILE)
    os.makedirs(os.path.dirname(output_path), exist_ok=True)
    with open(output_path, "w", encoding="utf-8") as json_file:
        json.dump(result, json_file, indent=2, ensure_ascii=False)
        json_file.write("\n")


def build_entrypoint(convex_build_directory, entry_point):
    return find_dts_files(
        os.path.join(convex_build_directory, entry_point), convex_build_directory
    )


def find_dts_files(path, base_path):
    dts_files = {}
    if os.path.isdir(path):
        for item in os.listdir(path):
            item_path = os.path.join(path, item)
            dts_files.update(find_dts_files(item_path, base_path))
    elif path.endswith(".d.ts"):
        relative_path = os.path.relpath(path, base_path)
        with open(path, "r", encoding="utf-8") as file:
            dts_files[PATH_PREFIX + relative_path] = strip_source_map_suffix(
                file.read()
            )
    return dts_files


def strip_source_map_suffix(code):
    return code.rsplit("//# sourceMappingURL", 1)[0]


if __name__ == "__main__":
    main()
