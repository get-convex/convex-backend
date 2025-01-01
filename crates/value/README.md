# Value Crate

The `value` crate provides value-related functionality in the Convex backend. It includes modules for handling various data types, serialization, and utilities for working with values.

## Purpose and Functionality

The `value` crate serves as a library for managing and manipulating different types of values used in the Convex backend. It provides utilities for handling arrays, base32 and base64 encoding, bytes, document IDs, fields, heap size, IDs, maps, metrics, numeric values, objects, sets, SHA-256 hashing, size calculations, sorting, strings, table mappings, table names, and various utilities.

## Main Modules and Components

- `array`: Handles array-related functionality.
- `base32`: Provides base32 encoding and decoding functionality.
- `base64`: Provides base64 encoding and decoding functionality.
- `bytes`: Handles byte-related functionality.
- `document_id`: Provides functionality for working with document IDs.
- `export`: Handles export-related functionality.
- `field_name`: Provides functionality for working with field names.
- `field_path`: Provides functionality for working with field paths.
- `heap_size`: Provides utilities for calculating heap size.
- `id_v6`: Provides functionality for working with version 6 IDs.
- `macros`: Contains macros used in the crate.
- `map`: Handles map-related functionality.
- `metrics`: Collects and reports metrics related to value operations.
- `numeric`: Provides functionality for working with numeric values.
- `object`: Handles object-related functionality.
- `set`: Handles set-related functionality.
- `sha256`: Provides SHA-256 hashing functionality.
- `size`: Provides utilities for size calculations.
- `sorting`: Provides functionality for sorting values.
- `string`: Handles string-related functionality.
- `table_mapping`: Provides functionality for working with table mappings.
- `table_name`: Provides functionality for working with table names.
- `tests`: Contains tests for the crate.
- `utils`: Provides various utility functions.

## Dependencies and Features

The `value` crate has the following dependencies:

- `anyhow`
- `base-62`
- `base64`
- `byteorder`
- `bytes`
- `derive_more`
- `errors`
- `hex`
- `humansize`
- `imbl`
- `metrics`
- `paste`
- `proptest`
- `proptest-derive`
- `serde`
- `serde_json`
- `sha2`
- `sync_types`
- `thiserror`
- `tokio`
- `uuid`

The `value` crate also provides the following features:

- `testing`: Enables testing-related functionality.
