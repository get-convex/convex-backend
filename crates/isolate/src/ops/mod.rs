#![allow(non_snake_case)]
//! This module contains the implementation of both synchronous and
//! async ops. Unlike syscalls, these functions are present in *every*
//! environment, but the environment may decide not to implement their
//! functionality, causing a runtime error.

mod blob;
mod console;
mod crypto;
mod database;
mod environment_variables;
mod errors;
mod http;
mod random;
mod storage;
mod stream;
mod text;
mod time;
mod validate_args;

pub use self::crypto::CryptoOps;
