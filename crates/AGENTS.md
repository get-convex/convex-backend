# Rust crates

## Development workflow

```sh
# After each change
just format-rust

# When a change is ready
just lint-rust
cargo build -p <package>
cargo test -p <package>
cargo test -p <package> "test_name" # for a specific test or test group
```

## Rust style

- Before adding a crate dependency or a new abstraction, check whether existing
  workspace infrastructure already provides the capability and prefer the
  simplest extension that works.
- Match domain enums exhaustively instead of using a `_ =>` catch-all so adding
  a variant causes a compile error.
- Use self-documenting domain types: prefer named structs over positional
  tuples, enums over boolean flags, and `Duration` or a newtype over bare
  numbers with implicit units.
- Prefer the `anyhow::Context` trait (`.context(...)`/`.with_context(...)`) over
  `.ok_or_else(|| anyhow::anyhow!(...))` or `.map_err(|_| anyhow::anyhow!(...))`
  for attaching a message to a `Result` or `Option`; it says the same thing more
  concisely.
