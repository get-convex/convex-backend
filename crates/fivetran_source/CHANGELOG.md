# 2.0.0

- Add support for Convex components.
- Add support for partial component, table, and column selection.
- This version requires a full historical resync for existing connections.

# 0.6.0

- Update rust nightly version in .rust-toolchain
- Remove deprecated set/map support
- Add gzip support to connector

# 0.5.0

- Emit a TRUNCATE op before the first write to any given table (in case it's a
  resync)
- Skip emitting an initial sync snapshot before the first page.

# 0.4.0

- Revert the `_convex_cursor` changes.
- Few updates to loglines
- docs updates

# 0.3.0

- Support a `_convex_cursor` table with a single column `cursor` which holds the
  `document_deltas` cursor representing the most recent sync.
- Add documentation to docs/

# 0.2.0

- Bump `convex` dep to 0.5.0
- Use /test_streaming_export_connection and /get_tables_and_columns endpoints
  rather than json_schemas json_schemas has stricter requirements around nested
  schemas than what fivetran requires.
