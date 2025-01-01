# Keybroker Crate

The `keybroker` crate handles key management-related functionality for the Convex backend. It provides mechanisms for issuing and verifying keys, managing identities, and encrypting/decrypting data.

## Purpose and Functionality

The primary purpose of the `keybroker` crate is to manage keys and identities within the Convex backend. It includes functionality for issuing admin keys, system keys, and store file authorizations. It also provides mechanisms for encrypting and decrypting data, as well as verifying keys and identities.

## Main Modules and Components

### broker
The `broker` module is the core of the `keybroker` crate. It includes the `KeyBroker` struct, which provides methods for issuing and verifying keys, managing identities, and encrypting/decrypting data.

### encryptor
The `encryptor` module provides functionality for encrypting and decrypting data using the `Encryptor` struct.

### metrics
The `metrics` module provides functionality for logging metrics related to key management.

### secret
The `secret` module provides functionality for managing secrets, including the `Secret` struct.

### testing
The `testing` module provides utilities for testing the `keybroker` crate.

## Dependencies and Features

The `keybroker` crate depends on several other crates within the Convex backend, including `common`, `errors`, `pb`, and `sync_types`. It also depends on external crates such as `anyhow`, `prost`, and `sodiumoxide`.

The `keybroker` crate includes a `testing` feature, which enables additional functionality for testing purposes.
