import argparse
import glob
import os
import random
import string

# Create a module
SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))


def clear_convex_dir() -> None:
    files_to_delete = glob.glob(os.path.join(SCRIPT_DIR, "convex/*"))

    # Delete the files
    for file in files_to_delete:
        if "_generated" in file:
            continue
        if "tables.ts" in file:
            continue
        if "tsconfig" in file:
            continue
        os.remove(file)


# TODO generate functions too! Just in case analyze is taking more time.
def generate_file(path: str, is_node: bool = False, size: int = 1_000_000) -> None:
    rand = "".join(random.choices(string.ascii_uppercase, k=size))
    s = '"use node";\n' if is_node else ""
    s = s + f"""export const a =\n  "{rand}";"""
    open(path, "w").write(s)


def main(
    node_files: int,
    v8_files: int,
    file_size: int,
    normal_tables: int,
    search_index_tables: int,
    vector_index_tables: int,
) -> None:
    print(
        f"Generating {node_files} node files and {v8_files} v8 files, each {file_size} bytes."
    )
    print(
        f"Schema will have {normal_tables} normal tables, {search_index_tables} search index tables, and {vector_index_tables} vector index tables."
    )
    clear_convex_dir()
    schema = f"""import {{ defineSchema }} from "convex/server";
import {{ tables }} from "./tables";

export default defineSchema({{
  ...tables({{
    normal: {normal_tables},
    withSearchIndex: {search_index_tables},
    withVectorIndex: {vector_index_tables},
  }}),
}});
"""
    open(os.path.join(SCRIPT_DIR, "convex/schema.ts"), "w").write(schema)

    for i in range(node_files):
        generate_file(f"convex/generated_node_{i}.ts", True, file_size)
    for i in range(v8_files):
        generate_file(f"convex/generated_{i}.ts", False, file_size)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Process some integers.")

    # Required flags with values
    parser.add_argument(
        "--node-files",
        type=int,
        required=True,
        help="How many node files should be generated?",
    )
    parser.add_argument(
        "--v8-files",
        type=int,
        required=True,
        help="How many v8 files should be generated?",
    )

    # Optional flags with default values
    parser.add_argument(
        "--file-size",
        type=int,
        default=1_000_000,
        help="How large should each file be (in MB)?",
    )
    parser.add_argument(
        "--normal-tables",
        type=int,
        default=0,
        help="How many normal tables should the schema have?",
    )
    parser.add_argument(
        "--search-index-tables",
        type=int,
        default=0,
        help="How many search index tables should exist?",
    )
    parser.add_argument(
        "--vector-index-tables",
        type=int,
        default=0,
        help="How many vector index tables should exist?",
    )

    args = parser.parse_args()

    main(
        args.node_files,
        args.v8_files,
        args.file_size,
        args.normal_tables,
        args.search_index_tables,
        args.vector_index_tables,
    )
