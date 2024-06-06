#![feature(impl_trait_in_assoc_type)]
#![feature(coroutines)]
#![feature(iterator_try_collect)]
#![feature(lazy_cell)]
#![feature(try_blocks)]

mod aes;
mod api_types;
mod constants;
mod convert;
mod convex_api;
mod error;
mod file_reader;
mod schema;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    todo!()
}
