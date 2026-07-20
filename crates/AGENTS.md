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
