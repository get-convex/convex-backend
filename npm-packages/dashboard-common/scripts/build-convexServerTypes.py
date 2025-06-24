import json
import os

RELATIVE_PATH_TO_TYPES = "../../convex/dist/esm-types"

RELATIVE_PATH_TO_OUTPUT_FILE = "../src/lib/generated/convexServerTypes.json"

# The entrypoints and helpers from `convex` NPM package used in Convex server functions
SERVER_ENTRYPOINTS = ["server", "values", "type_utils"]

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
        # Collect all .d.ts, .d.cts, .d.mts files in this directory
        dir_files = {}
        for item in os.listdir(path):
            item_path = os.path.join(path, item)
            if os.path.isdir(item_path):
                dts_files.update(find_dts_files(item_path, base_path))
            elif item.endswith((".d.mts", ".d.cts", ".d.ts")):
                # Extract base name (e.g., "a" from "a.d.mts")
                if item.endswith(".d.mts"):
                    base_name = item[:-6]  # Remove ".d.mts"
                    priority = 0  # Highest priority
                elif item.endswith(".d.cts"):
                    base_name = item[:-6]  # Remove ".d.cts"
                    priority = 1
                else:  # .d.ts
                    base_name = item[:-5]  # Remove ".d.ts"
                    priority = 2  # Lowest priority
                
                if base_name not in dir_files or priority < dir_files[base_name][1]:
                    dir_files[base_name] = (item_path, priority)
        
        # Process the selected files from this directory
        for item_path, _ in dir_files.values():
            relative_path = os.path.relpath(item_path, base_path)
            with open(item_path, "r", encoding="utf-8") as file:
                dts_files[PATH_PREFIX + relative_path] = strip_source_map_suffix(
                    file.read()
                )
    elif path.endswith((".d.mts", ".d.cts", ".d.ts")):
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
