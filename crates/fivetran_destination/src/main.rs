#![feature(impl_trait_in_assoc_type)]
#![feature(coroutines)]
#![feature(iterator_try_collect)]
#![feature(lazy_cell)]
#![feature(try_blocks)]

mod aes;
mod api_types;
mod constants;
mod convert;
mod error;
mod file_reader;
mod fivetran_sdk;
mod schema;

#[cfg(test)]
mod tests;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    todo!()
}
