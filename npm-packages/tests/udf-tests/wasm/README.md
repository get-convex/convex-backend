# WASM tests

This folder has some _very basic_ WASM tests for our V8 runtime. We don't
currently automatically rebuild `main.go` into its `wasmTests.js` binary, so
you'll have to do this manually whenever you make changes. Be sure to have
`tinygo` version `0.33.0` installed.

```bash
go mod tidy
python build.py
```
