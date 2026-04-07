import base64
import os
import subprocess

subprocess.check_call(["go", "mod", "tidy"])
assert subprocess.check_output(["tinygo", "version"]).strip().split()[2] == b"0.33.0"

args = ["tinygo", "build", "-o", "wasm_tests.wasm"]

subprocess.check_call(
    args,
    env={"GOOS": "wasi1p", "GOARCH": "wasm", **os.environ},
)
with open("wasm_tests.wasm", "rb") as f:
    wasm = f.read()
    with open("../convex/wasmTests.js", "wb") as f:
        f.write(b'export const wasmSource = "')
        f.write(base64.b64encode(wasm))
        f.write(b'";\n')
