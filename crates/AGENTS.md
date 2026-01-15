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
