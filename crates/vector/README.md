# Vector Crate

The `vector` crate handles vector-related functionality within the Convex backend. It provides various modules and components to manage and query vector data efficiently.

## Purpose and Functionality

The `vector` crate is designed to manage vector data and perform vector-based searches. It includes functionality for indexing, querying, and managing vector data. The crate is used by the `application` crate to provide vector-related features.

## Main Modules and Components

### `id_tracker`
This module handles tracking of vector IDs.

### `memory_index`
This module provides an in-memory implementation of a vector index.

### `metrics`
This module handles metrics related to vector operations.

### `qdrant_index`
This module provides an implementation of a vector index using Qdrant.

### `qdrant_segments`
This module manages segments of vector data in Qdrant.

### `query`
This module handles vector search queries.

### `searcher`
This module provides functionality for searching vector data.

### `vector_index_manager`
This module manages vector indexes.

## Dependencies and Features

The `vector` crate depends on several other crates within the Convex backend:

- `common`: Provides common utilities and components.
- `errors`: Handles error-related functionality.
- `indexing`: Provides indexing-related functionality.
- `metrics`: Handles metrics-related functionality.
- `pb`: Provides protocol buffer-related functionality.
- `storage`: Handles storage-related functionality.
- `value`: Provides value-related functionality.

The crate also includes optional dependencies for testing purposes, such as `proptest` and `criterion`.

## Usage

The `vector` crate is used by the `application` crate to provide vector-related features. It can be used to manage vector data, perform vector-based searches, and track vector IDs.

## Example

Here is an example of how to use the `vector` crate to perform a vector search:

```rust
use vector::{VectorSearcher, VectorSearchRequest};

let searcher = VectorSearcher::new();
let request = VectorSearchRequest::new(vec![1.0, 2.0, 3.0]);
let results = searcher.search(request);
```
