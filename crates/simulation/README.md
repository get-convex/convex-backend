```bash
ELLE_DOT_PATH=/tmp/elle.dot RUST_BACKTRACE=1 RUST_LOG=simulation=error cargo test test_elle -- --nocapture; dot -Tsvg /tmp/elle.dot > /tmp/elle.svg
```
