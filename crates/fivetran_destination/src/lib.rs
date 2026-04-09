#![feature(impl_trait_in_assoc_type)]
#![feature(coroutines)]
#![feature(iterator_try_collect)]
#![feature(try_blocks)]
#![feature(try_blocks_heterogeneous)]
mod aes;
pub mod api_types;
mod application;
pub mod connector;
pub mod constants;
mod convert;
mod convex_api;
mod error;
mod file_reader;
mod schema;
use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "kebab-case")]
struct LogLine<'a> {
    level: &'a str,
    message: &'a str,
    message_origin: &'a str,
}
pub fn log(message: &str) {
    let result = serde_json::to_string(&LogLine {
        level: "INFO",
        message,
        message_origin: "sdk_destination",
    });
    match result {
        Ok(msg) => println!("{msg}"),
        Err(e) => println!("Unable to serialize to json: {message}: {e}"),
    }
}
